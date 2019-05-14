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

// self
use super::prelude::*;
use crate::backend_utils;
use crate::backend_utils::text::{
    self,
    FontMetrics,
};
use super::style;


trait FromPangoScale {
    fn from_pango(&self) -> f64;
}

impl FromPangoScale for i32 {
    fn from_pango(&self) -> f64 {
        *self as f64 / pango::SCALE as f64
    }
}


trait ToPangoScale {
    fn to_pango(&self) -> i32;
}

impl ToPangoScale for f64 {
    fn to_pango(&self) -> i32 {
        (*self * pango::SCALE as f64) as i32
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
        set_text_spacing(font.letter_spacing, &self.layout);
        self.layout.set_font_description(&init_font(font, self.dpi));
    }

    fn font(&self) -> pango::FontDescription {
        self.layout.get_font_description().unwrap()
    }

    fn width(&self, text: &str) -> f64 {
        self.layout.set_text(text);
        self.layout.get_size().0.from_pango()
    }

    fn ascent(&self, text: &str) -> f64 {
        self.layout.set_text(text);
        let mut layout_iter = self.layout.get_iter().unwrap();
        layout_iter.get_baseline().from_pango()
    }

    fn height(&self) -> f64 {
        self.layout.get_size().1.from_pango()
    }
}

pub fn draw(
    tree: &usvg::Tree,
    text: &usvg::Text,
    opt: &Options,
    cr: &cairo::Context,
) -> Rect {
    let mut font_opt = cr.get_font_options();
    if !backend_utils::use_text_antialiasing(text.rendering_mode) {
        cr.set_antialias(cairo::Antialias::None);
        font_opt.set_antialias(cairo::Antialias::None);
        cr.set_font_options(&font_opt);
    }

    let mut fm = PangoFontMetrics::new(opt, cr);
    let (blocks, text_bbox) = text::prepare_blocks(text, &mut fm);
    text::draw_blocks(blocks, |block| draw_block(tree, block, text_bbox, opt, cr));

    // Revert anti-aliasing.
    cr.set_antialias(cairo::Antialias::Default);
    font_opt.set_antialias(cairo::Antialias::Default);
    cr.set_font_options(&font_opt);

    text_bbox
}

pub fn init_pango_context(opt: &Options, cr: &cairo::Context) -> pango::Context {
    let context = pc::create_context(cr).unwrap();
    pc::update_context(cr, &context);
    pc::context_set_resolution(&context, opt.usvg.dpi);
    context
}

pub fn init_pango_layout(
    block: &text::TextBlock<pango::FontDescription>,
    context: &pango::Context,
) -> pango::Layout {
    let layout = pango::Layout::new(context);
    layout.set_font_description(&block.font);
    set_text_spacing(block.letter_spacing, &layout);
    layout.set_text(&block.text);
    layout
}

fn set_text_spacing(
    letter_spacing: Option<f64>,
    layout: &pango::Layout,
) {
    let attr_list = pango::AttrList::new();

    if let Some(letter_spacing) = letter_spacing {
        attr_list.insert(pango::Attribute::new_letter_spacing(letter_spacing.to_pango()).unwrap());
    }

    layout.set_attributes(&attr_list);
}

fn draw_block(
    tree: &usvg::Tree,
    block: &text::TextBlock<pango::FontDescription>,
    text_bbox: Rect,
    opt: &Options,
    cr: &cairo::Context,
) {
    // `tspan` doesn't have a bbox by the SVG spec and should use the whole `text` bbox.
    // That's why we are using `text_bbox` instead of `block.bbox`.

    let context = init_pango_context(opt, cr);
    let layout = init_pango_layout(&block, &context);

    let fm = context.get_metrics(&block.font, None).unwrap();

    let bbox = block.bbox;

    let underline_height = fm.get_underline_thickness().from_pango();
    let line_rect = try_opt!(Rect::new(bbox.x(), bbox.y(), bbox.width(), underline_height), ());

    let old_ts = cr.get_matrix();
    if let Some(rotate) = block.rotate {
        let ts = usvg::Transform::new_rotate_at(rotate, bbox.x(), bbox.y() + block.font_ascent);
        cr.transform(ts.to_native());
    }

    // Draw underline.
    //
    // Should be drawn before/under text.
    if let Some(ref style) = block.decoration.underline {
        let ty = block.font_ascent - fm.get_underline_position().from_pango();
        draw_line(tree, line_rect.translate(0.0, ty), text_bbox, &style.fill, &style.stroke, opt, cr);
    }

    // Draw overline.
    //
    // Should be drawn before/under text.
    if let Some(ref style) = block.decoration.overline {
        let ty = underline_height;
        draw_line(tree, line_rect.translate(0.0, ty), text_bbox, &style.fill, &style.stroke, opt, cr);
    }

    // Draw text.
    cr.move_to(bbox.x(), bbox.y());

    style::fill(tree, &block.fill, opt, text_bbox, cr);
    pc::update_layout(cr, &layout);
    pc::show_layout(cr, &layout);

    style::stroke(tree, &block.stroke, opt, text_bbox, cr);
    pc::layout_path(cr, &layout);
    cr.stroke();

    cr.move_to(-bbox.x(), -bbox.y());

    // Draw line-through.
    //
    // Should be drawn after/over text.
    if let Some(ref style) = block.decoration.line_through {
        let line_rect = Rect::new(
            bbox.x(),
            bbox.y() + block.font_ascent - fm.get_strikethrough_position().from_pango(),
            bbox.width(),
            fm.get_strikethrough_thickness().from_pango(),
        );
        let line_rect = try_opt!(line_rect, ());

        draw_line(tree, line_rect, text_bbox, &style.fill, &style.stroke, opt, cr);
    }

    cr.set_matrix(old_ts);
}

fn init_font(dom_font: &usvg::Font, dpi: f64) -> pango::FontDescription {
    let mut font = pango::FontDescription::new();

    // We have to remove quotes, because `pango` doesn't support them.
    let font_family = dom_font.family.replace('\'', "");
    font.set_family(&font_family);

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

    let font_size = dom_font.size.value().to_pango() as f64 / dpi * 72.0;
    font.set_size(font_size as i32);

    font
}

fn draw_line(
    tree: &usvg::Tree,
    r: Rect,
    text_bbox: Rect,
    fill: &Option<usvg::Fill>,
    stroke: &Option<usvg::Stroke>,
    opt: &Options,
    cr: &cairo::Context,
) {
    cr.rectangle(r.x(), r.y(), r.width(), r.height());

    style::fill(tree, fill, opt, text_bbox, cr);
    if stroke.is_some() {
        cr.fill_preserve();

        style::stroke(tree, &stroke, opt, text_bbox, cr);
        cr.stroke();
    } else {
        cr.fill();
    }
}
