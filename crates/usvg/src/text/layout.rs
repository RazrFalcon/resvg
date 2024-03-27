use crate::text::{
    chunk_span_at, script_supports_letter_spacing, shape_text, span_contains, ByteIndex,
    DatabaseExt, FontsCache, Glyph, GlyphClusters, OutlinedCluster, ResolvedFont,
};
use crate::tree::{BBox, IsValidLength};
use crate::{
    ApproxZeroUlps, Fill, Font, FontStretch, FontStyle, Group, Node, PaintOrder, Path,
    ShapeRendering, Stroke, Text, TextChunk, TextFlow, TextRendering, Visibility, WritingMode,
};
use rustybuzz::ttf_parser::GlyphId;
use std::collections::HashMap;
use std::sync::Arc;
use strict_num::NonZeroPositiveF32;
use svgtypes::FontFamily;
use tiny_skia_path::{NonZeroRect, Transform};
use unicode_script::UnicodeScript;

#[derive(Clone, Debug)]
struct GlyphCluster {
    byte_idx: ByteIndex,
    codepoint: char,
    width: f32,
    advance: f32,
    ascent: f32,
    descent: f32,
    x_height: f32,
    has_relative_shift: bool,
    glyphs: Vec<PositionedGlyph>,
    transform: Transform,
    visible: bool,
}

impl GlyphCluster {
    fn height(&self) -> f32 {
        self.ascent - self.descent
    }
}

#[derive(Clone, Debug)]
pub struct PositionedGlyph {
    pub(crate) transform: Transform,
    pub(crate) glyph_id: GlyphId,
    byte_idx: ByteIndex,
}

#[derive(Clone, Debug)]
pub struct PositionedSpan {
    pub(crate) fill: Option<Fill>,
    pub(crate) stroke: Option<Stroke>,
    pub(crate) paint_order: PaintOrder,
    pub(crate) font: Font,
    pub(crate) font_size: NonZeroPositiveF32,
    pub(crate) visibility: Visibility,
    pub(crate) transform: Transform,
    pub(crate) glyph_clusters: Vec<GlyphCluster>,
}

#[derive(Clone, Debug)]
pub enum PositionedTextFragment {
    Span(PositionedSpan),
    Path(Path),
}

pub(crate) fn convert(text: &mut Text, fontdb: &fontdb::Database) -> Option<()> {
    let (text_fragments, bbox, stroke_bbox) = layout_text(text, fontdb)?;

    text.bounding_box = bbox.to_rect();
    text.abs_bounding_box = bbox.transform(text.abs_transform)?.to_rect();
    // TODO: test
    // TODO: should we stroke transformed paths?
    text.stroke_bounding_box = stroke_bbox.to_rect();
    text.abs_stroke_bounding_box = stroke_bbox.transform(text.abs_transform)?.to_rect();
    text.layouted = text_fragments;

    Some(())
}

fn layout_text(
    text_node: &Text,
    fontdb: &fontdb::Database,
) -> Option<(Vec<PositionedTextFragment>, NonZeroRect, NonZeroRect)> {
    let mut fonts_cache: FontsCache = HashMap::new();

    for chunk in &text_node.chunks {
        for span in &chunk.spans {
            if !fonts_cache.contains_key(&span.font) {
                if let Some(font) = resolve_font(&span.font, fontdb) {
                    fonts_cache.insert(span.font.clone(), Arc::new(font));
                }
            }
        }
    }

    let mut text_fragments = vec![];
    let mut bbox = BBox::default();
    let mut stroke_bbox = BBox::default();
    let mut char_offset = 0;
    let mut last_x = 0.0;
    let mut last_y = 0.0;
    for chunk in &text_node.chunks {
        let (x, y) = match chunk.text_flow {
            TextFlow::Linear => (chunk.x.unwrap_or(last_x), chunk.y.unwrap_or(last_y)),
            TextFlow::Path(_) => (0.0, 0.0),
        };

        let mut clusters = process_chunk(chunk, &fonts_cache, fontdb);
        if clusters.is_empty() {
            char_offset += chunk.text.chars().count();
            continue;
        }

        apply_writing_mode(text_node.writing_mode, &mut clusters);
        apply_letter_spacing(chunk, &mut clusters);
        //     apply_word_spacing(chunk, &mut clusters);
        //     apply_length_adjust(chunk, &mut clusters);
        //     let mut curr_pos = resolve_clusters_positions(
        //         text_node,
        //         chunk,
        //         char_offset,
        //         text_node.writing_mode,
        //         &fonts_cache,
        //         &mut clusters,
        //     );
        //
        //     let mut text_ts = Transform::default();
        //     if text_node.writing_mode == WritingMode::TopToBottom {
        //         if let TextFlow::Linear = chunk.text_flow {
        //             text_ts = text_ts.pre_rotate_at(90.0, x, y);
        //         }
        //     }
        //
        //     for span in &chunk.spans {
        //         let font = match fonts_cache.get(&span.font) {
        //             Some(v) => v,
        //             None => continue,
        //         };
        //
        //         let decoration_spans = collect_decoration_spans(span, &clusters);
        //
        //         let mut span_ts = text_ts;
        //         span_ts = span_ts.pre_translate(x, y);
        //         if let TextFlow::Linear = chunk.text_flow {
        //             let shift = resolve_baseline(span, font, text_node.writing_mode);
        //
        //             // In case of a horizontal flow, shift transform and not clusters,
        //             // because clusters can be rotated and an additional shift will lead
        //             // to invalid results.
        //             span_ts = span_ts.pre_translate(0.0, shift);
        //         }
        //
        //         if let Some(decoration) = span.decoration.underline.clone() {
        //             // TODO: No idea what offset should be used for top-to-bottom layout.
        //             // There is
        //             // https://www.w3.org/TR/css-text-decor-3/#text-underline-position-property
        //             // but it doesn't go into details.
        //             let offset = match text_node.writing_mode {
        //                 WritingMode::LeftToRight => -font.underline_position(span.font_size.get()),
        //                 WritingMode::TopToBottom => font.height(span.font_size.get()) / 2.0,
        //             };
        //
        //             if let Some(path) =
        //                 convert_decoration(offset, span, font, decoration, &decoration_spans, span_ts)
        //             {
        //                 bbox = bbox.expand(path.data.bounds());
        //                 stroke_bbox = stroke_bbox.expand(path.data.bounds());
        //                 new_paths.push(path);
        //             }
        //         }
        //
        //         if let Some(decoration) = span.decoration.overline.clone() {
        //             let offset = match text_node.writing_mode {
        //                 WritingMode::LeftToRight => -font.ascent(span.font_size.get()),
        //                 WritingMode::TopToBottom => -font.height(span.font_size.get()) / 2.0,
        //             };
        //
        //             if let Some(path) =
        //                 convert_decoration(offset, span, font, decoration, &decoration_spans, span_ts)
        //             {
        //                 bbox = bbox.expand(path.data.bounds());
        //                 stroke_bbox = stroke_bbox.expand(path.data.bounds());
        //                 new_paths.push(path);
        //             }
        //         }
        //
        //         if let Some((path, span_bbox)) = convert_span(span, &mut clusters, span_ts) {
        //             bbox = bbox.expand(span_bbox);
        //             stroke_bbox = stroke_bbox.expand(path.stroke_bounding_box());
        //             new_paths.push(path);
        //         }
        //
        //         if let Some(decoration) = span.decoration.line_through.clone() {
        //             let offset = match text_node.writing_mode {
        //                 WritingMode::LeftToRight => -font.line_through_position(span.font_size.get()),
        //                 WritingMode::TopToBottom => 0.0,
        //             };
        //
        //             if let Some(path) =
        //                 convert_decoration(offset, span, font, decoration, &decoration_spans, span_ts)
        //             {
        //                 bbox = bbox.expand(path.data.bounds());
        //                 stroke_bbox = stroke_bbox.expand(path.data.bounds());
        //                 new_paths.push(path);
        //             }
        //         }
        //     }
        //
        //     char_offset += chunk.text.chars().count();
        //
        //     if text_node.writing_mode == WritingMode::TopToBottom {
        //         if let TextFlow::Linear = chunk.text_flow {
        //             std::mem::swap(&mut curr_pos.0, &mut curr_pos.1);
        //         }
        //     }
        //
        //     last_x = x + curr_pos.0;
        //     last_y = y + curr_pos.1;
    }

    let bbox = bbox.to_non_zero_rect()?;
    let stroke_bbox = stroke_bbox.to_non_zero_rect().unwrap_or(bbox);
    Some((text_fragments, bbox, stroke_bbox))
}

/// Converts a text chunk into a list of outlined clusters.
///
/// This function will do the BIDI reordering, text shaping and glyphs outlining,
/// but not the text layouting. So all clusters are in the 0x0 position.
fn process_chunk(
    chunk: &TextChunk,
    fonts_cache: &FontsCache,
    fontdb: &fontdb::Database,
) -> Vec<GlyphCluster> {
    let mut glyphs = Vec::new();

    for span in &chunk.spans {
        let font = match fonts_cache.get(&span.font) {
            Some(v) => v.clone(),
            None => continue,
        };

        let tmp_glyphs = shape_text(
            &chunk.text,
            font,
            span.small_caps,
            span.apply_kerning,
            fontdb,
        );

        // Do nothing with the first run.
        if glyphs.is_empty() {
            glyphs = tmp_glyphs;
            continue;
        }

        // We assume, that shaping with an any font will produce the same amount of glyphs.
        // Otherwise an error.
        if glyphs.len() != tmp_glyphs.len() {
            log::warn!("Text layouting failed.");
            return Vec::new();
        }

        // Copy span's glyphs.
        for (i, glyph) in tmp_glyphs.iter().enumerate() {
            if span_contains(span, glyph.byte_idx) {
                glyphs[i] = glyph.clone();
            }
        }
    }

    // Convert glyphs to clusters.
    let mut clusters = Vec::new();
    for (range, byte_idx) in GlyphClusters::new(&glyphs) {
        if let Some(span) = chunk_span_at(chunk, byte_idx) {
            clusters.push(form_glyph_clusters(
                &glyphs[range],
                &chunk.text,
                span.font_size.get(),
            ));
        }
    }

    clusters
}

/// Rotates clusters according to
/// [Unicode Vertical_Orientation Property](https://www.unicode.org/reports/tr50/tr50-19.html).
fn apply_writing_mode(writing_mode: WritingMode, clusters: &mut [GlyphCluster]) {
    if writing_mode != WritingMode::TopToBottom {
        return;
    }

    for cluster in clusters {
        let orientation = unicode_vo::char_orientation(cluster.codepoint);
        if orientation == unicode_vo::Orientation::Upright {
            // Additional offset. Not sure why.
            let dy = cluster.width - cluster.height();

            // Rotate a cluster 90deg counter clockwise by the center.
            let mut ts = Transform::default();
            ts = ts.pre_translate(cluster.width / 2.0, 0.0);
            ts = ts.pre_rotate(-90.0);
            ts = ts.pre_translate(-cluster.width / 2.0, -dy);

            for glyph in &mut cluster.glyphs {
                glyph.transform = glyph.transform.pre_concat(ts);
            }

            // Move "baseline" to the middle and make height equal to width.
            cluster.ascent = cluster.width / 2.0;
            cluster.descent = -cluster.width / 2.0;
        } else {
            // Could not find a spec that explains this,
            // but this is how other applications are shifting the "rotated" characters
            // in the top-to-bottom mode.
            cluster.transform = cluster.transform.pre_translate(0.0, cluster.x_height / 2.0);
        }
    }
}

/// Applies the `letter-spacing` property to a text chunk clusters.
///
/// [In the CSS spec](https://www.w3.org/TR/css-text-3/#letter-spacing-property).
fn apply_letter_spacing(chunk: &TextChunk, clusters: &mut [GlyphCluster]) {
    // At least one span should have a non-zero spacing.
    if !chunk
        .spans
        .iter()
        .any(|span| !span.letter_spacing.approx_zero_ulps(4))
    {
        return;
    }

    let num_clusters = clusters.len();
    for (i, cluster) in clusters.iter_mut().enumerate() {
        // Spacing must be applied only to characters that belongs to the script
        // that supports spacing.
        // We are checking only the first code point, since it should be enough.
        // https://www.w3.org/TR/css-text-3/#cursive-tracking
        let script = cluster.codepoint.script();
        if script_supports_letter_spacing(script) {
            if let Some(span) = chunk_span_at(chunk, cluster.byte_idx) {
                // A space after the last cluster should be ignored,
                // since it affects the bbox and text alignment.
                if i != num_clusters - 1 {
                    cluster.advance += span.letter_spacing;
                }

                // If the cluster advance became negative - clear it.
                // This is an UB so we can do whatever we want, and we mimic Chrome's behavior.
                if !cluster.advance.is_valid_length() {
                    cluster.width = 0.0;
                    cluster.advance = 0.0;
                    cluster.glyphs = vec![];
                }
            }
        }
    }
}

fn form_glyph_clusters(glyphs: &[Glyph], text: &str, font_size: f32) -> GlyphCluster {
    debug_assert!(!glyphs.is_empty());

    let mut width = 0.0;
    let mut x: f32 = 0.0;

    let mut positioned_glyphs = vec![];

    for glyph in glyphs {
        let sx = glyph.font.scale(font_size);

        let mut ts = Transform::from_scale(sx, sx);

        // Apply offset.
        //
        // The first glyph in the cluster will have an offset from 0x0,
        // but the later one will have an offset from the "current position".
        // So we have to keep an advance.
        // TODO: should be done only inside a single text span
        ts = ts.pre_translate(x + glyph.dx as f32, glyph.dy as f32);

        positioned_glyphs.push(PositionedGlyph {
            transform: ts,
            glyph_id: glyph.id,
            byte_idx: glyph.byte_idx,
        });

        x += glyph.width as f32;

        let glyph_width = glyph.width as f32 * sx;
        if glyph_width > width {
            width = glyph_width;
        }
    }

    let byte_idx = glyphs[0].byte_idx;
    let font = glyphs[0].font.clone();
    GlyphCluster {
        byte_idx,
        codepoint: byte_idx.char_from(text),
        width,
        advance: width,
        ascent: font.ascent(font_size),
        descent: font.descent(font_size),
        x_height: font.x_height(font_size),
        has_relative_shift: false,
        transform: Transform::default(),
        glyphs: positioned_glyphs,
        visible: true,
    }
}

fn resolve_font(font: &Font, fontdb: &fontdb::Database) -> Option<ResolvedFont> {
    let mut name_list = Vec::new();
    for family in &font.families {
        name_list.push(match family {
            FontFamily::Serif => fontdb::Family::Serif,
            FontFamily::SansSerif => fontdb::Family::SansSerif,
            FontFamily::Cursive => fontdb::Family::Cursive,
            FontFamily::Fantasy => fontdb::Family::Fantasy,
            FontFamily::Monospace => fontdb::Family::Monospace,
            FontFamily::Named(s) => fontdb::Family::Name(s),
        });
    }

    // Use the default font as fallback.
    name_list.push(fontdb::Family::Serif);

    let stretch = match font.stretch {
        FontStretch::UltraCondensed => fontdb::Stretch::UltraCondensed,
        FontStretch::ExtraCondensed => fontdb::Stretch::ExtraCondensed,
        FontStretch::Condensed => fontdb::Stretch::Condensed,
        FontStretch::SemiCondensed => fontdb::Stretch::SemiCondensed,
        FontStretch::Normal => fontdb::Stretch::Normal,
        FontStretch::SemiExpanded => fontdb::Stretch::SemiExpanded,
        FontStretch::Expanded => fontdb::Stretch::Expanded,
        FontStretch::ExtraExpanded => fontdb::Stretch::ExtraExpanded,
        FontStretch::UltraExpanded => fontdb::Stretch::UltraExpanded,
    };

    let style = match font.style {
        FontStyle::Normal => fontdb::Style::Normal,
        FontStyle::Italic => fontdb::Style::Italic,
        FontStyle::Oblique => fontdb::Style::Oblique,
    };

    let query = fontdb::Query {
        families: &name_list,
        weight: fontdb::Weight(font.weight),
        stretch,
        style,
    };

    let id = fontdb.query(&query);
    if id.is_none() {
        log::warn!(
            "No match for '{}' font-family.",
            font.families
                .iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    fontdb.load_font(id?)
}

fn resolve_rendering_mode(text: &Text) -> ShapeRendering {
    match text.rendering_mode {
        TextRendering::OptimizeSpeed => ShapeRendering::CrispEdges,
        TextRendering::OptimizeLegibility => ShapeRendering::GeometricPrecision,
        TextRendering::GeometricPrecision => ShapeRendering::GeometricPrecision,
    }
}
