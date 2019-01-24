// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::cmp;
use std::mem;

// external
use svgdom;
use harfbuzz;
use unicode_bidi;
use unicode_script;

mod fk {
    pub use font_kit::hinting::HintingOptions;
    pub use font_kit::source::SystemSource;
    pub use font_kit::properties::*;
    pub use font_kit::family_name::FamilyName;
    pub use font_kit::font::Font;
    pub use font_kit::handle::Handle;
}

// self
use tree;
use tree::prelude::*;
use super::prelude::*;

// TODO: decorations
// TODO: word spacing
// TODO: visibility on text and tspan
// TODO: glyph fallback
// TODO: group when Options::keep_named_groups is set


// TODO: remove Debug

#[derive(Clone, Debug)]
struct FontStyle {
    font: fk::Handle,
    size: f64,
    letter_spacing: f64,
    word_spacing: f64,
}

#[derive(Clone, Debug)]
struct Glyph {
    byte_idx: usize,
    x: f64,
    y: f64,
    rotate: f64,
    path: Vec<tree::PathSegment>,
    width: f64,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum TextAnchor {
    Start,
    Middle,
    End,
}

#[derive(Clone, Debug)]
struct TextChunk {
    x: Option<f64>,
    y: Option<f64>,
    anchor: TextAnchor,
    spans: Vec<TextSpan>,
    text: String,
}

#[derive(Clone, Debug)]
struct TextSpan {
    id: String,
    fill: Option<tree::Fill>,
    stroke: Option<tree::Stroke>,
    font: FontStyle,
    baseline_shift: f64,
    visibility: tree::Visibility,
    start: usize,
    end: usize,
}

impl TextSpan {
    fn contains(&self, byte_offset: usize) -> bool {
        byte_offset >= self.start && byte_offset < self.end
    }
}

#[derive(Clone, Copy, Debug)]
struct CharacterPosition {
    x: Option<f64>,
    y: Option<f64>,
    dx: Option<f64>,
    dy: Option<f64>,
}

type PositionsList = Vec<CharacterPosition>;
type RotateList = Vec<f64>;

pub fn convert(
    text_elem: &svgdom::Node,
    mut parent: tree::Node,
    tree: &mut tree::Tree,
) {
    let marker = Box::new(tree::PathMarker {
        start: None,
        mid: None,
        end: None,
        stroke: None,
    });

    let pos_list = resolve_positions_list(text_elem);

    let mut rotate_list = RotateList::new();
    resolve_rotate(&text_elem, 0, &mut rotate_list);

    debug_assert_eq!(pos_list.len(), rotate_list.len());

    let text_ts = text_elem.attributes().get_transform(AId::Transform).unwrap_or_default();

    let mut chunks = process_text_children(&text_elem, tree, &pos_list);
    let mut glyphs = Vec::new();
    let mut char_offset = 0;
    let mut x = 0.0;
    let mut y = 0.0;
    for chunk in &mut chunks {
        x = chunk.x.unwrap_or(x);
        y = chunk.y.unwrap_or(y);

        glyphs.clear();
        // TODO: mixed fonts
        render_text_chunk(&chunk.text, &chunk.spans[0].font, &mut glyphs);
        resolve_glyph_positions(&chunk.text, char_offset, &pos_list, &rotate_list, &mut glyphs);

        let mut width = 0.0;
        for glyph in &glyphs {
            width += glyph.width;
        }

        x -= process_text_anchor(chunk.anchor, width);

        for span in &mut chunk.spans {
            let mut segments = Vec::new();

            for glyph in &mut glyphs {
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
                continue;
            }

            let mut transform = text_ts;
            transform.translate(x, y - span.baseline_shift);

            let mut path = tree::Path {
                id: String::new(),
                transform,
                visibility: span.visibility,
                fill: None,
                stroke: None,
                marker: marker.clone(),
                segments,
            };

            mem::swap(&mut path.id, &mut span.id);

            // TODO: fill and stroke with a gradient/pattern that has objectBoundingBox
            //       should use the text element bbox and not the tspan bbox.
            mem::swap(&mut path.fill, &mut span.fill);
            mem::swap(&mut path.stroke, &mut span.stroke);

            parent.append_kind(tree::NodeKind::Path(path));
        }

        char_offset += chunk.text.chars().count();
        x += width;
    }
}

// According to the https://github.com/w3c/svgwg/issues/537
// 'Assignment of multi-value text layout attributes (x, y, dx, dy, rotate) should be
// according to Unicode code point characters.'
fn resolve_positions_list(text_elem: &svgdom::Node) -> PositionsList {
    let total = count_chars(text_elem);

    let mut list = vec![CharacterPosition {
        x: None,
        y: None,
        dx: None,
        dy: None,
    }; total];

    let mut offset = 0;
    for child in text_elem.descendants() {
        if child.is_element() {
            let total = count_chars(&child);
            let ref attrs = child.attributes();

            macro_rules! push_list {
                ($aid:expr, $field:ident) => {
                    if let Some(num_list) = attrs.get_number_list($aid) {
                        let len = cmp::min(num_list.len(), total);
                        for i in 0..len {
                            list[offset + i].$field = Some(num_list[i]);
                        }
                    }
                };
            }

            push_list!(AId::X, x);
            push_list!(AId::Y, y);
            push_list!(AId::Dx, dx);
            push_list!(AId::Dy, dy);
        } else {
            offset += child.text().chars().count();
        }
    }

    list
}

fn resolve_rotate(parent: &svgdom::Node, mut offset: usize, list: &mut RotateList) {
    for child in parent.children() {
        if child.is_text() {
            let chars_count = child.text().chars().count();
            // TODO: should stop at the root 'text'
            if let Some(p) = child.find_node_with_attribute(AId::Rotate) {
                let attrs = p.attributes();
                if let Some(rotate_list) = attrs.get_number_list(AId::Rotate) {
                    for i in 0..chars_count {
                        let r = match rotate_list.get(i + offset) {
                            Some(r) => *r,
                            None => {
                                // Use the last angle if the index is out of bounds.
                                *rotate_list.last().unwrap_or(&0.0)
                            }
                        };

                        list.push(r);
                    }

                    offset += chars_count;
                }
            } else {
                for _ in 0..chars_count {
                    list.push(0.0);
                }
            }
        } else if child.is_element() {
            // Use parent rotate list if it is not set.
            let sub_offset = if child.has_attribute(AId::Rotate) { 0 } else { offset };
            resolve_rotate(&child, sub_offset, list);

            // TODO: why?
            // 'tspan' represents a single char.
            offset += 1;
        }
    }
}

fn count_chars(node: &svgdom::Node) -> usize {
    let mut total = 0;
    for child in node.descendants().filter(|n| n.is_text()) {
        total += child.text().chars().count();
    }

    total
}

fn process_text_children(
    parent: &svgdom::Node,
    tree: &tree::Tree,
    pos_list: &PositionsList,
) -> Vec<TextChunk> {
    let mut chunks = Vec::new();
    let mut char_idx = 0;
    let mut chunk_byte_idx = 0;
    for child in parent.descendants().filter(|n| n.is_text()) {
        let text_parent = child.parent().unwrap();
        let attrs = text_parent.attributes();
        let baseline_shift = text_parent.attributes().get_number_or(AId::BaselineShift, 0.0);
        let anchor = resolve_text_anchor(&text_parent);

        let font = match convert_font(&attrs) {
            Some(v) => v,
            None => {
                // Skip this span.
                char_idx += child.text().chars().count();
                continue;
            }
        };

        let span = TextSpan {
            id: parent.id().clone(),
            fill: super::fill::convert(tree, &attrs, true),
            stroke: super::stroke::convert(tree, &attrs, true),
            font,
            visibility: super::convert_visibility(&attrs),
            baseline_shift,
            start: 0,
            end: 0,
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

fn render_text_chunk(
    text: &str,
    font_style: &FontStyle,
    glyphs: &mut Vec<Glyph>,
) {
    // TODO: font caching
    let font = match font_style.font.load() {
        Ok(v) => v,
        Err(_) => {
//            warn!("Failed to load font for {:?} font-family.", style.family);
            return;
        }
    };

    let font_metrics = font.metrics();

    let scale = font_style.size / font_metrics.units_per_em as f64;

    let font_data = try_opt!(font.copy_font_data(), ());
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
        let mut letter_spacing = font_style.letter_spacing;
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
            let mut glyph = outline_glyph(&font, info.codepoint);

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

fn resolve_text_anchor(node: &svgdom::Node) -> TextAnchor {
    let attrs = node.attributes();
    match attrs.get_str_or(AId::TextAnchor, "start") {
        "start"  => TextAnchor::Start,
        "middle" => TextAnchor::Middle,
        "end"    => TextAnchor::End,
        _        => TextAnchor::Start,
    }
}

fn process_text_anchor(a: TextAnchor, text_width: f64) -> f64 {
    match a {
        TextAnchor::Start =>  0.0, // Nothing.
        TextAnchor::Middle => text_width / 2.0,
        TextAnchor::End =>    text_width,
    }
}

fn convert_font(
    attrs: &svgdom::Attributes,
) -> Option<FontStyle> {
    let style = attrs.get_str_or(AId::FontStyle, "normal");
    let style = match style {
        "normal"  => fk::Style::Normal,
        "italic"  => fk::Style::Italic,
        "oblique" => fk::Style::Oblique,
        _         => fk::Style::Normal,
    };

    let weight = attrs.get_str_or(AId::FontWeight, "normal");
    let weight = match weight {
        "normal" => fk::Weight::NORMAL,
        "bold"   => fk::Weight::BOLD,
        "100"    => fk::Weight::THIN,
        "200"    => fk::Weight::EXTRA_LIGHT,
        "300"    => fk::Weight::LIGHT,
        "400"    => fk::Weight::NORMAL,
        "500"    => fk::Weight::MEDIUM,
        "600"    => fk::Weight::SEMIBOLD,
        "700"    => fk::Weight::BOLD,
        "800"    => fk::Weight::EXTRA_BOLD,
        "900"    => fk::Weight::BLACK,
        "bolder" | "lighter" => {
            warn!("'bolder' and 'lighter' font-weight must be already resolved.");
            fk::Weight::NORMAL
        }
        _ => fk::Weight::NORMAL,
    };

    let stretch = attrs.get_str_or(AId::FontStretch, "normal");
    let stretch = match stretch {
        "normal"                 => fk::Stretch::NORMAL,
        "ultra-condensed"        => fk::Stretch::ULTRA_CONDENSED,
        "extra-condensed"        => fk::Stretch::EXTRA_CONDENSED,
        "narrower" | "condensed" => fk::Stretch::CONDENSED,
        "semi-condensed"         => fk::Stretch::SEMI_CONDENSED,
        "semi-expanded"          => fk::Stretch::SEMI_EXPANDED,
        "wider" | "expanded"     => fk::Stretch::EXPANDED,
        "extra-expanded"         => fk::Stretch::EXTRA_EXPANDED,
        "ultra-expanded"         => fk::Stretch::ULTRA_EXPANDED,
        _                        => fk::Stretch::NORMAL,
    };

    let mut font_list = Vec::new();
    let font_family = attrs.get_str_or(AId::FontFamily, "");
    for family in font_family.split(',') {
        let family = family.replace('\'', "");

        let name = match family.as_ref() {
            "serif"      => fk::FamilyName::Serif,
            "sans-serif" => fk::FamilyName::SansSerif,
            "monospace"  => fk::FamilyName::Monospace,
            "cursive"    => fk::FamilyName::Cursive,
            "fantasy"    => fk::FamilyName::Fantasy,
            _            => fk::FamilyName::Title(family)
        };

        font_list.push(name);
    }

    let size = attrs.get_number_or(AId::FontSize, 0.0);
    if !(size > 0.0) {
        return None;
    }

    let letter_spacing = attrs.get_number_or(AId::LetterSpacing, 0.0);
    let word_spacing = attrs.get_number_or(AId::WordSpacing, 0.0);

    let properties = fk::Properties { style, weight, stretch };
    let font = match fk::SystemSource::new().select_best_match(&font_list, &properties) {
        Ok(v) => v,
        Err(_) => {
            // TODO: Select any font.
            warn!("No match for {:?} font-family.", font_family);
            return None;
        }
    };

    Some(FontStyle {
        font,
        size,
        letter_spacing,
        word_spacing,
    })
}
