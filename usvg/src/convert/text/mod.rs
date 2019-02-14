// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::mem;

// external
use svgdom;
use harfbuzz;
use unicode_bidi;
//use unicode_script;

mod fk {
    pub use font_kit::hinting::HintingOptions;
    pub use font_kit::font::Font;
}

// self
mod convert;
use tree;
use tree::prelude::*;
use super::prelude::*;
use self::convert::*;

// TODO: letter spacing
// TODO: word spacing
// TODO: visibility on text and tspan
// TODO: glyph fallback
// TODO: group when Options::keep_named_groups is set


/// A raw glyph.
///
/// Basically, a glyph ID in the font and it's metrics.
#[derive(Clone, Copy)]
struct RawGlyph {
    /// The glyph ID in the font.
    id: u32,

    /// Position in bytes in the original string.
    byte_idx: usize,

    dx: f64,

    dy: f64,

    width: f64,
}

#[derive(Clone)]
struct OutlinedGlyph {
    /// Position in bytes in the original string.
    byte_idx: usize,

    x: f64,

    y: f64,

    rotate: f64,

    /// The glyph width.
    ///
    /// It's not the outline width, but the width specified in the font.
    /// So `width` != `path` bbox width.
    width: f64,

    /// Indicates that this glyph was affected by the relative shift (via dx/dy attributes)
    /// during the text layouting.
    ///
    /// Used during the `text-decoration` processing.
    has_relative_shift: bool,

    /// The actual outline.
    path: Vec<tree::PathSegment>,
}

impl Default for OutlinedGlyph {
    fn default() -> Self {
        OutlinedGlyph {
            byte_idx: 0,
            x: 0.0,
            y: 0.0,
            rotate: 0.0,
            width: 0.0,
            has_relative_shift: false,
            path: Vec::new(),
        }
    }
}

/// A text decoration span.
///
/// Basically a horizontal line, that will be used for underline, overline and line-through.
/// It doesn't have a height, since it depends on the font metrics.
struct DecorationSpan {
    x: f64,
    baseline: f64,
    width: f64,
    angle: f64,
}


pub fn convert(
    text_elem: &svgdom::Node,
    mut parent: tree::Node,
    tree: &mut tree::Tree,
) {
    let pos_list = resolve_positions_list(text_elem);
    let rotate_list = resolve_rotate(text_elem);

    let text_ts = text_elem.attributes().get_transform(AId::Transform).unwrap_or_default();

    let mut chunks = collect_text_chunks(tree, &text_elem, &pos_list);
    let mut char_offset = 0;
    let mut x = 0.0;
    let mut baseline = 0.0;
    for chunk in &mut chunks {
        x = chunk.x.unwrap_or(x);
        baseline = chunk.y.unwrap_or(baseline);

        let mut glyphs = render_chunk(&chunk);
        resolve_glyph_positions(&chunk.text, char_offset, &pos_list, &rotate_list, &mut glyphs);

        let width = glyphs.iter().fold(0.0, |w, glyph| w + glyph.width);

        x -= process_anchor(chunk.anchor, width);

        for span in &mut chunk.spans {
            let decoration_spans = collect_decoration_spans(span, &glyphs);

            if let Some(decoration) = span.decoration.underline.take() {
                parent.append_kind(convert_decoration(
                    x, baseline - span.font.underline_position,
                    &span, decoration, &decoration_spans, text_ts,
                ));
            }

            if let Some(decoration) = span.decoration.overline.take() {
                // TODO: overline pos from font
                parent.append_kind(convert_decoration(
                    x, baseline - span.font.ascent,
                    &span, decoration, &decoration_spans, text_ts,
                ));
            }

            if let Some(path) = convert_span(x, baseline, span, &mut glyphs, &text_ts) {
                parent.append_kind(path);
            }

            if let Some(decoration) = span.decoration.line_through.take() {
                // TODO: line-through pos from font
                parent.append_kind(convert_decoration(
                    x, baseline - span.font.ascent / 3.0,
                    &span, decoration, &decoration_spans, text_ts,
                ));
            }
        }

        char_offset += chunk.text.chars().count();
        x += width;
    }
}

fn convert_span(
    x: f64,
    baseline: f64,
    span: &mut TextSpan,
    glyphs: &mut [OutlinedGlyph],
    text_ts: &tree::Transform,
) -> Option<tree::NodeKind> {
    let mut segments = Vec::new();

    for glyph in glyphs {
        if span.contains(glyph.byte_idx) {
            let mut path = mem::replace(&mut glyph.path, Vec::new());
            let mut transform = tree::Transform::new_translate(glyph.x, glyph.y);
            if !glyph.rotate.is_fuzzy_zero() {
                transform.rotate(glyph.rotate);
            }

            transform_path(&mut path, &transform);

            if !path.is_empty() {
                segments.extend_from_slice(&path);
            }
        }
    }

    if segments.is_empty() {
        return None;
    }

    let mut transform = text_ts.clone();
    transform.translate(x, baseline - span.baseline_shift);

    // TODO: fill and stroke with a gradient/pattern that has objectBoundingBox
    //       should use the text element bbox and not the tspan bbox.

    let mut path = tree::Path {
        id: String::new(),
        transform,
        visibility: span.visibility,
        fill: span.fill.take(),
        stroke: span.stroke.take(),
        segments,
    };

    mem::swap(&mut path.id, &mut span.id);

    Some(tree::NodeKind::Path(path))
}

fn font_eq(f1: &Font, f2: &Font) -> bool {
       f1.path  == f2.path
    && f1.index == f2.index
    && f1.size  == f2.size
}

/// Converts a text chunk into a list of outlined glyphs.
///
/// This function will do the BIDI reordering, text shaping and glyphs outlining,
/// but not the text layouting. So all glyphs are in 0x0 position.
fn render_chunk(
    chunk: &TextChunk,
) -> Vec<OutlinedGlyph> {
    // Shape the text using all fonts in the chunk.
    //
    // I'm not sure if this can be optimized, but it's probably pretty expensive.
    let fonts = collect_unique_fonts(chunk);
    if fonts.is_empty() {
        return Vec::new();
    }

    let mut list = Vec::new();
    for font in &fonts {
        let raw_glyphs = shape_text(&chunk.text, font);
        list.push((font, raw_glyphs));
    }

    // TODO: is this even possible?
    // Check that all glyphs lists have the same length.
    if !list.iter().all(|v| v.1.len() == list[0].1.len()) {
        warn!("Text layouting failed.");
        return Vec::new();
    }

    // TODO: We assume, that all glyphs lists have the same clusters,
    //       so if a char was converted into one glyph in one font,
    //       it will be true for all fonts
    //       And I'm not sure of that.
    let mut glyphs = Vec::new();
    for (i, raw_glyph) in list[0].1.iter().enumerate() {
        for span in &chunk.spans {
            if span.contains(raw_glyph.byte_idx) {
                for (font, raw_glyphs) in &list {
                    if font_eq(&span.font, font) {
                        // TODO: outline cluster
                        glyphs.push(outline_glyph(font, raw_glyphs[i]));
                        break;
                    }
                }

                break;
            }
        }
    }

    glyphs
}

/// Returns a list of unique fonts in the specified chunk.
fn collect_unique_fonts(
    chunk: &TextChunk,
) -> Vec<Font> {
    let mut list = Vec::new();

    for span in &chunk.spans {
        let mut exists = false;
        for font in &list {
            if font_eq(font, &span.font) {
                exists = true;
                break;
            }
        }

        if !exists {
            list.push(span.font.clone());
        }
    }

    list
}

/// Converts a text into a list of glyph IDs.
///
/// This function will do the BIDI reordering and text shaping.
fn shape_text(
    text: &str,
    font: &Font,
) -> Vec<RawGlyph> {
    let font_data = try_opt!(font.font.copy_font_data(), Vec::new());
    let hb_face = harfbuzz::Face::from_bytes(&font_data, font.index);
    let hb_font = harfbuzz::Font::new(hb_face);

    let bidi_info = unicode_bidi::BidiInfo::new(text, Some(unicode_bidi::Level::ltr()));
    let paragraph = &bidi_info.paragraphs[0];
    let line = paragraph.range.clone();

    let mut glyphs = Vec::new();

    let (levels, runs) = bidi_info.visual_runs(&paragraph, line);
    for run in runs.iter() {
        let sub_text = &text[run.clone()];
        if sub_text.is_empty() {
            continue;
        }

        let hb_direction = if levels[run.start].is_rtl() {
            harfbuzz::Direction::Rtl
        } else {
            harfbuzz::Direction::Ltr
        };

        let buffer = harfbuzz::UnicodeBuffer::new()
            .add_str(sub_text)
            .set_direction(hb_direction);

        // TODO: feature smcp / small cups
        let output = harfbuzz::shape(&hb_font, buffer, &[]);

        let positions = output.get_glyph_positions();
        let infos = output.get_glyph_infos();

        // TODO: merge clusters?
        for (pos, info) in positions.iter().zip(infos) {
            glyphs.push(RawGlyph {
                byte_idx: run.start + info.cluster as usize,
                id: info.codepoint,
                dx: pos.x_offset as f64,
                dy: pos.y_offset as f64,
                width: pos.x_advance as f64,
            });
        }
    }

    glyphs
}

fn outline_glyph(
    font: &Font,
    raw_glyph: RawGlyph,
) -> OutlinedGlyph {
    use lyon_path::builder::FlatPathBuilder;

    let scale = font.size / font.units_per_em as f64;

    let mut builder = svgdom_path_builder::Builder::new();
    let mut glyph = match font.font.outline(raw_glyph.id, fk::HintingOptions::None, &mut builder) {
        Ok(_) => {
            super::path::convert_path(builder.build())
        }
        Err(_) => {
            warn!("Glyph {} not found in the font.", raw_glyph.id);
            Vec::new()
        }
    };

    if !glyph.is_empty() {
        // Mirror and scale to the `font-size`.
        let mut ts = svgdom::Transform::new_scale(scale, -scale);
        // Apply offset.
        ts.translate(raw_glyph.dx, raw_glyph.dy);

        transform_path(&mut glyph, &ts);
    }

    OutlinedGlyph {
        byte_idx: raw_glyph.byte_idx,
        path: glyph,
        width: raw_glyph.width * scale,
        .. OutlinedGlyph::default()
    }
}

/// Applies the transform to the path segments.
fn transform_path(segments: &mut [tree::PathSegment], ts: &tree::Transform) {
    for seg in segments {
        match seg {
            tree::PathSegment::MoveTo { x, y } => {
                ts.apply_to(x, y);
            }
            tree::PathSegment::LineTo { x, y } => {
                ts.apply_to(x, y);
            }
            tree::PathSegment::CurveTo { x1, y1, x2, y2, x,  y } => {
                ts.apply_to(x1, y1);
                ts.apply_to(x2, y2);
                ts.apply_to(x, y);
            }
            tree::PathSegment::ClosePath => {}
        }
    }
}

fn resolve_glyph_positions(
    text: &str,
    offset: usize,
    pos_list: &PositionsList,
    rotate_list: &RotateList,
    glyphs: &mut Vec<OutlinedGlyph>,
) {
    let mut x = 0.0;
    let mut y = 0.0;

    for glyph in glyphs {
        glyph.x = x;
        glyph.y = y;

        let cp = offset + byte_to_code_point(text, glyph.byte_idx);
        if let Some(pos) = pos_list.get(cp) {
            glyph.x += pos.dx.unwrap_or(0.0);
            glyph.y += pos.dy.unwrap_or(0.0);
            glyph.has_relative_shift = pos.dx.is_some() || pos.dy.is_some();
        }

        if let Some(angle) = rotate_list.get(cp).cloned() {
            glyph.rotate = angle;
        }

        x = glyph.x + glyph.width;
        y = glyph.y;
    }
}

/// Converts byte position into code point position.
fn byte_to_code_point(text: &str, byte: usize) -> usize {
    let mut idx = 0;
    for (i, _) in text.char_indices() {
        if byte == i {
            break;
        }

        idx += 1;
    }

    idx
}

fn process_anchor(a: TextAnchor, text_width: f64) -> f64 {
    match a {
        TextAnchor::Start   => 0.0, // Nothing.
        TextAnchor::Middle  => text_width / 2.0,
        TextAnchor::End     => text_width,
    }
}

fn collect_decoration_spans(
    span: &TextSpan,
    glyphs: &[OutlinedGlyph],
) -> Vec<DecorationSpan> {
    let mut spans = Vec::new();

    let mut started = false;
    let mut x = 0.0;
    let mut y = 0.0;
    let mut width = 0.0;
    let mut angle = 0.0;
    for glyph in glyphs {
        if span.contains(glyph.byte_idx) {
            if started && (glyph.has_relative_shift || !glyph.rotate.is_fuzzy_zero()) {
                started = false;
                spans.push(DecorationSpan { x, baseline: y, width, angle });
            }

            if !started {
                x = glyph.x;
                y = glyph.y;
                width = glyph.x + glyph.width - x;
                angle = glyph.rotate;
                started = true;
            } else {
                width = glyph.x + glyph.width - x;
            }
        } else if started {
            spans.push(DecorationSpan { x, baseline: y, width, angle });
            started = false;
        }
    }

    if started {
        spans.push(DecorationSpan { x, baseline: y, width, angle });
    }

    spans
}

fn convert_decoration(
    x: f64,
    baseline: f64,
    span: &TextSpan,
    mut decoration: TextDecorationStyle,
    decoration_spans: &[DecorationSpan],
    transform: tree::Transform,
) -> tree::NodeKind {
    debug_assert!(!decoration_spans.is_empty());

    let mut segments = Vec::new();
    for dec_span in decoration_spans {
        let tx = x + dec_span.x;
        let ty = baseline + dec_span.baseline - span.baseline_shift
                 - span.font.underline_thickness / 2.0;

        let rect = Rect::new(
            0.0,
            0.0,
            dec_span.width,
            span.font.underline_thickness,
        );

        let start_idx = segments.len();
        add_rect_to_path(rect, &mut segments);

        let mut ts = tree::Transform::new_translate(tx, ty);
        ts.rotate(dec_span.angle);
        transform_path(&mut segments[start_idx..], &ts);
    }

    tree::NodeKind::Path(tree::Path {
        id: String::new(),
        transform,
        visibility: span.visibility,
        fill: decoration.fill.take(),
        stroke: decoration.stroke.take(),
        segments,
    })
}

fn add_rect_to_path(rect: Rect, path: &mut Vec<tree::PathSegment>) {
    path.extend_from_slice(&[
        tree::PathSegment::MoveTo {
            x: rect.x, y: rect.y
        },
        tree::PathSegment::LineTo {
            x: rect.right(), y: rect.y
        },
        tree::PathSegment::LineTo {
            x: rect.right(), y: rect.bottom()
        },
        tree::PathSegment::LineTo {
            x: rect.x, y: rect.bottom()
        },
        tree::PathSegment::ClosePath,
    ]);
}

//fn script_supports_letter_spacing(script: unicode_script::Script) -> bool {
//    // Details https://www.w3.org/TR/css-text-3/#cursive-tracking
//    //
//    // List from https://github.com/harfbuzz/harfbuzz/issues/64
//
//    use unicode_script::Script;
//
//    match script {
//          Script::Arabic
//        | Script::Syriac
//        | Script::Nko
//        | Script::Manichaean
//        | Script::Psalter_Pahlavi
//        | Script::Mandaic
//        | Script::Mongolian
//        | Script::Phags_Pa
//        | Script::Devanagari
//        | Script::Bengali
//        | Script::Gurmukhi
//        | Script::Modi
//        | Script::Sharada
//        | Script::Syloti_Nagri
//        | Script::Tirhuta
//        | Script::Ogham => false,
//        _ => true,
//    }
//}

/// Implements an ability to outline a glyph directly into the `svgdom::Path`.
mod svgdom_path_builder {
    use lyon_geom::math::*;
    use lyon_path::builder::{FlatPathBuilder, PathBuilder};

    pub struct Builder {
        path: svgdom::Path,
        current_position: Point,
        first_position: Point,
    }

    impl Builder {
        pub fn new() -> Self {
            Builder {
                path: svgdom::Path::new(),
                current_position: Point::new(0.0, 0.0),
                first_position: Point::new(0.0, 0.0),
            }
        }
    }

    impl FlatPathBuilder for Builder {
        type PathType = svgdom::Path;

        fn move_to(&mut self, to: Point) {
            self.first_position = to;
            self.current_position = to;
            self.path.push(svgdom::PathSegment::MoveTo { abs: true, x: to.x as f64, y: to.y as f64 });
        }

        fn line_to(&mut self, to: Point) {
            self.current_position = to;
            self.path.push(svgdom::PathSegment::LineTo { abs: true, x: to.x as f64, y: to.y as f64 });
        }

        fn close(&mut self) {
            self.current_position = self.first_position;
            self.path.push(svgdom::PathSegment::ClosePath { abs: true });
        }

        fn build(self) -> Self::PathType {
            self.path
        }

        fn build_and_reset(&mut self) -> Self::PathType {
            let p = self.path.clone();
            self.path.clear();
            self.current_position = Point::new(0.0, 0.0);
            self.first_position = Point::new(0.0, 0.0);
            p
        }

        fn current_position(&self) -> Point {
            self.current_position
        }
    }

    impl PathBuilder for Builder {
        fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) {
            self.current_position = to;
            self.path.push(svgdom::PathSegment::Quadratic {
                abs: true,
                x1: ctrl.x as f64,
                y1: ctrl.y as f64,
                x: to.x as f64,
                y: to.y as f64,
            });
        }

        fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) {
            self.current_position = to;
            self.path.push(svgdom::PathSegment::CurveTo {
                abs: true,
                x1: ctrl1.x as f64,
                y1: ctrl1.y as f64,
                x2: ctrl2.x as f64,
                y2: ctrl2.y as f64,
                x: to.x as f64,
                y: to.y as f64,
            });
        }

        fn arc(&mut self, center: Point, radii: Vector, sweep_angle: Angle, x_rotation: Angle) {
            let arc = lyon_geom::arc::Arc {
                start_angle: (self.current_position() - center).angle_from_x_axis() - x_rotation,
                center, radii, sweep_angle, x_rotation,
            };
            let arc = arc.to_svg_arc();

            self.path.push(svgdom::PathSegment::EllipticalArc {
                abs: true,
                rx: arc.radii.x as f64,
                ry: arc.radii.y as f64,
                x_axis_rotation: arc.x_rotation.to_degrees() as f64,
                large_arc: arc.flags.large_arc,
                sweep: arc.flags.sweep,
                x: arc.to.x as f64,
                y: arc.to.y as f64,
            });
        }
    }
}
