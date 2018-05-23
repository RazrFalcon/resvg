// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use unicode_segmentation::UnicodeSegmentation;
use qt;
use usvg;
use usvg::prelude::*;

// self
use super::prelude::*;
use super::{
    fill,
    stroke,
};

pub struct TextBlock {
    pub text: String,
    pub bbox: Rect,
    pub rotate: f64,
    pub fill: Option<usvg::Fill>,
    pub stroke: Option<usvg::Stroke>,
    pub font: qt::Font,
    pub decoration: usvg::TextDecoration,
}

pub fn draw(
    node: &usvg::Node,
    opt: &Options,
    p: &qt::Painter,
) -> Rect {
    let tree = &node.tree();

    if let usvg::NodeKind::Text(ref text) = *node.borrow() {
        draw_blocks(text, node, p, |block| draw_block(tree, block, opt, p))
    } else {
        unreachable!();
    }
}

// TODO: find a way to merge this with a cairo backend
pub fn draw_blocks<DrawAt>(
    text_kind: &usvg::Text,
    node: &usvg::Node,
    p: &qt::Painter,
    mut draw: DrawAt
) -> Rect
    where DrawAt: FnMut(&TextBlock)
{
    fn first_number_or(list: &Option<usvg::NumberList>, def: f64) -> f64 {
        list.as_ref().map(|list| list[0]).unwrap_or(def)
    }

    let mut blocks: Vec<TextBlock> = Vec::new();
    let mut last_x = 0.0;
    let mut last_y = 0.0;
    for chunk_node in node.children() {
        let kind = chunk_node.borrow();
        let chunk = match *kind {
            usvg::NodeKind::TextChunk(ref chunk) => chunk,
            _ => continue,
        };

        let chunk_x = first_number_or(&chunk.x, last_x);
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

            let font = init_font(&tspan.font);
            p.set_font(&font);
            let font_metrics = p.font_metrics();

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
                    let yy = y - font_metrics.ascent();
                    let height = font_metrics.height();
                    let width = font_metrics.width(c);
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
                        font: init_font(&tspan.font), // TODO: clone
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

fn draw_block(
    tree: &usvg::Tree,
    block: &TextBlock,
    opt: &Options,
    p: &qt::Painter,
) {
    p.set_font(&block.font);
    let font_metrics = p.font_metrics();

    let bbox = block.bbox;

    let old_ts = p.get_transform();

    if !block.rotate.is_fuzzy_zero() {
        let mut ts = usvg::Transform::default();
        ts.rotate_at(block.rotate, bbox.x, bbox.y + font_metrics.ascent());
        p.apply_transform(&ts.to_native());
    }

    let mut line_rect = Rect::new(bbox.x, 0.0, bbox.width, font_metrics.line_width());

    // Draw underline.
    //
    // Should be drawn before/under text.
    if let Some(ref style) = block.decoration.underline {
        line_rect.y = bbox.y + font_metrics.height() - font_metrics.underline_pos();
        draw_line(tree, line_rect, &style.fill, &style.stroke, opt, p);
    }

    // Draw overline.
    //
    // Should be drawn before/under text.
    if let Some(ref style) = block.decoration.overline {
        line_rect.y = bbox.y + font_metrics.height() - font_metrics.overline_pos();
        draw_line(tree, line_rect, &style.fill, &style.stroke, opt, p);
    }

    // Draw text.
    fill::apply(tree, &block.fill, opt, bbox, p);
    stroke::apply(tree, &block.stroke, opt, bbox, p);

    p.draw_text(bbox.x, bbox.y, &block.text);

    // Draw line-through.
    //
    // Should be drawn after/over text.
    if let Some(ref style) = block.decoration.line_through {
        line_rect.y = bbox.y + font_metrics.ascent() - font_metrics.strikeout_pos();
        draw_line(tree, line_rect, &style.fill, &style.stroke, opt, p);
    }

    p.set_transform(&old_ts);
}

fn init_font(dom_font: &usvg::Font) -> qt::Font {
    let mut font = qt::Font::new();

    font.set_family(&dom_font.family);

    let font_style = match dom_font.style {
        usvg::FontStyle::Normal => qt::FontStyle::StyleNormal,
        usvg::FontStyle::Italic => qt::FontStyle::StyleItalic,
        usvg::FontStyle::Oblique => qt::FontStyle::StyleOblique,
    };
    font.set_style(font_style);

    if dom_font.variant == usvg::FontVariant::SmallCaps {
        font.set_small_caps(true);
    }

    let font_weight = match dom_font.weight {
        usvg::FontWeight::W100       => qt::FontWeight::Thin,
        usvg::FontWeight::W200       => qt::FontWeight::ExtraLight,
        usvg::FontWeight::W300       => qt::FontWeight::Light,
        usvg::FontWeight::W400       => qt::FontWeight::Normal,
        usvg::FontWeight::W500       => qt::FontWeight::Medium,
        usvg::FontWeight::W600       => qt::FontWeight::DemiBold,
        usvg::FontWeight::W700       => qt::FontWeight::Bold,
        usvg::FontWeight::W800       => qt::FontWeight::ExtraBold,
        usvg::FontWeight::W900       => qt::FontWeight::Black,
    };
    font.set_weight(font_weight);

    let font_stretch = match dom_font.stretch {
        usvg::FontStretch::Normal         => qt::FontStretch::Unstretched,
        usvg::FontStretch::Narrower |
        usvg::FontStretch::Condensed      => qt::FontStretch::Condensed,
        usvg::FontStretch::UltraCondensed => qt::FontStretch::UltraCondensed,
        usvg::FontStretch::ExtraCondensed => qt::FontStretch::ExtraCondensed,
        usvg::FontStretch::SemiCondensed  => qt::FontStretch::SemiCondensed,
        usvg::FontStretch::SemiExpanded   => qt::FontStretch::SemiExpanded,
        usvg::FontStretch::Wider |
        usvg::FontStretch::Expanded       => qt::FontStretch::Expanded,
        usvg::FontStretch::ExtraExpanded  => qt::FontStretch::ExtraExpanded,
        usvg::FontStretch::UltraExpanded  => qt::FontStretch::UltraExpanded,
    };
    font.set_stretch(font_stretch);

    // a-font-size-001.svg
    font.set_size(dom_font.size);

    font
}

fn draw_line(
    tree: &usvg::Tree,
    r: Rect,
    fill: &Option<usvg::Fill>,
    stroke: &Option<usvg::Stroke>,
    opt: &Options,
    p: &qt::Painter,
) {
    fill::apply(tree, fill, opt, r, p);
    stroke::apply(tree, stroke, opt, r, p);
    p.draw_rect(r.x, r.y, r.width + 1.0, r.height);
}
