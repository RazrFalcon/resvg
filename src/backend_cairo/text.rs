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
use backend_utils::text::{
    self,
    FontMetrics,
};
use super::{
    fill,
    stroke,
};


trait PangoScale {
    fn scale(&self) -> f64;
}

impl PangoScale for i32 {
    fn scale(&self) -> f64 {
        *self as f64 / pango::SCALE as f64
    }
}

pub struct PangoFontMetrics {
    layout: pango::Layout,
    dpi: f64,
}

impl PangoFontMetrics {
    pub fn new(opt: &Options, cr: &cairo::Context) -> Self {
        let context = init_pango_context(opt, cr);
        let layout = pango::Layout::new(&context);
        PangoFontMetrics { layout, dpi: opt.usvg.dpi }
    }
}

impl FontMetrics<pango::FontDescription> for PangoFontMetrics {
    fn set_font(&mut self, font: &usvg::Font) {
        let font = init_font(font, self.dpi);
        self.layout.set_font_description(&font);
    }

    fn font(&self) -> pango::FontDescription {
        self.layout.get_font_description().unwrap()
    }

    fn width(&self, text: &str) -> f64 {
        self.layout.set_text(text);
        self.layout.get_size().0.scale()
    }

    fn ascent(&self) -> f64 {
        let mut layout_iter = self.layout.get_iter().unwrap();
        layout_iter.get_baseline().scale()
    }

    fn height(&self) -> f64 {
        self.layout.get_size().1.scale()
    }
}

pub fn draw(
    tree: &usvg::Tree,
    text_node: &usvg::Text,
    opt: &Options,
    cr: &cairo::Context,
) -> Rect {
    let mut fm = PangoFontMetrics::new(opt, cr);
    let blocks = text::prepare_blocks(text_node, &mut fm);
    text::draw_blocks(blocks, |block| draw_block(tree, block, opt, cr))
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
    block: &text::TextBlock<pango::FontDescription>,
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
    let inner_bbox = get_layout_bbox(&layout, bbox.x, bbox.y);

    let mut line_rect = Rect::new(bbox.x, 0.0, bbox.width, fm.get_underline_thickness().scale());

    let old_ts = cr.get_matrix();
    if !block.rotate.is_fuzzy_zero() {
        let mut ts = usvg::Transform::default();
        ts.rotate_at(block.rotate, bbox.x, bbox.y + baseline_offset);
        cr.transform(ts.to_native());
    }

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

    cr.set_matrix(old_ts);
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

    let font_size = dom_font.size.value() * (pango::SCALE as f64) / dpi * 72.0;
    font.set_size(font_size as i32);

    font
}

pub fn get_layout_bbox(layout: &pango::Layout, x: f64, y: f64) -> Rect {
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
    debug_assert!(!r.height.is_fuzzy_zero());

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
