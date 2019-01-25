// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::mem;

// external
use svgdom;
use harfbuzz;
use unicode_bidi;
use unicode_script;

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

// TODO: word spacing
// TODO: visibility on text and tspan
// TODO: glyph fallback
// TODO: group when Options::keep_named_groups is set


#[derive(Clone)]
struct Glyph {
    byte_idx: usize,
    x: f64,
    y: f64,
    rotate: f64,
    width: f64,
    has_relative_shift: bool, // via dx/dy
    path: Vec<tree::PathSegment>,
}

#[derive(Clone)]
struct TextChunk {
    x: Option<f64>,
    y: Option<f64>,
    anchor: TextAnchor,
    spans: Vec<TextSpan>,
    text: String,
}

#[derive(Clone)]
struct TextSpan {
    start: usize,
    end: usize,
    id: String,
    fill: Option<tree::Fill>,
    stroke: Option<tree::Stroke>,
    font: Font,
    decoration: TextDecoration,
    baseline_shift: f64,
    visibility: tree::Visibility,
}

impl TextSpan {
    fn contains(&self, byte_offset: usize) -> bool {
        byte_offset >= self.start && byte_offset < self.end
    }
}

struct DecorationSpan {
    x: f64,
    y: f64,
    width: f64,
    angle: f64,
}


pub fn convert(
    text_elem: &svgdom::Node,
    mut parent: tree::Node,
    tree: &mut tree::Tree,
) {
    let pos_list = resolve_positions_list(text_elem);

    let mut rotate_list = RotateList::new();
    resolve_rotate(&text_elem, 0, &mut rotate_list);

    debug_assert_eq!(pos_list.len(), rotate_list.len());

    let text_ts = text_elem.attributes().get_transform(AId::Transform).unwrap_or_default();

    let mut chunks = resolve_chunks(tree, &text_elem, &pos_list);
    let mut glyphs = Vec::new();
    let mut char_offset = 0;
    let mut x = 0.0;
    let mut y = 0.0;
    for chunk in &mut chunks {
        x = chunk.x.unwrap_or(x);
        y = chunk.y.unwrap_or(y);

        glyphs.clear();
        // TODO: mixed fonts
        render_chunk(&chunk.text, &chunk.spans[0].font, &mut glyphs);
        resolve_glyph_positions(&chunk.text, char_offset, &pos_list, &rotate_list, &mut glyphs);

        let width = glyphs.iter().fold(0.0, |w, glyph| w + glyph.width);

        x -= process_anchor(chunk.anchor, width);

        for span in &mut chunk.spans {
            let decoration_spans = collect_decoration_spans(span, &glyphs);

            if let Some(decoration) = span.decoration.underline.take() {
                parent.append_kind(convert_decoration(
                    x, y - span.font.underline_position,
                    &span, &decoration, &decoration_spans, text_ts.clone(),
                ));
            }

            if let Some(decoration) = span.decoration.overline.take() {
                // TODO: overline pos from font
                parent.append_kind(convert_decoration(
                    x, y - span.font.ascent,
                    &span, &decoration, &decoration_spans, text_ts.clone(),
                ));
            }

            if let Some(path) = convert_span(x, y, span, &mut glyphs, &text_ts) {
                parent.append_kind(path);
            }

            if let Some(decoration) = span.decoration.line_through.take() {
                // TODO: line-through pos from font
                parent.append_kind(convert_decoration(
                    x, y - span.font.ascent / 2.0,
                    &span, &decoration, &decoration_spans, text_ts.clone(),
                ));
            }
        }

        char_offset += chunk.text.chars().count();
        x += width;
    }
}

fn convert_span(
    x: f64,
    y: f64,
    span: &mut TextSpan,
    glyphs: &mut [Glyph],
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
    transform.translate(x, y - span.baseline_shift);

    let mut path = tree::Path {
        id: String::new(),
        transform,
        visibility: span.visibility,
        fill: None,
        stroke: None,
        marker: Box::new(tree::PathMarker::default()),
        segments,
    };

    mem::swap(&mut path.id, &mut span.id);

    // TODO: fill and stroke with a gradient/pattern that has objectBoundingBox
    //       should use the text element bbox and not the tspan bbox.
    mem::swap(&mut path.fill, &mut span.fill);
    mem::swap(&mut path.stroke, &mut span.stroke);

    Some(tree::NodeKind::Path(path))
}

fn collect_decoration_spans(
    span: &TextSpan,
    glyphs: &[Glyph],
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
                spans.push(DecorationSpan { x, y, width, angle });
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
            spans.push(DecorationSpan { x, y, width, angle });
            started = false;
        }
    }

    if started {
        spans.push(DecorationSpan { x, y, width, angle });
    }

    spans
}

fn convert_decoration(
    x: f64,
    y: f64,
    span: &TextSpan,
    decoration: &TextDecorationStyle,
    decoration_spans: &[DecorationSpan],
    transform: tree::Transform,
) -> tree::NodeKind {
    debug_assert!(!decoration_spans.is_empty());

    let mut segments = Vec::new();
    for dec_span in decoration_spans {
        let tx = x + dec_span.x;
        let ty = y + dec_span.y - span.baseline_shift - span.font.underline_thickness / 2.0;

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
        fill: decoration.fill.clone(),
        stroke: decoration.stroke.clone(),
        marker: Box::new(tree::PathMarker::default()),
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

fn resolve_chunks(
    tree: &tree::Tree,
    text_elem: &svgdom::Node,
    pos_list: &PositionsList,
) -> Vec<TextChunk> {
    let mut chunks = Vec::new();
    let mut char_idx = 0;
    let mut chunk_byte_idx = 0;
    for child in text_elem.descendants().filter(|n| n.is_text()) {
        let text_parent = child.parent().unwrap();
        let attrs = text_parent.attributes();
        let baseline_shift = text_parent.attributes().get_number_or(AId::BaselineShift, 0.0);
        let anchor = resolve_text_anchor(&text_parent);

        let font = match resolve_font(&attrs) {
            Some(v) => v,
            None => {
                // Skip this span.
                char_idx += child.text().chars().count();
                continue;
            }
        };

        let span = TextSpan {
            start: 0,
            end: 0,
            id: text_elem.id().clone(),
            fill: super::fill::convert(tree, &attrs, true),
            stroke: super::stroke::convert(tree, &attrs, true),
            font,
            decoration: resolve_decoration(tree, text_elem, &text_parent),
            visibility: super::convert_visibility(&attrs),
            baseline_shift,
        };

        let mut is_new_span = true;
        for c in child.text().chars() {
            if pos_list[char_idx].x.is_some() || pos_list[char_idx].y.is_some() || chunks.is_empty() {
                let x = pos_list[char_idx].x;
                let y = pos_list[char_idx].y;

                chunk_byte_idx = 0;

                let mut span2 = span.clone();
                span2.start = 0;
                span2.end = c.len_utf8();

                chunks.push(TextChunk {
                    x,
                    y,
                    anchor,
                    spans: vec![span2],
                    text: c.to_string(),
                });
            } else if is_new_span {
                let mut span2 = span.clone();
                span2.start = chunk_byte_idx;
                span2.end = chunk_byte_idx + c.len_utf8();

                if let Some(chunk) = chunks.last_mut() {
                    chunk.text.push(c);
                    chunk.spans.push(span2);
                }
            } else {
                if let Some(chunk) = chunks.last_mut() {
                    chunk.text.push(c);
                    if let Some(span) = chunk.spans.last_mut() {
                        debug_assert_ne!(span.end, 0);
                        span.end += c.len_utf8();
                    }
                }
            }

            is_new_span = false;
            char_idx += 1;
            chunk_byte_idx += c.len_utf8();
        }
    }

    chunks
}

fn render_chunk(
    text: &str,
    font: &Font,
    glyphs: &mut Vec<Glyph>,
) {
    let scale = font.size / font.units_per_em as f64;

    let font_data = try_opt!(font.font.copy_font_data(), ());
    let hb_face = harfbuzz::Face::from_bytes(&font_data, 0);
    let hb_font = harfbuzz::Font::new(hb_face);

    let bidi_info = unicode_bidi::BidiInfo::new(text, Some(unicode_bidi::Level::ltr()));
    let paragraph = &bidi_info.paragraphs[0];
    let line = paragraph.range.clone();

    let (levels, runs) = bidi_info.visual_runs(&paragraph, line);
    for run in runs.iter() {
        let sub_text = &text[run.clone()];
        if sub_text.is_empty() {
            continue;
        }

        // TODO: do after, like resolve_glyph_positions
        let mut letter_spacing = font.letter_spacing;
        // TODO: rewrite
        let text_script = unicode_script::get_script(sub_text.chars().nth(0).unwrap());
        if !script_supports_letter_spacing(text_script) {
            letter_spacing = 0.0;
        }

        let is_rtl = levels[run.start].is_rtl();
        let hb_direction = if is_rtl {
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

        for (pos, info) in positions.iter().zip(infos) {
            let mut glyph = outline_glyph(&font.font, info.codepoint);

            // Mirror and scale to the `font-size`.
            if !glyph.is_empty() {
                let ts = svgdom::Transform::new_scale(scale, -scale);
                transform_path(&mut glyph, &ts);
            }

            let offset = Point::new(pos.x_offset as f64 * scale, pos.y_offset as f64 * scale);
            let advance = Size::new(pos.x_advance as f64 * scale + letter_spacing,
                                    pos.y_advance as f64 * scale);

            // TODO: word-spacing via UnicodeSegmentation::unicode_words

            // TODO: to glyph_offset?
            if !glyph.is_empty() && (!offset.x.is_fuzzy_zero() || !offset.y.is_fuzzy_zero()) {
                let ts = svgdom::Transform::new_translate(offset.x, -offset.y);
                transform_path(&mut glyph, &ts);
            }

            glyphs.push(Glyph {
                byte_idx: run.start + info.cluster as usize,
                x: 0.0,
                y: 0.0,
                rotate: 0.0,
                path: glyph,
                width: advance.width,
                has_relative_shift: false,
            });
        }

        if let Some(ref mut glyph) = glyphs.last_mut() {
            glyph.width -= letter_spacing;
        }
    }
}

// TODO: too many allocations
fn outline_glyph(
    font: &fk::Font,
    glyph_id: u32,
) -> Vec<tree::PathSegment> {
    use lyon_path::builder::FlatPathBuilder;
    use lyon_path::PathEvent;
    use svgdom::PathSegment;

    let mut path = lyon_path::default::Path::builder();

    if let Err(_) = font.outline(glyph_id, fk::HintingOptions::None, &mut path) {
        warn!("Glyph {} not found in the font.", glyph_id);
        return Vec::new();
    }

    let path = path.build();

    let mut segments = Vec::new();

    for event in &path {
        let seg = match event {
            PathEvent::MoveTo(p) => {
                PathSegment::MoveTo { abs: true, x: p.x as f64, y: p.y as f64 }
            }
            PathEvent::LineTo(p) => {
                PathSegment::LineTo { abs: true, x: p.x as f64, y: p.y as f64 }
            }
            PathEvent::QuadraticTo(p1, p) => {
                PathSegment::Quadratic {
                    abs: true,
                    x1: p1.x as f64, y1: p1.y as f64,
                    x:  p.x as f64,  y:  p.y as f64,
                }
            }
            PathEvent::CubicTo(p1, p2, p) => {
                PathSegment::CurveTo {
                    abs: true,
                    x1: p1.x as f64, y1: p1.y as f64,
                    x2: p2.x as f64, y2: p2.y as f64,
                    x:  p.x as f64,  y:  p.y as f64,
                }
            }
            PathEvent::Arc(..) => {
                // TODO: this
                warn!("Arc in glyphs is not supported.");
                continue;
            }
            PathEvent::Close => {
                PathSegment::ClosePath { abs: true }
            }
        };

        segments.push(seg);
    }

    super::path::convert_path(svgdom::Path(segments))
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
    glyphs: &mut Vec<Glyph>,
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

        if let Some(angle) = rotate_list.get(cp) {
            glyph.rotate = *angle;
        }

        x = glyph.x + glyph.width;
        y = glyph.y;
    }
}

fn byte_to_code_point(text: &str, byte: usize) -> usize {
    let mut idx = 0;
    for (i, c) in text.char_indices() {
        if byte >= i && byte < i + c.len_utf8() {
            break;
        }

        idx += 1;
    }

    idx
}

fn process_anchor(a: TextAnchor, text_width: f64) -> f64 {
    match a {
        TextAnchor::Start =>  0.0, // Nothing.
        TextAnchor::Middle => text_width / 2.0,
        TextAnchor::End =>    text_width,
    }
}

fn script_supports_letter_spacing(script: unicode_script::Script) -> bool {
    // Details https://www.w3.org/TR/css-text-3/#cursive-tracking
    //
    // List from https://github.com/harfbuzz/harfbuzz/issues/64

    use unicode_script::Script;

    match script {
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
        | Script::Ogham => false,
        _ => true,
    }
}
