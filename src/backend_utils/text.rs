// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use lyon_geom;
use harfbuzz;
use unicode_bidi;
use unicode_segmentation::UnicodeSegmentation;
use unicode_script;

mod fk {
    pub use font_kit::hinting::HintingOptions;
    pub use font_kit::source::SystemSource;
    pub use font_kit::properties::*;
    pub use font_kit::family_name::FamilyName;
    pub use font_kit::font::Font;
}

// self
use super::prelude::*;


struct Glyph {
    offset: Point,
    advance: Size,
    path: Vec<usvg::PathSegment>, // can be empty in case of whitespace
}

struct TextBlock {
    x: f64,
    baseline: f64,
    width: f64,
    glyphs: Vec<Glyph>,
    visibility: usvg::Visibility,
    fill: Option<usvg::Fill>,
    stroke: Option<usvg::Stroke>,
    decoration: usvg::TextDecoration,
    font_metrics: FontMetrics,
    rotate_angle: Option<f64>,
}

struct FontMetrics {
    ascent: f64,
    underline_position: f64,
    underline_thinkness: f64,
}

pub fn draw_text<Draw>(
    text: &usvg::Text,
    mut draw: Draw,
) -> Rect
    where Draw: FnMut(&[usvg::PathSegment],
                      &Option<usvg::Fill>,
                      &Option<usvg::Stroke>,
                      usvg::Visibility,
                     ) -> Rect
{
    let mut blocks = collect_text_blocks(text);
    let mut bbox = Rect::new_bbox();

    for block in &mut blocks {
        let mut segments = Vec::new();
        let mut x = 0.0;
        let mut y = 0.0;
        for glyph in &mut block.glyphs {
            let mut ts = usvg::Transform::new_translate(x + glyph.offset.x, y + glyph.offset.y);
            utils::transform_path(&mut glyph.path, &ts);

            x += glyph.advance.width;
            y += glyph.advance.height;

            segments.extend_from_slice(&glyph.path);
        }

        let mut block_ts = usvg::Transform::new_translate(block.x, block.baseline);

        if let Some(angle) = block.rotate_angle {
            block_ts.rotate(angle);
        }

        utils::transform_path(&mut segments, &block_ts);

        let line_offset = block.font_metrics.underline_thinkness / 2.0;
        let mut line_rect = Rect::new(0.0, 0.0, block.width, block.font_metrics.underline_thinkness);

        // Draw underline.
        //
        // Should be drawn before/under text.
        if let Some(ref style) = block.decoration.underline {
            line_rect.y = -(block.font_metrics.underline_position - line_offset);

            let mut line = [usvg::PathSegment::ClosePath; 5];
            utils::rect_to_path_slice(line_rect, &mut line);
            utils::transform_path(&mut line, &block_ts);
            draw(&line, &style.fill, &style.stroke, block.visibility);
        }

        // Draw overline.
        //
        // Should be drawn before/under text.
        if let Some(ref style) = block.decoration.overline {
            line_rect.y = -(block.font_metrics.ascent + line_offset);

            let mut line = [usvg::PathSegment::ClosePath; 5];
            utils::rect_to_path_slice(line_rect, &mut line);
            utils::transform_path(&mut line, &block_ts);
            draw(&line, &style.fill, &style.stroke, block.visibility);
        }

        if !segments.is_empty() {
            let text_bbox = draw(&segments, &block.fill, &block.stroke, block.visibility);
            bbox.expand(text_bbox);
        }

        // Draw line-through.
        //
        // Should be drawn after/over text.
        if let Some(ref style) = block.decoration.line_through {
            line_rect.y = -(block.font_metrics.ascent / 2.0 + line_offset);

            let mut line = [usvg::PathSegment::ClosePath; 5];
            utils::rect_to_path_slice(line_rect, &mut line);
            utils::transform_path(&mut line, &block_ts);
            draw(&line, &style.fill, &style.stroke, block.visibility);
        }
    }

    bbox
}

fn collect_text_blocks(text: &usvg::Text) -> Vec<TextBlock> {
    // TODO: rename
    struct TextBlock2 {
        x: f64,
        y: f64,
        start_idx: usize,
        end_idx: usize,
    }

    let mut blocks = Vec::new();
    let mut last_x = 0.0;
    let mut last_y = 0.0;
    for chunk in &text.chunks {
        let mut chunk_x = first_number_or(&chunk.x, last_x);
        let mut x = chunk_x;
        let mut y = first_number_or(&chunk.y, last_y);
        let mut grapheme_idx = 0;
        let mut chunk_w = 0.0;
        let start_idx = blocks.len();

        for tspan in &chunk.spans {
            let mut is_first = true;
            let mut block: Option<TextBlock2> = None;
            let iter = UnicodeSegmentation::grapheme_indices(tspan.text.as_str(), true);
            for (idx, c) in iter {
                let mut has_custom_offset = is_first;

                {
                    let mut number_at = |list: &Option<usvg::NumberList>| -> Option<f64> {
                        if let &Some(ref list) = list {
                            if let Some(n) = list.get(grapheme_idx) {
                                has_custom_offset = true;
                                return Some(*n);
                            }
                        }

                        None
                    };

                    if let Some(n) = number_at(&chunk.x) { x = n; }
                    if let Some(n) = number_at(&chunk.y) { y = n; }
                    if let Some(n) = number_at(&chunk.dx) { x += n; }
                    if let Some(n) = number_at(&chunk.dy) { y += n; }

                    if is_first {
                        if let Some(n) = number_at(&chunk.x) { chunk_x = n; }
                        if let Some(n) = number_at(&chunk.dx) { chunk_x += n; }
                    }
                }

                if text.rotate.is_some() {
                    has_custom_offset = true;
                }

                let can_merge = block.is_some() && !has_custom_offset;
                if can_merge {
                    if let Some(ref mut block) = block {
                        block.end_idx += c.len();
                    }
                } else {
                    if let Some(ref block) = block {
                        let s = &tspan.text.as_str()[block.start_idx..block.end_idx];

                        if let Some(mut text_block) = text_to_path(s, tspan) {
                            let rotate_angle = match text.rotate {
                                Some(ref list) => Some(list[blocks.len()]),
                                None => None,
                            };

                            text_block.x = block.x;
                            text_block.baseline = block.y - tspan.baseline_shift;
                            text_block.rotate_angle = rotate_angle;

                            let w = text_block.width;
                            chunk_w += text_block.width;

                            blocks.push(text_block);

                            if let Some(ref list) = chunk.x {
                                if list.get(grapheme_idx).is_none() {
                                    x += w;
                                }
                            } else {
                                x += w;
                            }
                        }
                    }

                    block = Some(TextBlock2 {
                        x,
                        y,
                        start_idx: idx,
                        end_idx: idx + c.len(),
                    });
                }

                grapheme_idx += 1;
                is_first = false;
            }

            if let Some(ref block) = block {
                let s = &tspan.text.as_str()[block.start_idx..block.end_idx];

                if let Some(mut text_block) = text_to_path(s, tspan) {
                    let rotate_angle = match text.rotate {
                        Some(ref list) => Some(list[blocks.len()]),
                        None => None,
                    };

                    text_block.x = block.x;
                    text_block.baseline = block.y - tspan.baseline_shift;
                    text_block.rotate_angle = rotate_angle;
                    x += text_block.width;
                    chunk_w += text_block.width;

                    blocks.push(text_block);
                }
            }
        }

        let adx = process_text_anchor(chunk.anchor, chunk_w);
        for i in start_idx..blocks.len() {
            blocks[i].x -= adx;
        }

        last_x = chunk_x + chunk_w - adx;
        last_y = y;
    }

    blocks
}

fn text_to_path(
    text: &str,
    span: &usvg::TextSpan,
) -> Option<TextBlock> {
    let font = try_opt!(load_font(&span.font), None);
    let font_metrics = font.metrics();

    let scale = span.font.size.value() / font_metrics.units_per_em as f64;

    let font_data = try_opt!(font.copy_font_data(), None);
    let hb_face = harfbuzz::Face::from_bytes(&font_data, 0);
    let hb_font = harfbuzz::Font::new(hb_face);

    let bidi_info = unicode_bidi::BidiInfo::new(text, Some(unicode_bidi::Level::ltr()));
    let paragraph = &bidi_info.paragraphs[0];
    let line = paragraph.range.clone();

    let mut glyphs = Vec::new();
    let mut width = 0.0;
    let (levels, runs) = bidi_info.visual_runs(&paragraph, line);
    for run in runs.iter() {
        let sub_text = &text[run.clone()];
        if sub_text.is_empty() {
            continue;
        }

        let mut letter_spacing = 0.0;
        if let Some(spacing) = span.font.letter_spacing {
            // TODO: rewrite
            let text_script = unicode_script::get_script(sub_text.chars().nth(0).unwrap());
            if script_supports_letter_spacing(text_script) {
                letter_spacing = spacing;
            }
        }

        let word_spacing = span.font.word_spacing.unwrap_or(0.0);

        let mut buffer = harfbuzz::UnicodeBuffer::new().add_str(sub_text);

        if levels[run.start].is_rtl() {
            buffer = buffer.set_direction(harfbuzz::Direction::Rtl);
        } else {
            buffer = buffer.set_direction(harfbuzz::Direction::Ltr);
        }

        // TODO: smcp
        let output = harfbuzz::shape(&hb_font, buffer, &[]);

        let positions = output.get_glyph_positions();
        let infos = output.get_glyph_infos();

        let mut i = 0;
        for (p, info) in positions.iter().zip(infos) {
            let mut path = outline_glyph(&font, info.codepoint);

            if !path.is_empty() {
                let mut ts = usvg::Transform::default();
                ts.scale(scale, -scale); // Mirror
                utils::transform_path(&mut path, &ts);
            }

            let offset = Point::new(p.x_offset as f64 * scale, p.y_offset as f64 * scale);
            let mut advance = Size::new(p.x_advance as f64 * scale + letter_spacing,
                                        p.y_advance as f64 * scale);

            // TODO: grapheme
            let is_space = sub_text.chars().nth(i).unwrap() == ' ';
            if is_space {
                advance.width += word_spacing;
            }

            glyphs.push(Glyph {
                offset,
                advance,
                path,
            });

            width += advance.width;
            i += 1;
        }

        if let Some(ref mut glyph) = glyphs.last_mut() {
            glyph.advance.width -= letter_spacing;
            width -= letter_spacing;
        }
    }

    let font_metrics2 = FontMetrics {
        ascent: font_metrics.ascent as f64 * scale,
        underline_position: font_metrics.underline_position as f64 * scale,
        underline_thinkness: font_metrics.underline_thickness as f64 * scale,
    };

    Some(TextBlock {
        x: 0.0,
        baseline: 0.0,
        width,
        glyphs,
        visibility: span.visibility,
        fill: span.fill.clone(),
        stroke: span.stroke.clone(),
        decoration: span.decoration.clone(),
        font_metrics: font_metrics2,
        rotate_angle: None,
    })
}

fn first_number_or(list: &Option<usvg::NumberList>, def: f64) -> f64 {
    list.as_ref().map(|list| list[0]).unwrap_or(def)
}

fn load_font(svg_font: &usvg::Font) -> Option<fk::Font> {
    let style = match svg_font.style {
        usvg::FontStyle::Normal  => fk::Style::Normal,
        usvg::FontStyle::Italic  => fk::Style::Italic,
        usvg::FontStyle::Oblique => fk::Style::Oblique,
    };

    let weight = match svg_font.weight {
        usvg::FontWeight::W100  => fk::Weight::THIN,
        usvg::FontWeight::W200  => fk::Weight::EXTRA_LIGHT,
        usvg::FontWeight::W300  => fk::Weight::LIGHT,
        usvg::FontWeight::W400  => fk::Weight::NORMAL,
        usvg::FontWeight::W500  => fk::Weight::MEDIUM,
        usvg::FontWeight::W600  => fk::Weight::SEMIBOLD,
        usvg::FontWeight::W700  => fk::Weight::BOLD,
        usvg::FontWeight::W800  => fk::Weight::EXTRA_BOLD,
        usvg::FontWeight::W900  => fk::Weight::BLACK,
    };

    let stretch = match svg_font.stretch {
        usvg::FontStretch::Normal         => fk::Stretch::NORMAL,
        usvg::FontStretch::Narrower |
        usvg::FontStretch::Condensed      => fk::Stretch::CONDENSED,
        usvg::FontStretch::UltraCondensed => fk::Stretch::ULTRA_CONDENSED,
        usvg::FontStretch::ExtraCondensed => fk::Stretch::EXTRA_CONDENSED,
        usvg::FontStretch::SemiCondensed  => fk::Stretch::SEMI_CONDENSED,
        usvg::FontStretch::SemiExpanded   => fk::Stretch::SEMI_EXPANDED,
        usvg::FontStretch::Wider |
        usvg::FontStretch::Expanded       => fk::Stretch::EXPANDED,
        usvg::FontStretch::ExtraExpanded  => fk::Stretch::EXTRA_EXPANDED,
        usvg::FontStretch::UltraExpanded  => fk::Stretch::ULTRA_EXPANDED,
    };

    let font_properties = fk::Properties { style, weight, stretch };

    let mut font_list = Vec::new();
    for family in svg_font.family.split(',') {
        let family = family.replace('\'', "");

        let name = match family.as_ref() {
            "serif" => fk::FamilyName::Serif,
            "sans-serif" => fk::FamilyName::SansSerif,
            "monospace" => fk::FamilyName::Monospace,
            "cursive" => fk::FamilyName::Cursive,
            "fantasy" => fk::FamilyName::Fantasy,
            _ => fk::FamilyName::Title(family)
        };

        font_list.push(name);
    }

    let font = match fk::SystemSource::new().select_best_match(&font_list, &font_properties) {
        Ok(v) => v,
        Err(_) => {
            warn!("No match for '{}' font-family.", svg_font.family);
            return None;
        }
    };

    let font = match font.load() {
        Ok(v) => v,
        Err(_) => {
            warn!("Failed to load font for '{}' font-family.", svg_font.family);
            return None;
        }
    };

    Some(font)
}

fn process_text_anchor(a: usvg::TextAnchor, text_width: f64) -> f64 {
    match a {
        usvg::TextAnchor::Start =>  0.0, // Nothing.
        usvg::TextAnchor::Middle => text_width / 2.0,
        usvg::TextAnchor::End =>    text_width,
    }
}

fn outline_glyph(
    font: &fk::Font,
    glyph_id: u32,
) -> Vec<usvg::PathSegment> {
    use lyon_path::builder::FlatPathBuilder;
    use lyon_path::PathEvent;

    let mut path = lyon_path::default::Path::builder();

    if let Err(_) = font.outline(glyph_id, fk::HintingOptions::None, &mut path) {
        warn!("Glyph {} not found in the font.", glyph_id);
        return Vec::new();
    }

    let path = path.build();

    // Previous MoveTo coordinate.
    let mut pmx = 0.0;
    let mut pmy = 0.0;

    // Previous coordinate.
    let mut px = 0.0;
    let mut py = 0.0;

    let mut segments = Vec::new();

    for event in &path {
        let seg = match event {
            PathEvent::MoveTo(p) => {
                usvg::PathSegment::MoveTo { x: p.x as f64, y: p.y as f64 }
            }
            PathEvent::LineTo(p) => {
                usvg::PathSegment::LineTo { x: p.x as f64, y: p.y as f64 }
            }
            PathEvent::QuadraticTo(p1, p) => {
                let quad = lyon_geom::QuadraticBezierSegment {
                    from: [px as f32, py as f32].into(),
                    ctrl: [p1.x, p1.y].into(),
                    to:   [p.x, p.y].into(),
                };

                let cubic = quad.to_cubic();

                usvg::PathSegment::CurveTo {
                    x1: cubic.ctrl1.x as f64, y1: cubic.ctrl1.y as f64,
                    x2: cubic.ctrl2.x as f64, y2: cubic.ctrl2.y as f64,
                    x:  cubic.to.x as f64,    y:  cubic.to.y as f64,
                }
            }
            PathEvent::CubicTo(p1, p2, p) => {
                usvg::PathSegment::CurveTo {
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
                usvg::PathSegment::ClosePath
            }
        };

        // Remember last position.
        match seg {
            usvg::PathSegment::MoveTo { x, y } => {
                px = x;
                py = y;
                pmx = x;
                pmy = y;
            }
            usvg::PathSegment::LineTo { x, y } => {
                px = x;
                py = y;
            }
            usvg::PathSegment::CurveTo { x, y, .. } => {
                px = x;
                py = y;
            }
            usvg::PathSegment::ClosePath => {
                // ClosePath moves us to the last MoveTo coordinate,
                // not previous.
                px = pmx;
                py = pmy;
            }
        }

        segments.push(seg);
    }

    segments
}

fn script_supports_letter_spacing(script: unicode_script::Script) -> bool {
    // Details https://www.w3.org/TR/css-text-3/#cursive-tracking
    //
    // List from https://github.com/harfbuzz/harfbuzz/issues/64

    use unicode_script::Script;

    match script {
        Script::Arabic |
        Script::Syriac |
        Script::Nko |
        Script::Manichaean |
        Script::Psalter_Pahlavi |
        Script::Mandaic |
        Script::Mongolian |
        Script::Phags_Pa |
        Script::Devanagari |
        Script::Bengali |
        Script::Gurmukhi |
        Script::Modi |
        Script::Sharada |
        Script::Syloti_Nagri |
        Script::Tirhuta |
        Script::Ogham => false,
        _ => true,
    }
}
