// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::f64;

// external
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


pub struct PangoData {
    pub layout: pango::Layout,
    pub context: pango::Context,
    pub font: pango::FontDescription,
}

pub fn draw(
    node: &usvg::Node,
    opt: &Options,
    cr: &cairo::Context,
) -> Rect {
    draw_tspan(node, opt, cr,
        |tspan, x, y, w, d| _draw_tspan(node, tspan, opt, x, y, w, d, cr))
}

pub fn draw_tspan<DrawAt>(
    node: &usvg::Node,
    opt: &Options,
    cr: &cairo::Context,
    mut draw_at: DrawAt,
) -> Rect
    where DrawAt: FnMut(&usvg::TSpan, f64, f64, f64, &PangoData)
{
    let mut bbox = Rect::new_bbox();
    let mut pc_list = Vec::new();
    let mut tspan_w_list = Vec::new();
    for chunk_node in node.children() {
        pc_list.clear();
        tspan_w_list.clear();
        let mut chunk_width = 0.0;

        if let usvg::NodeKind::TextChunk(ref chunk) = *chunk_node.borrow() {
            for tspan_node in chunk_node.children() {
                if let usvg::NodeKind::TSpan(ref tspan) = *tspan_node.borrow() {
                    let context = pc::create_context(cr).unwrap();
                    pc::update_context(cr, &context);
                    pc::context_set_resolution(&context, opt.usvg.dpi);

                    let font = init_font(&tspan.font, opt.usvg.dpi);

                    let layout = pango::Layout::new(&context);
                    layout.set_font_description(Some(&font));
                    layout.set_text(&tspan.text);
                    let tspan_width = layout.get_size().0.scale();

                    let mut layout_iter = layout.get_iter().unwrap();
                    let ascent = layout_iter.get_baseline().scale();
                    let text_h = layout.get_size().1.scale();

                    pc_list.push(PangoData {
                        layout,
                        context,
                        font,
                    });
                    chunk_width += tspan_width;
                    tspan_w_list.push((tspan_width, ascent));

                    bbox.expand((chunk.x, chunk.y - ascent, chunk_width, text_h).into());
                }
            }

            let mut x = utils::process_text_anchor(chunk.x, chunk.anchor, chunk_width);

            for (idx, tspan_node) in chunk_node.children().enumerate() {
                if let usvg::NodeKind::TSpan(ref tspan) = *tspan_node.borrow() {
                    let (width, ascent) = tspan_w_list[idx];
                    let pc = &pc_list[idx];

                    draw_at(tspan, x, chunk.y - ascent, width, pc);
                    x += width;
                }
            }
        }
    }

    if bbox.x == f64::MAX { bbox.x = 0.0; }
    if bbox.y == f64::MAX { bbox.y = 0.0; }

    bbox
}

fn _draw_tspan(
    node: &usvg::Node,
    tspan: &usvg::TSpan,
    opt: &Options,
    x: f64,
    y: f64,
    width: f64,
    pd: &PangoData,
    cr: &cairo::Context,
) {
    let font_metrics = pd.context.get_metrics(Some(&pd.font), None).unwrap();

    let mut layout_iter = pd.layout.get_iter().unwrap();
    let baseline_offset = layout_iter.get_baseline().scale();

    // Contains only characters path bounding box,
    // so spaces around text are ignored.
    let bbox = calc_layout_bbox(&pd.layout, x, y);

    let mut line_rect = Rect::new(
        x,
        0.0,
        width,
        font_metrics.get_underline_thickness().scale(),
    );

    // Draw underline.
    //
    // Should be drawn before/under text.
    if let Some(ref style) = tspan.decoration.underline {
        line_rect.y = y + baseline_offset
                        - font_metrics.get_underline_position().scale();
        draw_line(&node.tree(), line_rect, &style.fill, &style.stroke, opt, cr);
    }

    // Draw overline.
    //
    // Should be drawn before/under text.
    if let Some(ref style) = tspan.decoration.overline {
        line_rect.y = y + font_metrics.get_underline_thickness().scale();
        draw_line(&node.tree(), line_rect, &style.fill, &style.stroke, opt, cr);
    }

    // Draw text.
    cr.move_to(x, y);

    fill::apply(&node.tree(), &tspan.fill, opt, bbox, cr);
    pc::update_layout(cr, &pd.layout);
    pc::show_layout(cr, &pd.layout);

    stroke::apply(&node.tree(), &tspan.stroke, opt, bbox, cr);
    pc::layout_path(cr, &pd.layout);
    cr.stroke();

    cr.move_to(-x, -y);

    // Draw line-through.
    //
    // Should be drawn after/over text.
    if let Some(ref style) = tspan.decoration.line_through {
        line_rect.y = y + baseline_offset - font_metrics.get_strikethrough_position().scale();
        line_rect.height = font_metrics.get_strikethrough_thickness().scale();
        draw_line(&node.tree(), line_rect, &style.fill, &style.stroke, opt, cr);
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
    line_bbox: Rect,
    fill: &Option<usvg::Fill>,
    stroke: &Option<usvg::Stroke>,
    opt: &Options,
    cr: &cairo::Context,
) {
    cr.rectangle(line_bbox.x, line_bbox.y, line_bbox.width, line_bbox.height);

    fill::apply(tree, fill, opt, line_bbox, cr);
    if stroke.is_some() {
        cr.fill_preserve();

        stroke::apply(tree, &stroke, opt, line_bbox, cr);
        cr.stroke();
    } else {
        cr.fill();
    }
}
