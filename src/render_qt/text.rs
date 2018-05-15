// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::f64;

// external
use qt;
use usvg;
use usvg::prelude::*;

// self
use super::prelude::*;
use super::{
    fill,
    stroke,
};


pub fn draw(
    node: &usvg::Node,
    opt: &Options,
    p: &qt::Painter,
) -> Rect {
    draw_tspan(node, p,
        |tspan, x, y, w, font| _draw_tspan(node, tspan, opt, x, y, w, &font, p))
}

pub fn draw_tspan<DrawAt>(
    node: &usvg::Node,
    p: &qt::Painter,
    mut draw_at: DrawAt
) -> Rect
    where DrawAt: FnMut(&usvg::TSpan, f64, f64, f64, &qt::Font)
{
    let mut bbox = Rect::new_bbox();
    let mut font_list = Vec::new();
    let mut tspan_w_list = Vec::new();
    for chunk_node in node.children() {
        font_list.clear();
        tspan_w_list.clear();
        let mut chunk_width = 0.0;

        if let usvg::NodeKind::TextChunk(ref chunk) = *chunk_node.borrow() {
            for tspan_node in chunk_node.children() {
                if let usvg::NodeKind::TSpan(ref tspan) = *tspan_node.borrow() {
                    let font = init_font(&tspan.font);
                    p.set_font(&font);
                    let font_metrics = p.font_metrics();
                    let tspan_width = font_metrics.width(&tspan.text);

                    font_list.push(font);
                    chunk_width += tspan_width;
                    tspan_w_list.push(tspan_width);

                    bbox.expand((chunk.x, chunk.y - font_metrics.ascent(),
                                 chunk_width, font_metrics.height()).into());
                }
            }

            let mut x = utils::process_text_anchor(chunk.x, chunk.anchor, chunk_width);

            for (idx, tspan_node) in chunk_node.children().enumerate() {
                if let usvg::NodeKind::TSpan(ref tspan) = *tspan_node.borrow() {
                    let width = tspan_w_list[idx];
                    let font = &font_list[idx];

                    draw_at(tspan, x, chunk.y, width, font);
                    x += width;
                }
            }
        }
    }

    bbox
}

fn _draw_tspan(
    node: &usvg::Node,
    tspan: &usvg::TSpan,
    opt: &Options,
    x: f64,
    mut y: f64,
    width: f64,
    font: &qt::Font,
    p: &qt::Painter,
) {
    p.set_font(&font);
    let font_metrics = p.font_metrics();

    let baseline_offset = font_metrics.ascent();
    y -= baseline_offset;

    let mut line_rect = Rect::new(
        x,
        0.0,
        width,
        font_metrics.line_width(),
    );

    // Draw underline.
    //
    // Should be drawn before/under text.
    if let Some(ref style) = tspan.decoration.underline {
        line_rect.y = y + font_metrics.height() - font_metrics.underline_pos();
        draw_line(&node.tree(), line_rect, &style.fill, &style.stroke, opt, p);
    }

    // Draw overline.
    //
    // Should be drawn before/under text.
    if let Some(ref style) = tspan.decoration.overline {
        line_rect.y = y + font_metrics.height() - font_metrics.overline_pos();
        draw_line(&node.tree(), line_rect, &style.fill, &style.stroke, opt, p);
    }

    let bbox = Rect::new(x, y, width, font_metrics.height());

    // Draw text.
    fill::apply(&node.tree(), &tspan.fill, opt, bbox, p);
    stroke::apply(&node.tree(), &tspan.stroke, opt, bbox, p);

    p.draw_text(x, y, &tspan.text);

    // Draw line-through.
    //
    // Should be drawn after/over text.
    if let Some(ref style) = tspan.decoration.line_through {
        line_rect.y = y + baseline_offset - font_metrics.strikeout_pos();
        draw_line(&node.tree(), line_rect, &style.fill, &style.stroke, opt, p);
    }
}

pub fn init_font(dom_font: &usvg::Font) -> qt::Font {
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
    p.draw_rect(r.x, r.y, r.width, r.height);
}
