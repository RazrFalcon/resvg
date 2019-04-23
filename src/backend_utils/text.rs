// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// self
use super::prelude::*;


pub struct TextBlock<Font> {
    pub text: String,
    pub is_visible: bool,
    pub bbox: Rect,
    pub rotate: Option<f64>,
    pub fill: Option<usvg::Fill>,
    pub stroke: Option<usvg::Stroke>,
    pub font: Font,
    pub font_ascent: f64,
    pub letter_spacing: Option<f64>,
    pub word_spacing: Option<f64>,
    pub decoration: usvg::TextDecoration,
}

pub trait FontMetrics<Font> {
    fn set_font(&mut self, font: &usvg::Font);
    fn font(&self) -> Font;
    fn width(&self, text: &str) -> f64;
    fn ascent(&self, text: &str) -> f64;
    fn height(&self) -> f64;
}

pub fn draw_blocks<Font, Draw>(
    blocks: Vec<TextBlock<Font>>,
    mut draw: Draw,
)
    where Draw: FnMut(&TextBlock<Font>)
{
    for block in blocks {
        if block.is_visible {
            draw(&block);
        }
    }
}

pub fn prepare_blocks<Font>(
    text_kind: &usvg::Text,
    font_metrics: &mut FontMetrics<Font>,
) -> (Vec<TextBlock<Font>>, Rect) {
    let mut buf_str = String::with_capacity(4);
    let mut blocks: Vec<TextBlock<Font>> = Vec::new();
    let mut last_x = 0.0;
    let mut last_y = 0.0;
    let mut char_idx = 0;
    for chunk in &text_kind.chunks {
        let chunk_x = chunk.x.unwrap_or(last_x) + chunk.dx.unwrap_or(0.0);
        let mut x = chunk_x;
        let y = chunk.y.unwrap_or(last_y) + chunk.dy.unwrap_or(0.0);
        let start_idx = blocks.len();

        for tspan in &chunk.spans {
            font_metrics.set_font(&tspan.font);

            for (i, c) in tspan.text.chars().enumerate() {
                let mut has_custom_offset = i == 0;

                if text_kind.rotate.is_some() {
                    has_custom_offset = true;
                }

                let can_merge = !blocks.is_empty() && !has_custom_offset;
                if can_merge {
                    let prev_idx = blocks.len() - 1;
                    blocks[prev_idx].text.push(c);

                    let mut width = font_metrics.width(&blocks[prev_idx].text);

                    // Width can be negative when letter-spacing is negative.
                    // Not sure what should be done in this case.
                    if !width.is_valid_length() {
                        width = 1.0;
                    }

                    {
                        let r = blocks[prev_idx].bbox;
                        let r = Rect::new(r.x(), r.y(), width, r.height()).unwrap();
                        blocks[prev_idx].bbox = r;
                    }

                    let mut new_width = chunk_x;
                    for block in blocks.iter().skip(start_idx) {
                        new_width += block.bbox.width();
                    }

                    x = new_width;
                } else {
                    buf_str.clear();
                    buf_str.push(c);

                    let mut width = font_metrics.width(&buf_str);

                    // Width can be negative when letter-spacing is negative.
                    // Not sure what should be done in this case.
                    if !width.is_valid_length() {
                        width = 1.0;
                    }

                    let font_ascent = font_metrics.ascent(&buf_str);
                    let yy = y - font_ascent - tspan.baseline_shift;
                    let height = font_metrics.height();
                    let bbox = Rect::new(x, yy, width, height).unwrap();
                    x += width;

                    let mut rotate = None;
                    if let Some(ref list) = text_kind.rotate {
                        if let Some(angle) = list.get(char_idx) {
                            if !angle.is_fuzzy_zero() {
                                rotate = Some(*angle);
                            }
                        }
                    }

                    blocks.push(TextBlock {
                        text: c.to_string(),
                        is_visible: tspan.visibility == usvg::Visibility::Visible,
                        bbox,
                        rotate,
                        fill: tspan.fill.clone(),
                        stroke: tspan.stroke.clone(),
                        font: font_metrics.font(),
                        font_ascent,
                        letter_spacing: tspan.font.letter_spacing,
                        word_spacing: tspan.font.word_spacing,
                        decoration: tspan.decoration.clone(),
                    });
                }

                char_idx += 1;
            }
        }

        let mut chunk_w = 0.0;
        for block in blocks.iter().skip(start_idx) {
            chunk_w += block.bbox.width();
        }

        let adx = process_text_anchor(chunk.anchor, chunk_w);
        for block in blocks.iter_mut().skip(start_idx) {
            block.bbox = block.bbox.translate(-adx, 0.0);
        }

        last_x = chunk_x + chunk_w - adx;
        last_y = y;
    }

    let mut text_bbox = Rect::new_bbox();
    for block in &blocks {
        text_bbox = text_bbox.expand(block.bbox);
    }

    (blocks, text_bbox)
}

fn process_text_anchor(a: usvg::TextAnchor, text_width: f64) -> f64 {
    match a {
        usvg::TextAnchor::Start =>  0.0, // Nothing.
        usvg::TextAnchor::Middle => text_width / 2.0,
        usvg::TextAnchor::End =>    text_width,
    }
}
