// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::f64;

// external
use unicode_segmentation::UnicodeSegmentation;
use cairo;
use pango::{
    self,
    LayoutExt,
    ContextExt,
};
use pangocairo::functions as pc;
use usvg;
use usvg::prelude::*;

// self
use super::prelude::*;
use super::{
    fill,
    stroke,
};


trait PangoScale {
    fn scale(&self) -> f64;
}

impl PangoScale for i32 {
    fn scale(&self) -> f64 {
        (self / pango::SCALE) as f64
    }
}


pub struct TextBlock {
    pub text: String,
    pub bbox: Rect,
    pub fill: Option<usvg::Fill>,
    pub stroke: Option<usvg::Stroke>,
    pub font: pango::FontDescription,
    pub decoration: usvg::TextDecoration,
}

pub fn draw(
    node: &usvg::Node,
    opt: &Options,
    cr: &cairo::Context,
) -> Rect {
    let tree = &node.tree();
    draw_blocks(node, opt, cr, |block| draw_block(tree, block, opt, cr))
}

// TODO: find a way to merge this with a Qt backend
pub fn draw_blocks<DrawAt>(
    node: &usvg::Node,
    opt: &Options,
    cr: &cairo::Context,
    mut draw: DrawAt,
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

            let context = init_pango_context(opt, cr);
            let font = init_font(&tspan.font, opt.usvg.dpi);
            let layout = pango::Layout::new(&context);
            layout.set_font_description(&font);

            let mut iter = UnicodeSegmentation::graphemes(tspan.text.as_str(), true);
            for (i, c) in iter.enumerate() {
                let mut has_custom_offset = i == 0;

                {
                    let mut number_at = |list: &Option<usvg::NumberList>| -> Option<f64> {
                        if let Some(ref list) = list {
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

                let can_merge = !blocks.is_empty() && !has_custom_offset;
                if can_merge {
                    let prev_idx = blocks.len() - 1;
                    blocks[prev_idx].text.push_str(c);

                    layout.set_text(&blocks[prev_idx].text);
                    let w = layout.get_size().0.scale();
                    blocks[prev_idx].bbox.width = w;

                    let mut new_w = chunk_x;
                    for i in start_idx..blocks.len() {
                        new_w += blocks[i].bbox.width;
                    }

                    x = new_w;
                } else {
                    let mut layout_iter = layout.get_iter().unwrap();
                    let yy = y - layout_iter.get_baseline().scale();
                    let height = layout.get_size().1.scale();

                    layout.set_text(c);
                    let width = layout.get_size().0.scale();

                    let mut bbox = Rect { x, y: yy, width, height };
                    x += width;

                    blocks.push(TextBlock {
                        text: c.to_string(),
                        bbox,
                        fill: tspan.fill.clone(),
                        stroke: tspan.stroke.clone(),
                        font: font.clone(),
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

        let mut adx = utils::process_text_anchor(chunk.anchor, chunk_w);
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

    if bbox.x == f64::MAX { bbox.x = 0.0; }
    if bbox.y == f64::MAX { bbox.y = 0.0; }

    bbox
}

pub fn init_pango_context(opt: &Options, cr: &cairo::Context) -> pango::Context {
    let context = pc::create_context(cr).unwrap();
    pc::update_context(cr, &context);
    pc::context_set_resolution(&context, opt.usvg.dpi);
    context
}

pub fn init_pango_layout(
    text: &str,
    font: &pango::FontDescription,
    context: &pango::Context,
) -> pango::Layout {
    let layout = pango::Layout::new(&context);
    layout.set_font_description(font);
    layout.set_text(text);
    layout
}

fn draw_block(
    tree: &usvg::Tree,
    block: &TextBlock,
    opt: &Options,
    cr: &cairo::Context,
) {
    let context = init_pango_context(opt, cr);
    let layout = init_pango_layout(&block.text, &block.font, &context);

    let fm = context.get_metrics(&block.font, None).unwrap();

    let mut layout_iter = layout.get_iter().unwrap();
    let baseline_offset = layout_iter.get_baseline().scale();

    let bbox = block.bbox;

    // Contains only characters path bounding box,
    // so spaces around text are ignored.
    let inner_bbox = calc_layout_bbox(&layout, bbox.x, bbox.y);

    let mut line_rect = Rect::new(
        bbox.x,
        0.0,
        bbox.width,
        fm.get_underline_thickness().scale(),
    );

    // Draw underline.
    //
    // Should be drawn before/under text.
    if let Some(ref style) = block.decoration.underline {
        line_rect.y = bbox.y + baseline_offset - fm.get_underline_position().scale();
        draw_line(tree, line_rect, &style.fill, &style.stroke, opt, cr);
    }

    // Draw overline.
    //
    // Should be drawn before/under text.
    if let Some(ref style) = block.decoration.overline {
        line_rect.y = bbox.y + fm.get_underline_thickness().scale();
        draw_line(tree, line_rect, &style.fill, &style.stroke, opt, cr);
    }

    // Draw text.
    cr.move_to(bbox.x, bbox.y);

    fill::apply(tree, &block.fill, opt, inner_bbox, cr);
    pc::update_layout(cr, &layout);
    pc::show_layout(cr, &layout);

    stroke::apply(tree, &block.stroke, opt, inner_bbox, cr);
    pc::layout_path(cr, &layout);
    cr.stroke();

    cr.move_to(-bbox.x, -bbox.y);

    // Draw line-through.
    //
    // Should be drawn after/over text.
    if let Some(ref style) = block.decoration.line_through {
        line_rect.y = bbox.y + baseline_offset - fm.get_strikethrough_position().scale();
        line_rect.height = fm.get_strikethrough_thickness().scale();
        draw_line(tree, line_rect, &style.fill, &style.stroke, opt, cr);
    }
}

fn init_font(dom_font: &usvg::Font, dpi: f64) -> pango::FontDescription {
    let mut font = pango::FontDescription::new();

    font.set_family(&dom_font.family);

    let font_style = match dom_font.style {
        usvg::FontStyle::Normal => pango::Style::Normal,
        usvg::FontStyle::Italic => pango::Style::Italic,
        usvg::FontStyle::Oblique => pango::Style::Oblique,
    };
    font.set_style(font_style);

    let font_variant = match dom_font.variant {
        usvg::FontVariant::Normal => pango::Variant::Normal,
        usvg::FontVariant::SmallCaps => pango::Variant::SmallCaps,
    };
    font.set_variant(font_variant);

    let font_weight = match dom_font.weight {
        usvg::FontWeight::W100       => pango::Weight::Thin,
        usvg::FontWeight::W200       => pango::Weight::Ultralight,
        usvg::FontWeight::W300       => pango::Weight::Light,
        usvg::FontWeight::W400       => pango::Weight::Normal,
        usvg::FontWeight::W500       => pango::Weight::Medium,
        usvg::FontWeight::W600       => pango::Weight::Semibold,
        usvg::FontWeight::W700       => pango::Weight::Bold,
        usvg::FontWeight::W800       => pango::Weight::Ultrabold,
        usvg::FontWeight::W900       => pango::Weight::Heavy,
    };
    font.set_weight(font_weight);

    let font_stretch = match dom_font.stretch {
        usvg::FontStretch::Normal         => pango::Stretch::Normal,
        usvg::FontStretch::Narrower |
        usvg::FontStretch::Condensed      => pango::Stretch::Condensed,
        usvg::FontStretch::UltraCondensed => pango::Stretch::UltraCondensed,
        usvg::FontStretch::ExtraCondensed => pango::Stretch::ExtraCondensed,
        usvg::FontStretch::SemiCondensed  => pango::Stretch::SemiCondensed,
        usvg::FontStretch::SemiExpanded   => pango::Stretch::SemiExpanded,
        usvg::FontStretch::Wider |
        usvg::FontStretch::Expanded       => pango::Stretch::Expanded,
        usvg::FontStretch::ExtraExpanded  => pango::Stretch::ExtraExpanded,
        usvg::FontStretch::UltraExpanded  => pango::Stretch::UltraExpanded,
    };
    font.set_stretch(font_stretch);

    // a-font-size-001.svg
    let font_size = dom_font.size * (pango::SCALE as f64) / dpi * 72.0;
    font.set_size(font_size as i32);

    font
}

pub fn calc_layout_bbox(layout: &pango::Layout, x: f64, y: f64) -> Rect {
    let (ink_rect, _) = layout.get_extents();

    (
        x + ink_rect.x.scale(),
        y + ink_rect.y.scale(),
        ink_rect.width.scale(),
        ink_rect.height.scale(),
    ).into()
}

fn draw_line(
    tree: &usvg::Tree,
    r: Rect,
    fill: &Option<usvg::Fill>,
    stroke: &Option<usvg::Stroke>,
    opt: &Options,
    cr: &cairo::Context,
) {
    cr.rectangle(r.x, r.y, r.width, r.height);

    fill::apply(tree, fill, opt, r, cr);
    if stroke.is_some() {
        cr.fill_preserve();

        stroke::apply(tree, &stroke, opt, r, cr);
        cr.stroke();
    } else {
        cr.fill();
    }
}
