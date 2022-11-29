// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/*!
An [SVG] text layout implementation on top of [`usvg`] crate.

[usvg]: https://github.com/RazrFalcon/resvg/usvg
[SVG]: https://en.wikipedia.org/wiki/Scalable_Vector_Graphics
*/

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![warn(missing_copy_implementations)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::neg_cmp_op_on_partial_ord)]
#![allow(clippy::identity_op)]
#![allow(clippy::question_mark)]
#![allow(clippy::upper_case_acronyms)]

pub use fontdb;

use std::collections::HashMap;
use std::convert::TryFrom;
use std::num::NonZeroU16;
use std::rc::Rc;

use fontdb::{Database, ID};
use kurbo::{ParamCurve, ParamCurveArclen, ParamCurveDeriv};
use rustybuzz::ttf_parser;
use ttf_parser::GlyphId;
use unicode_script::UnicodeScript;
use usvg::*;

/// A `usvg::Tree` extension trait.
pub trait TreeTextToPath {
    /// Converts text nodes into paths.
    ///
    /// We have not pass `Options::keep_named_groups` again,
    /// since this method affects the tree structure.
    fn convert_text(&mut self, fontdb: &fontdb::Database, keep_named_groups: bool);
}

impl TreeTextToPath for usvg::Tree {
    fn convert_text(&mut self, fontdb: &fontdb::Database, keep_named_groups: bool) {
        convert_text(self.root.clone(), fontdb, keep_named_groups);
    }
}

/// A `usvg::Text` extension trait.
pub trait TextToPath {
    /// Converts the text node into path(s).
    ///
    /// `absolute_ts` is node's absolute transform. Used primarily during text-on-path resolving.
    fn convert(&self, fontdb: &fontdb::Database, absolute_ts: Transform) -> Option<Node>;
}

impl TextToPath for Text {
    fn convert(&self, fontdb: &fontdb::Database, absolute_ts: Transform) -> Option<Node> {
        let (new_paths, bbox) = text_to_paths(self, fontdb, absolute_ts);
        if new_paths.is_empty() {
            return None;
        }

        // Create a group will all paths that was created during text-to-path conversion.
        let group = Node::new(NodeKind::Group(Group {
            id: self.id.clone(),
            transform: self.transform,
            ..Group::default()
        }));

        let rendering_mode = resolve_rendering_mode(self);
        for mut path in new_paths {
            fix_obj_bounding_box(&mut path, bbox);
            path.rendering_mode = rendering_mode;
            group.append_kind(NodeKind::Path(path));
        }

        Some(group)
    }
}

fn convert_text(root: Node, fontdb: &fontdb::Database, keep_named_groups: bool) {
    let mut text_nodes = Vec::new();
    // We have to update text nodes in clipPaths, masks and patterns as well.
    for node in root.descendants() {
        match *node.borrow() {
            NodeKind::Group(ref g) => {
                if let Some(ref clip) = g.clip_path {
                    convert_text(clip.root.clone(), fontdb, keep_named_groups);
                }

                if let Some(ref mask) = g.mask {
                    convert_text(mask.root.clone(), fontdb, keep_named_groups);
                }
            }
            NodeKind::Path(ref path) => {
                if let Some(ref fill) = path.fill {
                    if let Paint::Pattern(ref p) = fill.paint {
                        convert_text(p.root.clone(), fontdb, keep_named_groups);
                    }
                }
                if let Some(ref stroke) = path.stroke {
                    if let Paint::Pattern(ref p) = stroke.paint {
                        convert_text(p.root.clone(), fontdb, keep_named_groups);
                    }
                }
            }
            NodeKind::Image(_) => {}
            NodeKind::Text(ref text) => {
                text_nodes.push(node.clone());

                for chunk in &text.chunks {
                    for span in &chunk.spans {
                        if let Some(ref fill) = span.fill {
                            if let Paint::Pattern(ref p) = fill.paint {
                                convert_text(p.root.clone(), fontdb, keep_named_groups);
                            }
                        }
                        if let Some(ref stroke) = span.stroke {
                            if let Paint::Pattern(ref p) = stroke.paint {
                                convert_text(p.root.clone(), fontdb, keep_named_groups);
                            }
                        }
                    }
                }
            }
        }
    }

    if text_nodes.is_empty() {
        return;
    }

    for node in &text_nodes {
        let mut new_node = None;
        if let NodeKind::Text(ref text) = *node.borrow() {
            let mut absolute_ts = node.parent().unwrap().abs_transform();
            absolute_ts.append(&text.transform);
            new_node = text.convert(fontdb, absolute_ts);
        }

        if let Some(new_node) = new_node {
            node.insert_after(new_node);
        }
    }

    text_nodes.iter().for_each(|n| n.detach());
    Tree::ungroup_groups(root, keep_named_groups);
}

trait DatabaseExt {
    fn load_font(&self, id: ID) -> Option<ResolvedFont>;
    fn outline(&self, id: ID, glyph_id: GlyphId) -> Option<PathData>;
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
    fn outline(&self, id: ID, glyph_id: GlyphId) -> Option<PathData> {
        self.with_face_data(id, |data, face_index| -> Option<PathData> {
            let font = ttf_parser::Face::parse(data, face_index).ok()?;

            let mut builder = PathBuilder {
                path: PathData::new(),
            };
            font.outline_glyph(glyph_id, &mut builder)?;
            Some(builder.path)
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

#[derive(Clone, Copy, Debug)]
struct ResolvedFont {
    id: ID,

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

impl ResolvedFont {
    #[inline]
    fn scale(&self, font_size: f64) -> f64 {
        font_size / self.units_per_em.get() as f64
    }

    #[inline]
    fn ascent(&self, font_size: f64) -> f64 {
        self.ascent as f64 * self.scale(font_size)
    }

    #[inline]
    fn descent(&self, font_size: f64) -> f64 {
        self.descent as f64 * self.scale(font_size)
    }

    #[inline]
    fn height(&self, font_size: f64) -> f64 {
        self.ascent(font_size) - self.descent(font_size)
    }

    #[inline]
    fn x_height(&self, font_size: f64) -> f64 {
        self.x_height.get() as f64 * self.scale(font_size)
    }

    #[inline]
    fn underline_position(&self, font_size: f64) -> f64 {
        self.underline_position as f64 * self.scale(font_size)
    }

    #[inline]
    fn underline_thickness(&self, font_size: f64) -> f64 {
        self.underline_thickness.get() as f64 * self.scale(font_size)
    }

    #[inline]
    fn line_through_position(&self, font_size: f64) -> f64 {
        self.line_through_position as f64 * self.scale(font_size)
    }

    #[inline]
    fn subscript_offset(&self, font_size: f64) -> f64 {
        self.subscript_offset as f64 * self.scale(font_size)
    }

    #[inline]
    fn superscript_offset(&self, font_size: f64) -> f64 {
        self.superscript_offset as f64 * self.scale(font_size)
    }

    fn dominant_baseline_shift(&self, baseline: DominantBaseline, font_size: f64) -> f64 {
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
    fn alignment_baseline_shift(&self, alignment: AlignmentBaseline, font_size: f64) -> f64 {
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

struct PathBuilder {
    path: PathData,
}

impl ttf_parser::OutlineBuilder for PathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path.push_move_to(x as f64, y as f64);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.path.push_line_to(x as f64, y as f64);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.path
            .push_quad_to(x1 as f64, y1 as f64, x as f64, y as f64);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.path.push_curve_to(
            x1 as f64, y1 as f64, x2 as f64, y2 as f64, x as f64, y as f64,
        );
    }

    fn close(&mut self) {
        self.path.push_close_path();
    }
}

/// A read-only text index in bytes.
///
/// Guarantee to be on a char boundary and in text bounds.
#[derive(Clone, Copy, PartialEq)]
struct ByteIndex(usize);

impl ByteIndex {
    fn new(i: usize) -> Self {
        ByteIndex(i)
    }

    fn value(&self) -> usize {
        self.0
    }

    /// Converts byte position into a code point position.
    fn code_point_at(&self, text: &str) -> usize {
        text.char_indices()
            .take_while(|(i, _)| *i != self.0)
            .count()
    }

    /// Converts byte position into a character.
    fn char_from(&self, text: &str) -> char {
        text[self.0..].chars().next().unwrap()
    }
}

fn resolve_rendering_mode(text: &Text) -> ShapeRendering {
    match text.rendering_mode {
        TextRendering::OptimizeSpeed => ShapeRendering::CrispEdges,
        TextRendering::OptimizeLegibility => ShapeRendering::GeometricPrecision,
        TextRendering::GeometricPrecision => ShapeRendering::GeometricPrecision,
    }
}

fn chunk_span_at(chunk: &TextChunk, byte_offset: ByteIndex) -> Option<&TextSpan> {
    for span in &chunk.spans {
        if span_contains(span, byte_offset) {
            return Some(span);
        }
    }

    None
}

fn span_contains(span: &TextSpan, byte_offset: ByteIndex) -> bool {
    byte_offset.value() >= span.start && byte_offset.value() < span.end
}

// Baseline resolving in SVG is a mess.
// Not only it's poorly documented, but as soon as you start mixing
// `dominant-baseline` and `alignment-baseline` each application/browser will produce
// different results.
//
// For now, resvg simply tries to match Chrome's output and not the mythical SVG spec output.
//
// See `alignment_baseline_shift` method comment for more details.
fn resolve_baseline(span: &TextSpan, font: &ResolvedFont, writing_mode: WritingMode) -> f64 {
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

    return shift;
}

type FontsCache = HashMap<Font, Rc<ResolvedFont>>;

fn text_to_paths(
    text_node: &Text,
    fontdb: &fontdb::Database,
    abs_ts: Transform,
) -> (Vec<Path>, PathBbox) {
    let mut fonts_cache: FontsCache = HashMap::new();
    for chunk in &text_node.chunks {
        for span in &chunk.spans {
            if !fonts_cache.contains_key(&span.font) {
                if let Some(font) = resolve_font(&span.font, fontdb) {
                    fonts_cache.insert(span.font.clone(), Rc::new(font));
                }
            }
        }
    }

    let mut bbox = PathBbox::new_bbox();
    let mut char_offset = 0;
    let mut last_x = 0.0;
    let mut last_y = 0.0;
    let mut new_paths = Vec::new();
    for chunk in &text_node.chunks {
        let (x, y) = match chunk.text_flow {
            TextFlow::Linear => (chunk.x.unwrap_or(last_x), chunk.y.unwrap_or(last_y)),
            TextFlow::Path(_) => (0.0, 0.0),
        };

        let mut clusters = outline_chunk(chunk, &fonts_cache, fontdb);
        if clusters.is_empty() {
            char_offset += chunk.text.chars().count();
            continue;
        }

        apply_writing_mode(text_node.writing_mode, &mut clusters);
        apply_letter_spacing(chunk, &mut clusters);
        apply_word_spacing(chunk, &mut clusters);
        apply_length_adjust(chunk, &mut clusters);
        let mut curr_pos = resolve_clusters_positions(
            chunk,
            char_offset,
            &text_node.positions,
            &text_node.rotate,
            text_node.writing_mode,
            abs_ts,
            &fonts_cache,
            &mut clusters,
        );

        let mut text_ts = Transform::default();
        if text_node.writing_mode == WritingMode::TopToBottom {
            if let TextFlow::Linear = chunk.text_flow {
                text_ts.rotate_at(90.0, x, y);
            }
        }

        for span in &chunk.spans {
            let font = match fonts_cache.get(&span.font) {
                Some(v) => v,
                None => continue,
            };

            let decoration_spans = collect_decoration_spans(span, &clusters);

            let mut span_ts = text_ts;
            span_ts.translate(x, y);
            if let TextFlow::Linear = chunk.text_flow {
                let shift = resolve_baseline(span, font, text_node.writing_mode);

                // In case of a horizontal flow, shift transform and not clusters,
                // because clusters can be rotated and an additional shift will lead
                // to invalid results.
                span_ts.translate(0.0, shift);
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

                let path =
                    convert_decoration(offset, span, font, decoration, &decoration_spans, span_ts);

                if let Some(r) = path.data.bbox() {
                    bbox = bbox.expand(r);
                }

                new_paths.push(path);
            }

            if let Some(decoration) = span.decoration.overline.clone() {
                let offset = match text_node.writing_mode {
                    WritingMode::LeftToRight => -font.ascent(span.font_size.get()),
                    WritingMode::TopToBottom => -font.height(span.font_size.get()) / 2.0,
                };

                let path =
                    convert_decoration(offset, span, font, decoration, &decoration_spans, span_ts);

                if let Some(r) = path.data.bbox() {
                    bbox = bbox.expand(r);
                }

                new_paths.push(path);
            }

            if let Some(path) = convert_span(span, &mut clusters, &span_ts) {
                // Use `text_bbox` here and not `path.data.bbox()`.
                if let Some(r) = path.text_bbox {
                    bbox = bbox.expand(r.to_path_bbox());
                }

                new_paths.push(path);
            }

            if let Some(decoration) = span.decoration.line_through.clone() {
                let offset = match text_node.writing_mode {
                    WritingMode::LeftToRight => -font.line_through_position(span.font_size.get()),
                    WritingMode::TopToBottom => 0.0,
                };

                let path =
                    convert_decoration(offset, span, font, decoration, &decoration_spans, span_ts);

                if let Some(r) = path.data.bbox() {
                    bbox = bbox.expand(r);
                }

                new_paths.push(path);
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

    (new_paths, bbox)
}

fn resolve_font(font: &Font, fontdb: &fontdb::Database) -> Option<ResolvedFont> {
    let mut name_list = Vec::new();
    for family in &font.families {
        name_list.push(match family.as_str() {
            "serif" => fontdb::Family::Serif,
            "sans-serif" => fontdb::Family::SansSerif,
            "cursive" => fontdb::Family::Cursive,
            "fantasy" => fontdb::Family::Fantasy,
            "monospace" => fontdb::Family::Monospace,
            _ => fontdb::Family::Name(family),
        });
    }

    // Use the default font as fallback.
    name_list.push(fontdb::Family::Serif);

    let stretch = match font.stretch {
        Stretch::UltraCondensed => fontdb::Stretch::UltraCondensed,
        Stretch::ExtraCondensed => fontdb::Stretch::ExtraCondensed,
        Stretch::Condensed => fontdb::Stretch::Condensed,
        Stretch::SemiCondensed => fontdb::Stretch::SemiCondensed,
        Stretch::Normal => fontdb::Stretch::Normal,
        Stretch::SemiExpanded => fontdb::Stretch::SemiExpanded,
        Stretch::Expanded => fontdb::Stretch::Expanded,
        Stretch::ExtraExpanded => fontdb::Stretch::ExtraExpanded,
        Stretch::UltraExpanded => fontdb::Stretch::UltraExpanded,
    };

    let style = match font.style {
        Style::Normal => fontdb::Style::Normal,
        Style::Italic => fontdb::Style::Italic,
        Style::Oblique => fontdb::Style::Oblique,
    };

    let query = fontdb::Query {
        families: &name_list,
        weight: fontdb::Weight(font.weight),
        stretch,
        style,
    };

    let id = fontdb.query(&query);
    if id.is_none() {
        log::warn!("No match for '{}' font-family.", font.families.join(", "));
    }

    fontdb.load_font(id?)
}

fn convert_span(
    span: &TextSpan,
    clusters: &mut [OutlinedCluster],
    text_ts: &Transform,
) -> Option<Path> {
    let mut path_data = PathData::new();
    let mut bboxes_data = PathData::new();

    for cluster in clusters {
        if !cluster.visible {
            continue;
        }

        if span_contains(span, cluster.byte_idx) {
            let mut path = std::mem::replace(&mut cluster.path, PathData::new());
            path.transform(cluster.transform);

            path_data.push_path(&path);

            // We have to calculate text bbox using font metrics and not glyph shape.
            if let Some(r) = Rect::new(0.0, -cluster.ascent, cluster.advance, cluster.height()) {
                if let Some(r) = r.transform(&cluster.transform) {
                    bboxes_data.push_rect(r);
                }
            }
        }
    }

    if path_data.is_empty() {
        return None;
    }

    path_data.transform(*text_ts);
    bboxes_data.transform(*text_ts);

    let mut fill = span.fill.clone();
    if let Some(ref mut fill) = fill {
        // The `fill-rule` should be ignored.
        // https://www.w3.org/TR/SVG2/text.html#TextRenderingOrder
        //
        // 'Since the fill-rule property does not apply to SVG text elements,
        // the specific order of the subpaths within the equivalent path does not matter.'
        fill.rule = FillRule::NonZero;
    }

    let path = Path {
        id: String::new(),
        transform: Transform::default(),
        visibility: span.visibility,
        fill,
        stroke: span.stroke.clone(),
        paint_order: span.paint_order,
        rendering_mode: ShapeRendering::default(),
        text_bbox: bboxes_data.bbox().and_then(|r| r.to_rect()),
        data: Rc::new(path_data),
    };

    Some(path)
}

fn collect_decoration_spans(span: &TextSpan, clusters: &[OutlinedCluster]) -> Vec<DecorationSpan> {
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

fn convert_decoration(
    dy: f64,
    span: &TextSpan,
    font: &ResolvedFont,
    mut decoration: TextDecorationStyle,
    decoration_spans: &[DecorationSpan],
    transform: Transform,
) -> Path {
    debug_assert!(!decoration_spans.is_empty());

    let thickness = font.underline_thickness(span.font_size.get());

    let mut path = PathData::new();
    for dec_span in decoration_spans {
        let rect = Rect::new(0.0, -thickness / 2.0, dec_span.width, thickness).unwrap();

        let start_idx = path.len();
        path.push_rect(rect);

        let mut ts = dec_span.transform;
        ts.translate(0.0, dy);
        path.transform_from(start_idx, ts);
    }

    path.transform(transform);

    Path {
        visibility: span.visibility,
        fill: decoration.fill.take(),
        stroke: decoration.stroke.take(),
        data: Rc::new(path),
        ..Path::default()
    }
}

/// By the SVG spec, `tspan` doesn't have a bbox and uses the parent `text` bbox.
/// Since we converted `text` and `tspan` to `path`, we have to update
/// all linked paint servers (gradients and patterns) too.
fn fix_obj_bounding_box(path: &mut Path, bbox: PathBbox) {
    if let Some(ref mut fill) = path.fill {
        if let Some(new_paint) = paint_server_to_user_space_on_use(fill.paint.clone(), bbox) {
            fill.paint = new_paint;
        }
    }

    if let Some(ref mut stroke) = path.stroke {
        if let Some(new_paint) = paint_server_to_user_space_on_use(stroke.paint.clone(), bbox) {
            stroke.paint = new_paint;
        }
    }
}

/// Converts a selected paint server's units to `UserSpaceOnUse`.
///
/// Creates a deep copy of a selected paint server and returns its ID.
///
/// Returns `None` if a paint server already uses `UserSpaceOnUse`.
fn paint_server_to_user_space_on_use(paint: Paint, bbox: PathBbox) -> Option<Paint> {
    if paint.units() != Some(Units::ObjectBoundingBox) {
        return None;
    }

    // TODO: is `pattern` copying safe? Maybe we should reset id's on all `pattern` children.
    // We have to clone a paint server, in case some other element is already using it.
    // If not, the `convert` module will remove unused defs anyway.

    // Update id, transform and units.
    let ts = Transform::from_bbox(bbox.to_rect()?);
    let paint = match paint {
        Paint::Color(_) => paint,
        Paint::LinearGradient(ref lg) => {
            let mut transform = lg.transform;
            transform.prepend(&ts);
            Paint::LinearGradient(Rc::new(LinearGradient {
                id: String::new(),
                x1: lg.x1,
                y1: lg.y1,
                x2: lg.x2,
                y2: lg.y2,
                base: BaseGradient {
                    units: Units::UserSpaceOnUse,
                    transform,
                    spread_method: lg.spread_method,
                    stops: lg.stops.clone(),
                },
            }))
        }
        Paint::RadialGradient(ref rg) => {
            let mut transform = rg.transform;
            transform.prepend(&ts);
            Paint::RadialGradient(Rc::new(RadialGradient {
                id: String::new(),
                cx: rg.cx,
                cy: rg.cy,
                r: rg.r,
                fx: rg.fx,
                fy: rg.fy,
                base: BaseGradient {
                    units: Units::UserSpaceOnUse,
                    transform,
                    spread_method: rg.spread_method,
                    stops: rg.stops.clone(),
                },
            }))
        }
        Paint::Pattern(ref patt) => {
            let mut transform = patt.transform;
            transform.prepend(&ts);
            Paint::Pattern(Rc::new(Pattern {
                id: String::new(),
                units: Units::UserSpaceOnUse,
                content_units: patt.content_units,
                transform: transform,
                rect: patt.rect,
                view_box: patt.view_box,
                root: patt.root.clone().make_deep_copy(),
            }))
        }
    };

    Some(paint)
}

/// A text decoration span.
///
/// Basically a horizontal line, that will be used for underline, overline and line-through.
/// It doesn't have a height, since it depends on the Font metrics.
#[derive(Clone, Copy)]
struct DecorationSpan {
    width: f64,
    transform: Transform,
}

/// A glyph.
///
/// Basically, a glyph ID and it's metrics.
#[derive(Clone)]
struct Glyph {
    /// The glyph ID in the font.
    id: GlyphId,

    /// Position in bytes in the original string.
    ///
    /// We use it to match a glyph with a character in the text chunk and therefore with the style.
    byte_idx: ByteIndex,

    /// The glyph offset in font units.
    dx: i32,

    /// The glyph offset in font units.
    dy: i32,

    /// The glyph width / X-advance in font units.
    width: i32,

    /// Reference to the source font.
    ///
    /// Each glyph can have it's own source font.
    font: Rc<ResolvedFont>,
}

impl Glyph {
    fn is_missing(&self) -> bool {
        self.id.0 == 0
    }
}

/// An outlined cluster.
///
/// Cluster/grapheme is a single, unbroken, renderable character.
/// It can be positioned, rotated, spaced, etc.
///
/// Let's say we have `й` which is *CYRILLIC SMALL LETTER I* and *COMBINING BREVE*.
/// It consists of two code points, will be shaped (via harfbuzz) as two glyphs into one cluster,
/// and then will be combined into the one `OutlinedCluster`.
#[derive(Clone)]
struct OutlinedCluster {
    /// Position in bytes in the original string.
    ///
    /// We use it to match a cluster with a character in the text chunk and therefore with the style.
    byte_idx: ByteIndex,

    /// Cluster's original codepoint.
    ///
    /// Technically, a cluster can contain multiple codepoints,
    /// but we are storing only the first one.
    codepoint: char,

    /// Cluster's width.
    ///
    /// It's different from advance in that it's not affected by letter spacing and word spacing.
    width: f64,

    /// An advance along the X axis.
    ///
    /// Can be negative.
    advance: f64,

    /// An ascent in SVG coordinates.
    ascent: f64,

    /// A descent in SVG coordinates.
    descent: f64,

    /// A x-height in SVG coordinates.
    x_height: f64,

    /// Indicates that this cluster was affected by the relative shift (via dx/dy attributes)
    /// during the text layouting. Which breaks the `text-decoration` line.
    ///
    /// Used during the `text-decoration` processing.
    has_relative_shift: bool,

    /// An actual outline.
    path: PathData,

    /// A cluster's transform that contains it's position, rotation, etc.
    transform: Transform,

    /// Not all clusters should be rendered.
    ///
    /// For example, if a cluster is outside the text path than it should not be rendered.
    visible: bool,
}

impl OutlinedCluster {
    fn height(&self) -> f64 {
        self.ascent - self.descent
    }
}

/// An iterator over glyph clusters.
///
/// Input:  0 2 2 2 3 4 4 5 5
/// Result: 0 1     4 5   7
struct GlyphClusters<'a> {
    data: &'a [Glyph],
    idx: usize,
}

impl<'a> GlyphClusters<'a> {
    fn new(data: &'a [Glyph]) -> Self {
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

/// Converts a text chunk into a list of outlined clusters.
///
/// This function will do the BIDI reordering, text shaping and glyphs outlining,
/// but not the text layouting. So all clusters are in the 0x0 position.
fn outline_chunk(
    chunk: &TextChunk,
    fonts_cache: &FontsCache,
    fontdb: &fontdb::Database,
) -> Vec<OutlinedCluster> {
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
            clusters.push(outline_cluster(
                &glyphs[range],
                &chunk.text,
                span.font_size.get(),
                fontdb,
            ));
        }
    }

    clusters
}

/// Text shaping with font fallback.
fn shape_text(
    text: &str,
    font: Rc<ResolvedFont>,
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
                Some(v) => Rc::new(v),
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
    font: Rc<ResolvedFont>,
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

/// Outlines a glyph cluster.
///
/// Uses one or more `Glyph`s to construct an `OutlinedCluster`.
fn outline_cluster(
    glyphs: &[Glyph],
    text: &str,
    font_size: f64,
    db: &fontdb::Database,
) -> OutlinedCluster {
    debug_assert!(!glyphs.is_empty());

    let mut path = PathData::new();
    let mut width = 0.0;
    let mut x = 0.0;

    for glyph in glyphs {
        let mut outline = db.outline(glyph.font.id, glyph.id).unwrap_or_default();

        let sx = glyph.font.scale(font_size);

        if !outline.is_empty() {
            // By default, glyphs are upside-down, so we have to mirror them.
            let mut ts = Transform::new_scale(1.0, -1.0);

            // Scale to font-size.
            ts.scale(sx, sx);

            // Apply offset.
            //
            // The first glyph in the cluster will have an offset from 0x0,
            // but the later one will have an offset from the "current position".
            // So we have to keep an advance.
            // TODO: should be done only inside a single text span
            ts.translate(x + glyph.dx as f64, glyph.dy as f64);

            outline.transform(ts);

            path.push_path(&outline);
        }

        x += glyph.width as f64;

        let glyph_width = glyph.width as f64 * sx;
        if glyph_width > width {
            width = glyph_width;
        }
    }

    let byte_idx = glyphs[0].byte_idx;
    let font = glyphs[0].font.clone();
    OutlinedCluster {
        byte_idx,
        codepoint: byte_idx.char_from(text),
        width,
        advance: width,
        ascent: font.ascent(font_size),
        descent: font.descent(font_size),
        x_height: font.x_height(font_size),
        has_relative_shift: false,
        path,
        transform: Transform::default(),
        visible: true,
    }
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

        log::warn!("Fallback from {} to {}.", base_face.family, face.family);
        return fontdb.load_font(face.id);
    }

    None
}

/// Resolves clusters positions.
///
/// Mainly sets the `transform` property.
///
/// Returns the last text position. The next text chunk should start from that position.
fn resolve_clusters_positions(
    chunk: &TextChunk,
    char_offset: usize,
    pos_list: &[CharacterPosition],
    rotate_list: &[f64],
    writing_mode: WritingMode,
    ts: Transform,
    fonts_cache: &FontsCache,
    clusters: &mut [OutlinedCluster],
) -> (f64, f64) {
    match chunk.text_flow {
        TextFlow::Linear => resolve_clusters_positions_horizontal(
            chunk,
            char_offset,
            pos_list,
            rotate_list,
            writing_mode,
            clusters,
        ),
        TextFlow::Path(ref path) => resolve_clusters_positions_path(
            chunk,
            char_offset,
            path,
            pos_list,
            rotate_list,
            writing_mode,
            ts,
            fonts_cache,
            clusters,
        ),
    }
}

fn resolve_clusters_positions_horizontal(
    chunk: &TextChunk,
    offset: usize,
    pos_list: &[CharacterPosition],
    rotate_list: &[f64],
    writing_mode: WritingMode,
    clusters: &mut [OutlinedCluster],
) -> (f64, f64) {
    let mut x = process_anchor(chunk.anchor, clusters_length(clusters));
    let mut y = 0.0;

    for cluster in clusters {
        let cp = offset + cluster.byte_idx.code_point_at(&chunk.text);
        if let Some(pos) = pos_list.get(cp) {
            if writing_mode == WritingMode::LeftToRight {
                x += pos.dx.unwrap_or(0.0);
                y += pos.dy.unwrap_or(0.0);
            } else {
                y -= pos.dx.unwrap_or(0.0);
                x += pos.dy.unwrap_or(0.0);
            }
            cluster.has_relative_shift = pos.dx.is_some() || pos.dy.is_some();
        }

        cluster.transform.translate(x, y);

        if let Some(angle) = rotate_list.get(cp).cloned() {
            if !angle.is_fuzzy_zero() {
                cluster.transform.rotate(angle);
                cluster.has_relative_shift = true;
            }
        }

        x += cluster.advance;
    }

    (x, y)
}

fn resolve_clusters_positions_path(
    chunk: &TextChunk,
    char_offset: usize,
    path: &TextPath,
    pos_list: &[CharacterPosition],
    rotate_list: &[f64],
    writing_mode: WritingMode,
    ts: Transform,
    fonts_cache: &FontsCache,
    clusters: &mut [OutlinedCluster],
) -> (f64, f64) {
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

    let normals = collect_normals(
        chunk,
        clusters,
        &path.path,
        pos_list,
        char_offset,
        start_offset,
        ts,
    );
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
        cluster.transform.translate(x - half_width, y);
        cluster.transform.rotate_at(angle, half_width, 0.0);

        let cp = char_offset + cluster.byte_idx.code_point_at(&chunk.text);
        if let Some(pos) = pos_list.get(cp) {
            dy += pos.dy.unwrap_or(0.0);
        }

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
        if !dy.is_fuzzy_zero() || !baseline_shift.is_fuzzy_zero() {
            let shift = kurbo::Vec2::new(0.0, dy - baseline_shift);
            cluster.transform.translate(shift.x, shift.y);
        }

        if let Some(angle) = rotate_list.get(cp).cloned() {
            if !angle.is_fuzzy_zero() {
                cluster.transform.rotate(angle);
            }
        }

        // The possible `lengthAdjust` transform should be applied after text-on-path positioning.
        cluster.transform.append(&orig_ts);

        last_x = x + cluster.advance;
        last_y = y;
    }

    (last_x, last_y)
}

fn clusters_length(clusters: &[OutlinedCluster]) -> f64 {
    clusters.iter().fold(0.0, |w, cluster| w + cluster.advance)
}

fn process_anchor(a: TextAnchor, text_width: f64) -> f64 {
    match a {
        TextAnchor::Start => 0.0, // Nothing.
        TextAnchor::Middle => -text_width / 2.0,
        TextAnchor::End => -text_width,
    }
}

struct PathNormal {
    x: f64,
    y: f64,
    angle: f64,
}

fn collect_normals(
    chunk: &TextChunk,
    clusters: &[OutlinedCluster],
    path: &PathData,
    pos_list: &[CharacterPosition],
    char_offset: usize,
    offset: f64,
    ts: Transform,
) -> Vec<Option<PathNormal>> {
    debug_assert!(!path.is_empty());

    let mut offsets = Vec::with_capacity(clusters.len());
    let mut normals = Vec::with_capacity(clusters.len());
    {
        let mut advance = offset;
        for cluster in clusters {
            // Clusters should be rotated by the x-midpoint x baseline position.
            let half_width = cluster.width / 2.0;

            // Include relative position.
            let cp = char_offset + cluster.byte_idx.code_point_at(&chunk.text);
            if let Some(pos) = pos_list.get(cp) {
                advance += pos.dx.unwrap_or(0.0);
            }

            let offset = advance + half_width;

            // Clusters outside the path have no normals.
            if offset < 0.0 {
                normals.push(None);
            }

            offsets.push(offset);
            advance += cluster.advance;
        }
    }

    let mut prev_mx = path.points()[0];
    let mut prev_my = path.points()[1];
    let mut prev_x = prev_mx;
    let mut prev_y = prev_my;

    fn create_curve_from_line(px: f64, py: f64, x: f64, y: f64) -> kurbo::CubicBez {
        let line = kurbo::Line::new(kurbo::Point::new(px, py), kurbo::Point::new(x, y));
        let p1 = line.eval(0.33);
        let p2 = line.eval(0.66);
        cubic_from_points(px, py, p1.x, p1.y, p2.x, p2.y, x, y)
    }

    let mut length = 0.0;
    for seg in path.segments() {
        let curve = match seg {
            PathSegment::MoveTo { x, y } => {
                prev_mx = x;
                prev_my = y;
                prev_x = x;
                prev_y = y;
                continue;
            }
            PathSegment::LineTo { x, y } => create_curve_from_line(prev_x, prev_y, x, y),
            PathSegment::CurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => cubic_from_points(prev_x, prev_y, x1, y1, x2, y2, x, y),
            PathSegment::ClosePath => create_curve_from_line(prev_x, prev_y, prev_mx, prev_my),
        };

        let arclen_accuracy = {
            let base_arclen_accuracy = 0.5;
            // Accuracy depends on a current scale.
            // When we have a tiny path scaled by a large value,
            // we have to increase out accuracy accordingly.
            let (sx, sy) = ts.get_scale();
            // 1.0 acts as a threshold to prevent division by 0 and/or low accuracy.
            base_arclen_accuracy / (sx * sy).sqrt().max(1.0)
        };

        let curve_len = curve.arclen(arclen_accuracy);

        for offset in &offsets[normals.len()..] {
            if *offset >= length && *offset <= length + curve_len {
                let mut offset = curve.inv_arclen(offset - length, arclen_accuracy);
                // some rounding error may occur, so we give offset a little tolerance
                debug_assert!((-1.0e-3..=1.0 + 1.0e-3).contains(&offset));
                offset = offset.min(1.0).max(0.0);

                let pos = curve.eval(offset);
                let d = curve.deriv().eval(offset);
                let d = kurbo::Vec2::new(-d.y, d.x); // tangent
                let angle = d.atan2().to_degrees() - 90.0;

                normals.push(Some(PathNormal {
                    x: pos.x,
                    y: pos.y,
                    angle,
                }));

                if normals.len() == offsets.len() {
                    break;
                }
            }
        }

        length += curve_len;
        prev_x = curve.p3.x;
        prev_y = curve.p3.y;
    }

    // If path ended and we still have unresolved normals - set them to `None`.
    for _ in 0..(offsets.len() - normals.len()) {
        normals.push(None);
    }

    normals
}

/// Applies the `letter-spacing` property to a text chunk clusters.
///
/// [In the CSS spec](https://www.w3.org/TR/css-text-3/#letter-spacing-property).
fn apply_letter_spacing(chunk: &TextChunk, clusters: &mut [OutlinedCluster]) {
    // At least one span should have a non-zero spacing.
    if !chunk
        .spans
        .iter()
        .any(|span| !span.letter_spacing.is_fuzzy_zero())
    {
        return;
    }

    let num_clusters = clusters.len();
    for (i, cluster) in clusters.iter_mut().enumerate() {
        // Spacing must be applied only to characters that belongs to the script
        // that supports spacing.
        // We are checking only the first code point, since it should be enough.
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
                    cluster.path.clear();
                }
            }
        }
    }
}

/// Checks that selected script supports letter spacing.
///
/// [In the CSS spec](https://www.w3.org/TR/css-text-3/#cursive-tracking).
///
/// The list itself is from: https://github.com/harfbuzz/harfbuzz/issues/64
fn script_supports_letter_spacing(script: unicode_script::Script) -> bool {
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

/// Applies the `word-spacing` property to a text chunk clusters.
///
/// [In the CSS spec](https://www.w3.org/TR/css-text-3/#propdef-word-spacing).
fn apply_word_spacing(chunk: &TextChunk, clusters: &mut [OutlinedCluster]) {
    // At least one span should have a non-zero spacing.
    if !chunk
        .spans
        .iter()
        .any(|span| !span.word_spacing.is_fuzzy_zero())
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

/// Checks that the selected character is a word separator.
///
/// According to: https://www.w3.org/TR/css-text-3/#word-separator
fn is_word_separator_characters(c: char) -> bool {
    matches!(
        c as u32,
        0x0020 | 0x00A0 | 0x1361 | 0x010100 | 0x010101 | 0x01039F | 0x01091F
    )
}

fn apply_length_adjust(chunk: &TextChunk, clusters: &mut [OutlinedCluster]) {
    let is_horizontal = matches!(chunk.text_flow, TextFlow::Linear);

    for span in &chunk.spans {
        let target_width = if let Some(w) = span.text_length {
            w
        } else {
            continue;
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
            let factor = (target_width - width) / (cluster_indexes.len() - 1) as f64;
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
                clusters[i].transform.scale(factor, 1.0);

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
fn apply_writing_mode(writing_mode: WritingMode, clusters: &mut [OutlinedCluster]) {
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
            ts.translate(cluster.width / 2.0, 0.0);
            ts.rotate(-90.0);
            ts.translate(-cluster.width / 2.0, -dy);
            cluster.path.transform(ts);

            // Move "baseline" to the middle and make height equal to width.
            cluster.ascent = cluster.width / 2.0;
            cluster.descent = -cluster.width / 2.0;
        } else {
            // Could not find a spec that explains this,
            // but this is how other applications are shifting the "rotated" characters
            // in the top-to-bottom mode.
            cluster.transform.translate(0.0, cluster.x_height / 2.0);
        }
    }
}

fn resolve_baseline_shift(baselines: &[BaselineShift], font: &ResolvedFont, font_size: f64) -> f64 {
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

fn cubic_from_points(
    px: f64,
    py: f64,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    x: f64,
    y: f64,
) -> kurbo::CubicBez {
    kurbo::CubicBez {
        p0: kurbo::Point::new(px, py),
        p1: kurbo::Point::new(x1, y1),
        p2: kurbo::Point::new(x2, y2),
        p3: kurbo::Point::new(x, y),
    }
}
