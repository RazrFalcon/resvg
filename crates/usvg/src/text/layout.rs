use crate::text::{DatabaseExt, FontsCache, ResolvedFont};
use crate::tree::BBox;
use crate::{
    Fill, Font, FontStretch, FontStyle, Group, Node, PaintOrder, Path, ShapeRendering, Stroke,
    Text, TextRendering, Visibility,
};
use std::collections::HashMap;
use std::sync::Arc;
use strict_num::NonZeroPositiveF32;
use svgtypes::FontFamily;
use tiny_skia_path::{NonZeroRect, Transform};

#[derive(Clone, Debug)]
pub struct PositionedGlyph {
    pub(crate) transform: Transform,
    pub(crate) glyph_id: u16,
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
    pub(crate) glyphs: Vec<PositionedGlyph>,
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
    // let mut char_offset = 0;
    // let mut last_x = 0.0;
    // let mut last_y = 0.0;
    // let mut new_paths = Vec::new();
    // for chunk in &text_node.chunks {
    //     let (x, y) = match chunk.text_flow {
    //         TextFlow::Linear => (chunk.x.unwrap_or(last_x), chunk.y.unwrap_or(last_y)),
    //         TextFlow::Path(_) => (0.0, 0.0),
    //     };
    //
    //     let mut clusters = outline_chunk(chunk, &fonts_cache, fontdb);
    //     if clusters.is_empty() {
    //         char_offset += chunk.text.chars().count();
    //         continue;
    //     }
    //
    //     apply_writing_mode(text_node.writing_mode, &mut clusters);
    //     apply_letter_spacing(chunk, &mut clusters);
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
    // }
    //
    let bbox = bbox.to_non_zero_rect()?;
    let stroke_bbox = stroke_bbox.to_non_zero_rect().unwrap_or(bbox);
    Some((text_fragments, bbox, stroke_bbox))
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
