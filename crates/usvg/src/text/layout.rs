// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::tree::{BBox, IsValidLength};
use crate::{
    AlignmentBaseline, ApproxZeroUlps, BaselineShift, DominantBaseline, Fill, FillRule, Font,
    FontStretch, FontStyle, LengthAdjust, PaintOrder, Path, ShapeRendering, Stroke, Text,
    TextAnchor, TextChunk, TextDecorationStyle, TextFlow, TextPath, TextSpan, Visibility,
    WritingMode,
};
use fontdb::{Database, ID};
use kurbo::{ParamCurve, ParamCurveArclen, ParamCurveDeriv};
use rustybuzz::ttf_parser;
use rustybuzz::ttf_parser::GlyphId;
use std::collections::HashMap;
use std::num::NonZeroU16;
use std::sync::Arc;
use strict_num::NonZeroPositiveF32;
use svgtypes::FontFamily;
use tiny_skia_path::{NonZeroRect, Transform};
use unicode_script::UnicodeScript;

/// A glyph that has already been positioned correctly.
///
/// Note that the transform already takes the font size into consideration, so applying the
/// transform to the outline of the glyphs is all that is necessary to display it correctly.
#[derive(Clone, Copy, Debug)]
pub struct PositionedGlyph {
    /// The transform of the glyph. This transform should be applied to the _glyph outlines_, meaning
    /// that paint servers referenced by the glyph's span should not be affected by it.
    pub transform: Transform,
    /// The ID of the glyph.
    pub glyph_id: GlyphId,
    /// The ID of the font the glyph should be taken from.
    pub font: ID,
}

/// A span contains a number of layouted glyphs that share the same fill, stroke, paint order and
/// visibility.
#[derive(Clone, Debug)]
pub struct Span {
    /// The fill of the span.
    pub fill: Option<Fill>,
    /// The stroke of the span.
    pub stroke: Option<Stroke>,
    /// The paint order of the span.
    pub paint_order: PaintOrder,
    /// The font size of the span.
    pub font_size: NonZeroPositiveF32,
    /// The visibility of the span.
    pub visibility: Visibility,
    /// The glyphs that make up the span.
    pub positioned_glyphs: Vec<PositionedGlyph>,
}

/// A text fragment is a component of a text element.
///
/// It can either be a span (which contains a number of layouted glyphs that share
/// some properties) or a path (which is a layouted decoration span).
#[derive(Clone, Debug)]
pub enum TextFragment {
    /// A text span.
    Span(Span),
    /// A path.
    Path(Path),
}

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
    path_transform: Transform,
    visible: bool,
}

impl GlyphCluster {
    pub(crate) fn height(&self) -> f32 {
        self.ascent - self.descent
    }

    pub(crate) fn transform(&self) -> Transform {
        self.path_transform.post_concat(self.transform)
    }
}

pub(crate) fn layout_text(
    text_node: &Text,
    fontdb: &fontdb::Database,
) -> Option<(Vec<TextFragment>, NonZeroRect)> {
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
    let mut char_offset = 0;
    let mut last_x = 0.0;
    let mut last_y = 0.0;
    let mut bbox = BBox::default();
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
        apply_word_spacing(chunk, &mut clusters);

        apply_length_adjust(chunk, &mut clusters);
        let mut curr_pos = resolve_clusters_positions(
            text_node,
            chunk,
            char_offset,
            text_node.writing_mode,
            &fonts_cache,
            &mut clusters,
        );

        let mut text_ts = Transform::default();
        if text_node.writing_mode == WritingMode::TopToBottom {
            if let TextFlow::Linear = chunk.text_flow {
                text_ts = text_ts.pre_rotate_at(90.0, x, y);
            }
        }

        for span in &chunk.spans {
            let font = match fonts_cache.get(&span.font) {
                Some(v) => v,
                None => continue,
            };

            let decoration_spans = collect_decoration_spans(span, &clusters);

            let mut span_ts = text_ts;
            span_ts = span_ts.pre_translate(x, y);
            if let TextFlow::Linear = chunk.text_flow {
                let shift = resolve_baseline(span, font, text_node.writing_mode);

                // In case of a horizontal flow, shift transform and not clusters,
                // because clusters can be rotated and an additional shift will lead
                // to invalid results.
                span_ts = span_ts.pre_translate(0.0, shift);
            }

            if let Some(decoration) = span.decoration.underline.clone() {
                // TODO: No idea what offset should be used for top-to-bottom layout.
                // There is
                // https://www.w3.org/TR/css-text-decor-3/#text-underline-position-property
                // but it doesn't go into details.
                let offset = match text_node.writing_mode {
                    WritingMode::LeftToRight => -font.underline_position(span.font_size.get()),
                    WritingMode::TopToBottom => font.height(span.font_size.get()) / 2.0,
                };

                if let Some(path) =
                    convert_decoration(offset, span, font, decoration, &decoration_spans, span_ts)
                {
                    bbox = bbox.expand(path.data.bounds());
                    text_fragments.push(TextFragment::Path(path));
                }
            }

            if let Some(decoration) = span.decoration.overline.clone() {
                let offset = match text_node.writing_mode {
                    WritingMode::LeftToRight => -font.ascent(span.font_size.get()),
                    WritingMode::TopToBottom => -font.height(span.font_size.get()) / 2.0,
                };

                if let Some(path) =
                    convert_decoration(offset, span, font, decoration, &decoration_spans, span_ts)
                {
                    bbox = bbox.expand(path.data.bounds());
                    text_fragments.push(TextFragment::Path(path));
                }
            }

            let mut fill = span.fill.clone();
            if let Some(ref mut fill) = fill {
                // The `fill-rule` should be ignored.
                // https://www.w3.org/TR/SVG2/text.html#TextRenderingOrder
                //
                // 'Since the fill-rule property does not apply to SVG text elements,
                // the specific order of the subpaths within the equivalent path does not matter.'
                fill.rule = FillRule::NonZero;
            }

            if let Some((span_fragments, span_bbox)) = convert_span(span, &clusters, span_ts) {
                bbox = bbox.expand(span_bbox);

                let positioned_glyphs = span_fragments
                    .into_iter()
                    .flat_map(|mut gc| {
                        let cluster_ts = gc.transform();
                        gc.glyphs.iter_mut().for_each(|pg| {
                            pg.transform = pg.transform.post_concat(cluster_ts).post_concat(span_ts)
                        });
                        gc.glyphs
                    })
                    .collect();

                text_fragments.push(TextFragment::Span(Span {
                    fill,
                    stroke: span.stroke.clone(),
                    paint_order: span.paint_order,
                    font_size: span.font_size,
                    visibility: span.visibility,
                    positioned_glyphs,
                }));
            }

            if let Some(decoration) = span.decoration.line_through.clone() {
                let offset = match text_node.writing_mode {
                    WritingMode::LeftToRight => -font.line_through_position(span.font_size.get()),
                    WritingMode::TopToBottom => 0.0,
                };

                if let Some(path) =
                    convert_decoration(offset, span, font, decoration, &decoration_spans, span_ts)
                {
                    bbox = bbox.expand(path.data.bounds());
                    text_fragments.push(TextFragment::Path(path));
                }
            }
        }

        char_offset += chunk.text.chars().count();

        if text_node.writing_mode == WritingMode::TopToBottom {
            if let TextFlow::Linear = chunk.text_flow {
                std::mem::swap(&mut curr_pos.0, &mut curr_pos.1);
            }
        }

        last_x = x + curr_pos.0;
        last_y = y + curr_pos.1;
    }

    let bbox = bbox.to_non_zero_rect()?;

    Some((text_fragments, bbox))
}

fn convert_span(
    span: &TextSpan,
    clusters: &[GlyphCluster],
    text_ts: Transform,
) -> Option<(Vec<GlyphCluster>, NonZeroRect)> {
    let mut span_clusters = vec![];
    let mut bboxes_builder = tiny_skia_path::PathBuilder::new();

    for cluster in clusters {
        if !cluster.visible {
            continue;
        }

        if span_contains(span, cluster.byte_idx) {
            span_clusters.push(cluster.clone());
        }

        let mut advance = cluster.advance;
        if advance <= 0.0 {
            advance = 1.0;
        }

        // We have to calculate text bbox using font metrics and not glyph shape.
        if let Some(r) = NonZeroRect::from_xywh(0.0, -cluster.ascent, advance, cluster.height()) {
            if let Some(r) = r.transform(cluster.transform()) {
                bboxes_builder.push_rect(r.to_rect());
            }
        }
    }

    let mut bboxes = bboxes_builder.finish()?;
    bboxes = bboxes.transform(text_ts)?;
    let bbox = bboxes.compute_tight_bounds()?.to_non_zero_rect()?;

    Some((span_clusters, bbox))
}

fn collect_decoration_spans(span: &TextSpan, clusters: &[GlyphCluster]) -> Vec<DecorationSpan> {
    let mut spans = Vec::new();

    let mut started = false;
    let mut width = 0.0;
    let mut transform = Transform::default();

    for cluster in clusters {
        if span_contains(span, cluster.byte_idx) {
            if started && cluster.has_relative_shift {
                started = false;
                spans.push(DecorationSpan { width, transform });
            }

            if !started {
                width = cluster.advance;
                started = true;
                transform = cluster.transform;
            } else {
                width += cluster.advance;
            }
        } else if started {
            spans.push(DecorationSpan { width, transform });
            started = false;
        }
    }

    if started {
        spans.push(DecorationSpan { width, transform });
    }

    spans
}

pub(crate) fn convert_decoration(
    dy: f32,
    span: &TextSpan,
    font: &ResolvedFont,
    mut decoration: TextDecorationStyle,
    decoration_spans: &[DecorationSpan],
    transform: Transform,
) -> Option<Path> {
    debug_assert!(!decoration_spans.is_empty());

    let thickness = font.underline_thickness(span.font_size.get());

    let mut builder = tiny_skia_path::PathBuilder::new();
    for dec_span in decoration_spans {
        let rect = match NonZeroRect::from_xywh(0.0, -thickness / 2.0, dec_span.width, thickness) {
            Some(v) => v,
            None => {
                log::warn!("a decoration span has a malformed bbox");
                continue;
            }
        };

        let ts = dec_span.transform.pre_translate(0.0, dy);

        let mut path = tiny_skia_path::PathBuilder::from_rect(rect.to_rect());
        path = match path.transform(ts) {
            Some(v) => v,
            None => continue,
        };

        builder.push_path(&path);
    }

    let mut path_data = builder.finish()?;
    path_data = path_data.transform(transform)?;

    Path::new(
        String::new(),
        span.visibility,
        decoration.fill.take(),
        decoration.stroke.take(),
        PaintOrder::default(),
        ShapeRendering::default(),
        Arc::new(path_data),
        Transform::default(),
    )
}

/// A text decoration span.
///
/// Basically a horizontal line, that will be used for underline, overline and line-through.
/// It doesn't have a height, since it depends on the Font metrics.
#[derive(Clone, Copy)]
pub(crate) struct DecorationSpan {
    pub(crate) width: f32,
    pub(crate) transform: Transform,
}

/// Resolves clusters positions.
///
/// Mainly sets the `transform` property.
///
/// Returns the last text position. The next text chunk should start from that position.
fn resolve_clusters_positions(
    text: &Text,
    chunk: &TextChunk,
    char_offset: usize,
    writing_mode: WritingMode,
    fonts_cache: &FontsCache,
    clusters: &mut [GlyphCluster],
) -> (f32, f32) {
    match chunk.text_flow {
        TextFlow::Linear => {
            resolve_clusters_positions_horizontal(text, chunk, char_offset, writing_mode, clusters)
        }
        TextFlow::Path(ref path) => resolve_clusters_positions_path(
            text,
            chunk,
            char_offset,
            path,
            writing_mode,
            fonts_cache,
            clusters,
        ),
    }
}

fn clusters_length(clusters: &[GlyphCluster]) -> f32 {
    clusters.iter().fold(0.0, |w, cluster| w + cluster.advance)
}

fn resolve_clusters_positions_horizontal(
    text: &Text,
    chunk: &TextChunk,
    offset: usize,
    writing_mode: WritingMode,
    clusters: &mut [GlyphCluster],
) -> (f32, f32) {
    let mut x = process_anchor(chunk.anchor, clusters_length(clusters));
    let mut y = 0.0;

    for cluster in clusters {
        let cp = offset + cluster.byte_idx.code_point_at(&chunk.text);
        if let (Some(dx), Some(dy)) = (text.dx.get(cp), text.dy.get(cp)) {
            if writing_mode == WritingMode::LeftToRight {
                x += dx;
                y += dy;
            } else {
                y -= dx;
                x += dy;
            }
            cluster.has_relative_shift = !dx.approx_zero_ulps(4) || !dy.approx_zero_ulps(4);
        }

        cluster.transform = cluster.transform.pre_translate(x, y);

        if let Some(angle) = text.rotate.get(cp).cloned() {
            if !angle.approx_zero_ulps(4) {
                cluster.transform = cluster.transform.pre_rotate(angle);
                cluster.has_relative_shift = true;
            }
        }

        x += cluster.advance;
    }

    (x, y)
}

// Baseline resolving in SVG is a mess.
// Not only it's poorly documented, but as soon as you start mixing
// `dominant-baseline` and `alignment-baseline` each application/browser will produce
// different results.
//
// For now, resvg simply tries to match Chrome's output and not the mythical SVG spec output.
//
// See `alignment_baseline_shift` method comment for more details.
pub(crate) fn resolve_baseline(
    span: &TextSpan,
    font: &ResolvedFont,
    writing_mode: WritingMode,
) -> f32 {
    let mut shift = -resolve_baseline_shift(&span.baseline_shift, font, span.font_size.get());

    // TODO: support vertical layout as well
    if writing_mode == WritingMode::LeftToRight {
        if span.alignment_baseline == AlignmentBaseline::Auto
            || span.alignment_baseline == AlignmentBaseline::Baseline
        {
            shift += font.dominant_baseline_shift(span.dominant_baseline, span.font_size.get());
        } else {
            shift += font.alignment_baseline_shift(span.alignment_baseline, span.font_size.get());
        }
    }

    shift
}

fn resolve_baseline_shift(baselines: &[BaselineShift], font: &ResolvedFont, font_size: f32) -> f32 {
    let mut shift = 0.0;
    for baseline in baselines.iter().rev() {
        match baseline {
            BaselineShift::Baseline => {}
            BaselineShift::Subscript => shift -= font.subscript_offset(font_size),
            BaselineShift::Superscript => shift += font.superscript_offset(font_size),
            BaselineShift::Number(n) => shift += n,
        }
    }

    shift
}

fn resolve_clusters_positions_path(
    text: &Text,
    chunk: &TextChunk,
    char_offset: usize,
    path: &TextPath,
    writing_mode: WritingMode,
    fonts_cache: &FontsCache,
    clusters: &mut [GlyphCluster],
) -> (f32, f32) {
    let mut last_x = 0.0;
    let mut last_y = 0.0;

    let mut dy = 0.0;

    // In the text path mode, chunk's x/y coordinates provide an additional offset along the path.
    // The X coordinate is used in a horizontal mode, and Y in vertical.
    let chunk_offset = match writing_mode {
        WritingMode::LeftToRight => chunk.x.unwrap_or(0.0),
        WritingMode::TopToBottom => chunk.y.unwrap_or(0.0),
    };

    let start_offset =
        chunk_offset + path.start_offset + process_anchor(chunk.anchor, clusters_length(clusters));

    let normals = collect_normals(text, chunk, clusters, &path.path, char_offset, start_offset);
    for (cluster, normal) in clusters.iter_mut().zip(normals) {
        let (x, y, angle) = match normal {
            Some(normal) => (normal.x, normal.y, normal.angle),
            None => {
                // Hide clusters that are outside the text path.
                cluster.visible = false;
                continue;
            }
        };

        // We have to break a decoration line for each cluster during text-on-path.
        cluster.has_relative_shift = true;

        let orig_ts = cluster.transform;

        // Clusters should be rotated by the x-midpoint x baseline position.
        let half_width = cluster.width / 2.0;
        cluster.transform = Transform::default();
        cluster.transform = cluster.transform.pre_translate(x - half_width, y);
        cluster.transform = cluster.transform.pre_rotate_at(angle, half_width, 0.0);

        let cp = char_offset + cluster.byte_idx.code_point_at(&chunk.text);
        dy += text.dy.get(cp).cloned().unwrap_or(0.0);

        let baseline_shift = chunk_span_at(chunk, cluster.byte_idx)
            .map(|span| {
                let font = match fonts_cache.get(&span.font) {
                    Some(v) => v,
                    None => return 0.0,
                };
                -resolve_baseline(span, font, writing_mode)
            })
            .unwrap_or(0.0);

        // Shift only by `dy` since we already applied `dx`
        // during offset along the path calculation.
        if !dy.approx_zero_ulps(4) || !baseline_shift.approx_zero_ulps(4) {
            let shift = kurbo::Vec2::new(0.0, (dy - baseline_shift) as f64);
            cluster.transform = cluster
                .transform
                .pre_translate(shift.x as f32, shift.y as f32);
        }

        if let Some(angle) = text.rotate.get(cp).cloned() {
            if !angle.approx_zero_ulps(4) {
                cluster.transform = cluster.transform.pre_rotate(angle);
            }
        }

        // The possible `lengthAdjust` transform should be applied after text-on-path positioning.
        cluster.transform = cluster.transform.pre_concat(orig_ts);

        last_x = x + cluster.advance;
        last_y = y;
    }

    (last_x, last_y)
}

pub(crate) fn process_anchor(a: TextAnchor, text_width: f32) -> f32 {
    match a {
        TextAnchor::Start => 0.0, // Nothing.
        TextAnchor::Middle => -text_width / 2.0,
        TextAnchor::End => -text_width,
    }
}

pub(crate) struct PathNormal {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) angle: f32,
}

fn collect_normals(
    text: &Text,
    chunk: &TextChunk,
    clusters: &[GlyphCluster],
    path: &tiny_skia_path::Path,
    char_offset: usize,
    offset: f32,
) -> Vec<Option<PathNormal>> {
    let mut offsets = Vec::with_capacity(clusters.len());
    let mut normals = Vec::with_capacity(clusters.len());
    {
        let mut advance = offset;
        for cluster in clusters {
            // Clusters should be rotated by the x-midpoint x baseline position.
            let half_width = cluster.width / 2.0;

            // Include relative position.
            let cp = char_offset + cluster.byte_idx.code_point_at(&chunk.text);
            advance += text.dx.get(cp).cloned().unwrap_or(0.0);

            let offset = advance + half_width;

            // Clusters outside the path have no normals.
            if offset < 0.0 {
                normals.push(None);
            }

            offsets.push(offset as f64);
            advance += cluster.advance;
        }
    }

    let mut prev_mx = path.points()[0].x;
    let mut prev_my = path.points()[0].y;
    let mut prev_x = prev_mx;
    let mut prev_y = prev_my;

    fn create_curve_from_line(px: f32, py: f32, x: f32, y: f32) -> kurbo::CubicBez {
        let line = kurbo::Line::new(
            kurbo::Point::new(px as f64, py as f64),
            kurbo::Point::new(x as f64, y as f64),
        );
        let p1 = line.eval(0.33);
        let p2 = line.eval(0.66);
        kurbo::CubicBez {
            p0: line.p0,
            p1,
            p2,
            p3: line.p1,
        }
    }

    let mut length: f64 = 0.0;
    for seg in path.segments() {
        let curve = match seg {
            tiny_skia_path::PathSegment::MoveTo(p) => {
                prev_mx = p.x;
                prev_my = p.y;
                prev_x = p.x;
                prev_y = p.y;
                continue;
            }
            tiny_skia_path::PathSegment::LineTo(p) => {
                create_curve_from_line(prev_x, prev_y, p.x, p.y)
            }
            tiny_skia_path::PathSegment::QuadTo(p1, p) => kurbo::QuadBez {
                p0: kurbo::Point::new(prev_x as f64, prev_y as f64),
                p1: kurbo::Point::new(p1.x as f64, p1.y as f64),
                p2: kurbo::Point::new(p.x as f64, p.y as f64),
            }
            .raise(),
            tiny_skia_path::PathSegment::CubicTo(p1, p2, p) => kurbo::CubicBez {
                p0: kurbo::Point::new(prev_x as f64, prev_y as f64),
                p1: kurbo::Point::new(p1.x as f64, p1.y as f64),
                p2: kurbo::Point::new(p2.x as f64, p2.y as f64),
                p3: kurbo::Point::new(p.x as f64, p.y as f64),
            },
            tiny_skia_path::PathSegment::Close => {
                create_curve_from_line(prev_x, prev_y, prev_mx, prev_my)
            }
        };

        let arclen_accuracy = {
            let base_arclen_accuracy = 0.5;
            // Accuracy depends on a current scale.
            // When we have a tiny path scaled by a large value,
            // we have to increase out accuracy accordingly.
            let (sx, sy) = text.abs_transform.get_scale();
            // 1.0 acts as a threshold to prevent division by 0 and/or low accuracy.
            base_arclen_accuracy / (sx * sy).sqrt().max(1.0)
        };

        let curve_len = curve.arclen(arclen_accuracy as f64);

        for offset in &offsets[normals.len()..] {
            if *offset >= length && *offset <= length + curve_len {
                let mut offset = curve.inv_arclen(offset - length, arclen_accuracy as f64);
                // some rounding error may occur, so we give offset a little tolerance
                debug_assert!((-1.0e-3..=1.0 + 1.0e-3).contains(&offset));
                offset = offset.min(1.0).max(0.0);

                let pos = curve.eval(offset);
                let d = curve.deriv().eval(offset);
                let d = kurbo::Vec2::new(-d.y, d.x); // tangent
                let angle = d.atan2().to_degrees() - 90.0;

                normals.push(Some(PathNormal {
                    x: pos.x as f32,
                    y: pos.y as f32,
                    angle: angle as f32,
                }));

                if normals.len() == offsets.len() {
                    break;
                }
            }
        }

        length += curve_len;
        prev_x = curve.p3.x as f32;
        prev_y = curve.p3.y as f32;
    }

    // If path ended and we still have unresolved normals - set them to `None`.
    for _ in 0..(offsets.len() - normals.len()) {
        normals.push(None);
    }

    normals
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

fn apply_length_adjust(chunk: &TextChunk, clusters: &mut [GlyphCluster]) {
    let is_horizontal = matches!(chunk.text_flow, TextFlow::Linear);

    for span in &chunk.spans {
        let target_width = match span.text_length {
            Some(v) => v,
            None => continue,
        };

        let mut width = 0.0;
        let mut cluster_indexes = Vec::new();
        for i in span.start..span.end {
            if let Some(index) = clusters.iter().position(|c| c.byte_idx.value() == i) {
                cluster_indexes.push(index);
            }
        }
        // Complex scripts can have mutli-codepoint clusters therefore we have to remove duplicates.
        cluster_indexes.sort();
        cluster_indexes.dedup();

        for i in &cluster_indexes {
            // Use the original cluster `width` and not `advance`.
            // This method essentially discards any `word-spacing` and `letter-spacing`.
            width += clusters[*i].width;
        }

        if cluster_indexes.is_empty() {
            continue;
        }

        if span.length_adjust == LengthAdjust::Spacing {
            let factor = if cluster_indexes.len() > 1 {
                (target_width - width) / (cluster_indexes.len() - 1) as f32
            } else {
                0.0
            };

            for i in cluster_indexes {
                clusters[i].advance = clusters[i].width + factor;
            }
        } else {
            let factor = target_width / width;
            // Prevent multiplying by zero.
            if factor < 0.001 {
                continue;
            }

            for i in cluster_indexes {
                clusters[i].transform = clusters[i].transform.pre_scale(factor, 1.0);

                // Technically just a hack to support the current text-on-path algorithm.
                if !is_horizontal {
                    clusters[i].advance *= factor;
                    clusters[i].width *= factor;
                }
            }
        }
    }
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

            cluster.path_transform = ts;

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

/// Applies the `word-spacing` property to a text chunk clusters.
///
/// [In the CSS spec](https://www.w3.org/TR/css-text-3/#propdef-word-spacing).
fn apply_word_spacing(chunk: &TextChunk, clusters: &mut [GlyphCluster]) {
    // At least one span should have a non-zero spacing.
    if !chunk
        .spans
        .iter()
        .any(|span| !span.word_spacing.approx_zero_ulps(4))
    {
        return;
    }

    for cluster in clusters {
        if is_word_separator_characters(cluster.codepoint) {
            if let Some(span) = chunk_span_at(chunk, cluster.byte_idx) {
                // Technically, word spacing 'should be applied half on each
                // side of the character', but it doesn't affect us in any way,
                // so we are ignoring this.
                cluster.advance += span.word_spacing;

                // After word spacing, `advance` can be negative.
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

        // By default, glyphs are upside-down, so we have to mirror them.
        let mut ts = Transform::from_scale(1.0, -1.0);

        // Scale to font-size.
        ts = ts.pre_scale(sx, sx);

        // Apply offset.
        //
        // The first glyph in the cluster will have an offset from 0x0,
        // but the later one will have an offset from the "current position".
        // So we have to keep an advance.
        // TODO: should be done only inside a single text span
        ts = ts.pre_translate(x + glyph.dx as f32, glyph.dy as f32);

        positioned_glyphs.push(PositionedGlyph {
            transform: ts,
            font: glyph.font.id,
            glyph_id: glyph.id,
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
        path_transform: Transform::default(),
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

pub(crate) trait DatabaseExt {
    fn load_font(&self, id: ID) -> Option<ResolvedFont>;
    fn has_char(&self, id: ID, c: char) -> bool;
}

impl DatabaseExt for Database {
    #[inline(never)]
    fn load_font(&self, id: ID) -> Option<ResolvedFont> {
        self.with_face_data(id, |data, face_index| -> Option<ResolvedFont> {
            let font = ttf_parser::Face::parse(data, face_index).ok()?;

            let units_per_em = NonZeroU16::new(font.units_per_em())?;

            let ascent = font.ascender();
            let descent = font.descender();

            let x_height = font
                .x_height()
                .and_then(|x| u16::try_from(x).ok())
                .and_then(NonZeroU16::new);
            let x_height = match x_height {
                Some(height) => height,
                None => {
                    // If not set - fallback to height * 45%.
                    // 45% is what Firefox uses.
                    u16::try_from((f32::from(ascent - descent) * 0.45) as i32)
                        .ok()
                        .and_then(NonZeroU16::new)?
                }
            };

            let line_through = font.strikeout_metrics();
            let line_through_position = match line_through {
                Some(metrics) => metrics.position,
                None => x_height.get() as i16 / 2,
            };

            let (underline_position, underline_thickness) = match font.underline_metrics() {
                Some(metrics) => {
                    let thickness = u16::try_from(metrics.thickness)
                        .ok()
                        .and_then(NonZeroU16::new)
                        // `ttf_parser` guarantees that units_per_em is >= 16
                        .unwrap_or_else(|| NonZeroU16::new(units_per_em.get() / 12).unwrap());

                    (metrics.position, thickness)
                }
                None => (
                    -(units_per_em.get() as i16) / 9,
                    NonZeroU16::new(units_per_em.get() / 12).unwrap(),
                ),
            };

            // 0.2 and 0.4 are generic offsets used by some applications (Inkscape/librsvg).
            let mut subscript_offset = (units_per_em.get() as f32 / 0.2).round() as i16;
            let mut superscript_offset = (units_per_em.get() as f32 / 0.4).round() as i16;
            if let Some(metrics) = font.subscript_metrics() {
                subscript_offset = metrics.y_offset;
            }

            if let Some(metrics) = font.superscript_metrics() {
                superscript_offset = metrics.y_offset;
            }

            Some(ResolvedFont {
                id,
                units_per_em,
                ascent,
                descent,
                x_height,
                underline_position,
                underline_thickness,
                line_through_position,
                subscript_offset,
                superscript_offset,
            })
        })?
    }

    #[inline(never)]
    fn has_char(&self, id: ID, c: char) -> bool {
        let res = self.with_face_data(id, |font_data, face_index| -> Option<bool> {
            let font = ttf_parser::Face::parse(font_data, face_index).ok()?;
            font.glyph_index(c)?;
            Some(true)
        });

        res == Some(Some(true))
    }
}

/// Text shaping with font fallback.
pub(crate) fn shape_text(
    text: &str,
    font: Arc<ResolvedFont>,
    small_caps: bool,
    apply_kerning: bool,
    fontdb: &fontdb::Database,
) -> Vec<Glyph> {
    let mut glyphs = shape_text_with_font(text, font.clone(), small_caps, apply_kerning, fontdb)
        .unwrap_or_default();

    // Remember all fonts used for shaping.
    let mut used_fonts = vec![font.id];

    // Loop until all glyphs become resolved or until no more fonts are left.
    'outer: loop {
        let mut missing = None;
        for glyph in &glyphs {
            if glyph.is_missing() {
                missing = Some(glyph.byte_idx.char_from(text));
                break;
            }
        }

        if let Some(c) = missing {
            let fallback_font = match find_font_for_char(c, &used_fonts, fontdb) {
                Some(v) => Arc::new(v),
                None => break 'outer,
            };

            // Shape again, using a new font.
            let fallback_glyphs = shape_text_with_font(
                text,
                fallback_font.clone(),
                small_caps,
                apply_kerning,
                fontdb,
            )
            .unwrap_or_default();

            let all_matched = fallback_glyphs.iter().all(|g| !g.is_missing());
            if all_matched {
                // Replace all glyphs when all of them were matched.
                glyphs = fallback_glyphs;
                break 'outer;
            }

            // We assume, that shaping with an any font will produce the same amount of glyphs.
            // This is incorrect, but good enough for now.
            if glyphs.len() != fallback_glyphs.len() {
                break 'outer;
            }

            // TODO: Replace clusters and not glyphs. This should be more accurate.

            // Copy new glyphs.
            for i in 0..glyphs.len() {
                if glyphs[i].is_missing() && !fallback_glyphs[i].is_missing() {
                    glyphs[i] = fallback_glyphs[i].clone();
                }
            }

            // Remember this font.
            used_fonts.push(fallback_font.id);
        } else {
            break 'outer;
        }
    }

    // Warn about missing glyphs.
    for glyph in &glyphs {
        if glyph.is_missing() {
            let c = glyph.byte_idx.char_from(text);
            // TODO: print a full grapheme
            log::warn!(
                "No fonts with a {}/U+{:X} character were found.",
                c,
                c as u32
            );
        }
    }

    glyphs
}

/// Converts a text into a list of glyph IDs.
///
/// This function will do the BIDI reordering and text shaping.
fn shape_text_with_font(
    text: &str,
    font: Arc<ResolvedFont>,
    small_caps: bool,
    apply_kerning: bool,
    fontdb: &fontdb::Database,
) -> Option<Vec<Glyph>> {
    fontdb.with_face_data(font.id, |font_data, face_index| -> Option<Vec<Glyph>> {
        let rb_font = rustybuzz::Face::from_slice(font_data, face_index)?;

        let bidi_info = unicode_bidi::BidiInfo::new(text, Some(unicode_bidi::Level::ltr()));
        let paragraph = &bidi_info.paragraphs[0];
        let line = paragraph.range.clone();

        let mut glyphs = Vec::new();

        let (levels, runs) = bidi_info.visual_runs(paragraph, line);
        for run in runs.iter() {
            let sub_text = &text[run.clone()];
            if sub_text.is_empty() {
                continue;
            }

            let hb_direction = if levels[run.start].is_rtl() {
                rustybuzz::Direction::RightToLeft
            } else {
                rustybuzz::Direction::LeftToRight
            };

            let mut buffer = rustybuzz::UnicodeBuffer::new();
            buffer.push_str(sub_text);
            buffer.set_direction(hb_direction);

            let mut features = Vec::new();
            if small_caps {
                features.push(rustybuzz::Feature::new(
                    rustybuzz::Tag::from_bytes(b"smcp"),
                    1,
                    ..,
                ));
            }

            if !apply_kerning {
                features.push(rustybuzz::Feature::new(
                    rustybuzz::Tag::from_bytes(b"kern"),
                    0,
                    ..,
                ));
            }

            let output = rustybuzz::shape(&rb_font, &features, buffer);

            let positions = output.glyph_positions();
            let infos = output.glyph_infos();

            for (pos, info) in positions.iter().zip(infos) {
                let idx = run.start + info.cluster as usize;
                debug_assert!(text.get(idx..).is_some());

                glyphs.push(Glyph {
                    byte_idx: ByteIndex::new(idx),
                    id: GlyphId(info.glyph_id as u16),
                    dx: pos.x_offset,
                    dy: pos.y_offset,
                    width: pos.x_advance,
                    font: font.clone(),
                });
            }
        }

        Some(glyphs)
    })?
}

/// Finds a font with a specified char.
///
/// This is a rudimentary font fallback algorithm.
fn find_font_for_char(
    c: char,
    exclude_fonts: &[fontdb::ID],
    fontdb: &fontdb::Database,
) -> Option<ResolvedFont> {
    let base_font_id = exclude_fonts[0];

    // Iterate over fonts and check if any of them support the specified char.
    for face in fontdb.faces() {
        // Ignore fonts, that were used for shaping already.
        if exclude_fonts.contains(&face.id) {
            continue;
        }

        // Check that the new face has the same style.
        let base_face = fontdb.face(base_font_id)?;
        if base_face.style != face.style
            && base_face.weight != face.weight
            && base_face.stretch != face.stretch
        {
            continue;
        }

        if !fontdb.has_char(face.id, c) {
            continue;
        }

        let base_family = base_face
            .families
            .iter()
            .find(|f| f.1 == fontdb::Language::English_UnitedStates)
            .unwrap_or(&base_face.families[0]);

        let new_family = face
            .families
            .iter()
            .find(|f| f.1 == fontdb::Language::English_UnitedStates)
            .unwrap_or(&base_face.families[0]);

        log::warn!("Fallback from {} to {}.", base_family.0, new_family.0);
        return fontdb.load_font(face.id);
    }

    None
}

/// An iterator over glyph clusters.
///
/// Input:  0 2 2 2 3 4 4 5 5
/// Result: 0 1     4 5   7
pub(crate) struct GlyphClusters<'a> {
    data: &'a [Glyph],
    idx: usize,
}

impl<'a> GlyphClusters<'a> {
    pub(crate) fn new(data: &'a [Glyph]) -> Self {
        GlyphClusters { data, idx: 0 }
    }
}

impl<'a> Iterator for GlyphClusters<'a> {
    type Item = (std::ops::Range<usize>, ByteIndex);

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx == self.data.len() {
            return None;
        }

        let start = self.idx;
        let cluster = self.data[self.idx].byte_idx;
        for g in &self.data[self.idx..] {
            if g.byte_idx != cluster {
                break;
            }

            self.idx += 1;
        }

        Some((start..self.idx, cluster))
    }
}

/// Checks that selected script supports letter spacing.
///
/// [In the CSS spec](https://www.w3.org/TR/css-text-3/#cursive-tracking).
///
/// The list itself is from: https://github.com/harfbuzz/harfbuzz/issues/64
pub(crate) fn script_supports_letter_spacing(script: unicode_script::Script) -> bool {
    use unicode_script::Script;

    !matches!(
        script,
        Script::Arabic
            | Script::Syriac
            | Script::Nko
            | Script::Manichaean
            | Script::Psalter_Pahlavi
            | Script::Mandaic
            | Script::Mongolian
            | Script::Phags_Pa
            | Script::Devanagari
            | Script::Bengali
            | Script::Gurmukhi
            | Script::Modi
            | Script::Sharada
            | Script::Syloti_Nagri
            | Script::Tirhuta
            | Script::Ogham
    )
}

/// A glyph.
///
/// Basically, a glyph ID and it's metrics.
#[derive(Clone)]
pub(crate) struct Glyph {
    /// The glyph ID in the font.
    pub(crate) id: GlyphId,

    /// Position in bytes in the original string.
    ///
    /// We use it to match a glyph with a character in the text chunk and therefore with the style.
    pub(crate) byte_idx: ByteIndex,

    /// The glyph offset in font units.
    pub(crate) dx: i32,

    /// The glyph offset in font units.
    pub(crate) dy: i32,

    /// The glyph width / X-advance in font units.
    pub(crate) width: i32,

    /// Reference to the source font.
    ///
    /// Each glyph can have it's own source font.
    pub(crate) font: Arc<ResolvedFont>,
}

impl Glyph {
    fn is_missing(&self) -> bool {
        self.id.0 == 0
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ResolvedFont {
    pub(crate) id: ID,

    units_per_em: NonZeroU16,

    // All values below are in font units.
    ascent: i16,
    descent: i16,
    x_height: NonZeroU16,

    underline_position: i16,
    underline_thickness: NonZeroU16,

    // line-through thickness should be the the same as underline thickness
    // according to the TrueType spec:
    // https://docs.microsoft.com/en-us/typography/opentype/spec/os2#ystrikeoutsize
    line_through_position: i16,

    subscript_offset: i16,
    superscript_offset: i16,
}

pub(crate) fn chunk_span_at(chunk: &TextChunk, byte_offset: ByteIndex) -> Option<&TextSpan> {
    chunk
        .spans
        .iter()
        .find(|&span| span_contains(span, byte_offset))
}

pub(crate) fn span_contains(span: &TextSpan, byte_offset: ByteIndex) -> bool {
    byte_offset.value() >= span.start && byte_offset.value() < span.end
}

/// Checks that the selected character is a word separator.
///
/// According to: https://www.w3.org/TR/css-text-3/#word-separator
pub(crate) fn is_word_separator_characters(c: char) -> bool {
    matches!(
        c as u32,
        0x0020 | 0x00A0 | 0x1361 | 0x010100 | 0x010101 | 0x01039F | 0x01091F
    )
}

impl ResolvedFont {
    #[inline]
    pub(crate) fn scale(&self, font_size: f32) -> f32 {
        font_size / self.units_per_em.get() as f32
    }

    #[inline]
    pub(crate) fn ascent(&self, font_size: f32) -> f32 {
        self.ascent as f32 * self.scale(font_size)
    }

    #[inline]
    pub(crate) fn descent(&self, font_size: f32) -> f32 {
        self.descent as f32 * self.scale(font_size)
    }

    #[inline]
    pub(crate) fn height(&self, font_size: f32) -> f32 {
        self.ascent(font_size) - self.descent(font_size)
    }

    #[inline]
    pub(crate) fn x_height(&self, font_size: f32) -> f32 {
        self.x_height.get() as f32 * self.scale(font_size)
    }

    #[inline]
    pub(crate) fn underline_position(&self, font_size: f32) -> f32 {
        self.underline_position as f32 * self.scale(font_size)
    }

    #[inline]
    fn underline_thickness(&self, font_size: f32) -> f32 {
        self.underline_thickness.get() as f32 * self.scale(font_size)
    }

    #[inline]
    pub(crate) fn line_through_position(&self, font_size: f32) -> f32 {
        self.line_through_position as f32 * self.scale(font_size)
    }

    #[inline]
    fn subscript_offset(&self, font_size: f32) -> f32 {
        self.subscript_offset as f32 * self.scale(font_size)
    }

    #[inline]
    fn superscript_offset(&self, font_size: f32) -> f32 {
        self.superscript_offset as f32 * self.scale(font_size)
    }

    fn dominant_baseline_shift(&self, baseline: DominantBaseline, font_size: f32) -> f32 {
        let alignment = match baseline {
            DominantBaseline::Auto => AlignmentBaseline::Auto,
            DominantBaseline::UseScript => AlignmentBaseline::Auto, // unsupported
            DominantBaseline::NoChange => AlignmentBaseline::Auto,  // already resolved
            DominantBaseline::ResetSize => AlignmentBaseline::Auto, // unsupported
            DominantBaseline::Ideographic => AlignmentBaseline::Ideographic,
            DominantBaseline::Alphabetic => AlignmentBaseline::Alphabetic,
            DominantBaseline::Hanging => AlignmentBaseline::Hanging,
            DominantBaseline::Mathematical => AlignmentBaseline::Mathematical,
            DominantBaseline::Central => AlignmentBaseline::Central,
            DominantBaseline::Middle => AlignmentBaseline::Middle,
            DominantBaseline::TextAfterEdge => AlignmentBaseline::TextAfterEdge,
            DominantBaseline::TextBeforeEdge => AlignmentBaseline::TextBeforeEdge,
        };

        self.alignment_baseline_shift(alignment, font_size)
    }

    // The `alignment-baseline` property is a mess.
    //
    // The SVG 1.1 spec (https://www.w3.org/TR/SVG11/text.html#BaselineAlignmentProperties)
    // goes on and on about what this property suppose to do, but doesn't actually explain
    // how it should be implemented. It's just a very verbose overview.
    //
    // As of Nov 2022, only Chrome and Safari support `alignment-baseline`. Firefox isn't.
    // Same goes for basically every SVG library in existence.
    // Meaning we have no idea how exactly it should be implemented.
    //
    // And even Chrome and Safari cannot agree on how to handle `baseline`, `after-edge`,
    // `text-after-edge` and `ideographic` variants. Producing vastly different output.
    //
    // As per spec, a proper implementation should get baseline values from the font itself,
    // using `BASE` and `bsln` TrueType tables. If those tables are not present,
    // we have to synthesize them (https://drafts.csswg.org/css-inline/#baseline-synthesis-fonts).
    // And in the worst case scenario simply fallback to hardcoded values.
    //
    // Also, most fonts do not provide `BASE` and `bsln` tables to begin with.
    //
    // Again, as of Nov 2022, Chrome does only the latter:
    // https://github.com/chromium/chromium/blob/main/third_party/blink/renderer/platform/fonts/font_metrics.cc#L153
    //
    // Since baseline TrueType tables parsing and baseline synthesis are pretty hard,
    // we do what Chrome does - use hardcoded values. And it seems like Safari does the same.
    //
    //
    // But that's not all! SVG 2 and CSS Inline Layout 3 did a baseline handling overhaul,
    // and it's far more complex now. Not sure if anyone actually supports it.
    fn alignment_baseline_shift(&self, alignment: AlignmentBaseline, font_size: f32) -> f32 {
        match alignment {
            AlignmentBaseline::Auto => 0.0,
            AlignmentBaseline::Baseline => 0.0,
            AlignmentBaseline::BeforeEdge | AlignmentBaseline::TextBeforeEdge => {
                self.ascent(font_size)
            }
            AlignmentBaseline::Middle => self.x_height(font_size) * 0.5,
            AlignmentBaseline::Central => self.ascent(font_size) - self.height(font_size) * 0.5,
            AlignmentBaseline::AfterEdge | AlignmentBaseline::TextAfterEdge => {
                self.descent(font_size)
            }
            AlignmentBaseline::Ideographic => self.descent(font_size),
            AlignmentBaseline::Alphabetic => 0.0,
            AlignmentBaseline::Hanging => self.ascent(font_size) * 0.8,
            AlignmentBaseline::Mathematical => self.ascent(font_size) * 0.5,
        }
    }
}

pub(crate) type FontsCache = HashMap<Font, Arc<ResolvedFont>>;

/// A read-only text index in bytes.
///
/// Guarantee to be on a char boundary and in text bounds.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct ByteIndex(usize);

impl ByteIndex {
    fn new(i: usize) -> Self {
        ByteIndex(i)
    }

    pub(crate) fn value(&self) -> usize {
        self.0
    }

    /// Converts byte position into a code point position.
    pub(crate) fn code_point_at(&self, text: &str) -> usize {
        text.char_indices()
            .take_while(|(i, _)| *i != self.0)
            .count()
    }

    /// Converts byte position into a character.
    pub(crate) fn char_from(&self, text: &str) -> char {
        text[self.0..].chars().next().unwrap()
    }
}
