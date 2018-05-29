// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use unicode_segmentation::UnicodeSegmentation;
use usvg;

// self
use geom::*;
use utils;


pub struct TextBlock<Font> {
    pub text: String,
    pub bbox: Rect,
    pub rotate: f64,
    pub fill: Option<usvg::Fill>,
    pub stroke: Option<usvg::Stroke>,
    pub font: Font,
    pub decoration: usvg::TextDecoration,
}

pub trait FontMetrics<Font> {
    fn set_font(&mut self, font: &usvg::Font);
    fn font(&self) -> Font;
    fn width(&self, text: &str) -> f64;
    fn ascent(&self) -> f64;
    fn height(&self) -> f64;
}

pub fn draw_blocks<Font, DrawAt>(
    text_kind: &usvg::Text,
    node: &usvg::Node,
    font_metrics: &mut FontMetrics<Font>,
    mut draw: DrawAt
) -> Rect
    where DrawAt: FnMut(&TextBlock<Font>)
{
    fn first_number_or(list: &Option<usvg::NumberList>, def: f64) -> f64 {
        list.as_ref().map(|list| list[0]).unwrap_or(def)
    }

    let mut blocks: Vec<TextBlock<Font>> = Vec::new();
    let mut last_x = 0.0;
    let mut last_y = 0.0;
    for chunk_node in node.children() {
        let kind = chunk_node.borrow();
        let chunk = match *kind {
            usvg::NodeKind::TextChunk(ref chunk) => chunk,
            _ => continue,
        };

        let mut chunk_x = first_number_or(&chunk.x, last_x);
        let mut x = chunk_x;
        let mut y = first_number_or(&chunk.y, last_y);
        let start_idx = blocks.len();
        let mut grapheme_idx = 0;

        for tspan_node in chunk_node.children() {
            let kind = tspan_node.borrow();
            let tspan = match *kind {
                usvg::NodeKind::TSpan(ref tspan) => tspan,
                _ => continue,
            };

            font_metrics.set_font(&tspan.font);

            let iter = UnicodeSegmentation::graphemes(tspan.text.as_str(), true);
            for (i, c) in iter.enumerate() {
                let mut has_custom_offset = i == 0;

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

                    if i == 0 {
                        if let Some(n) = number_at(&chunk.x) { chunk_x = n; }
                        if let Some(n) = number_at(&chunk.dx) { chunk_x += n; }
                    }
                }

                if text_kind.rotate.is_some() {
                    has_custom_offset = true;
                }

                let can_merge = !blocks.is_empty() && !has_custom_offset;
                if can_merge {
                    let prev_idx = blocks.len() - 1;
                    blocks[prev_idx].text.push_str(c);
                    let w = font_metrics.width(&blocks[prev_idx].text);
                    blocks[prev_idx].bbox.width = w;

                    let mut new_w = chunk_x;
                    for i in start_idx..blocks.len() {
                        new_w += blocks[i].bbox.width;
                    }

                    x = new_w;
                } else {
                    let width = font_metrics.width(c);
                    let yy = y - font_metrics.ascent();
                    let height = font_metrics.height();
                    let bbox = Rect { x, y: yy, width, height };
                    x += width;

                    let rotate = match text_kind.rotate {
                        Some(ref list) => { list[blocks.len()] }
                        None => 0.0,
                    };

                    blocks.push(TextBlock {
                        text: c.to_string(),
                        bbox,
                        rotate,
                        fill: tspan.fill.clone(),
                        stroke: tspan.stroke.clone(),
                        font: font_metrics.font(),
                        decoration: tspan.decoration.clone(),
                    });
                }

                grapheme_idx += 1;
            }
        }

        let mut chunk_w = 0.0;
        for i in start_idx..blocks.len() {
            chunk_w += blocks[i].bbox.width;
        }

        let adx = utils::process_text_anchor(chunk.anchor, chunk_w);
        for i in start_idx..blocks.len() {
            blocks[i].bbox.x -= adx;
        }

        last_x = chunk_x + chunk_w - adx;
        last_y = y;
    }

    let mut bbox = Rect::new_bbox();
    for block in blocks {
        bbox.expand(block.bbox);
        draw(&block);
    }

    bbox
}
