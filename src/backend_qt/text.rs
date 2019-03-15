// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use qt;

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


pub struct QtFontMetrics<'a> {
    p: &'a mut qt::Painter,
}

impl<'a> QtFontMetrics<'a> {
    pub fn new(p: &'a mut qt::Painter) -> Self {
        QtFontMetrics { p }
    }
}

impl<'a> FontMetrics<qt::Font> for QtFontMetrics<'a> {
    fn set_font(&mut self, font: &usvg::Font) {
        let font = init_font(font);
        self.p.set_font(&font);
    }

    fn font(&self) -> qt::Font {
        self.p.font()
    }

    fn width(&self, text: &str) -> f64 {
        self.p.font_metrics().width(text)
    }

    fn ascent(&self, _: &str) -> f64 {
        self.p.font_metrics().ascent()
    }

    fn height(&self) -> f64 {
        self.p.font_metrics().height()
    }
}

pub fn draw(
    tree: &usvg::Tree,
    text_node: &usvg::Text,
    opt: &Options,
    p: &mut qt::Painter,
) -> Rect {
    let (blocks, text_bbox) = text::prepare_blocks(text_node, &mut QtFontMetrics::new(p));
    text::draw_blocks(blocks, |block| draw_block(tree, block, text_bbox, opt, p));
    text_bbox
}

fn draw_block(
    tree: &usvg::Tree,
    block: &text::TextBlock<qt::Font>,
    text_bbox: Rect,
    opt: &Options,
    p: &mut qt::Painter,
) {
    // `tspan` doesn't have a bbox by the SVG spec and should use the whole `text` bbox.
    // That's why we are using `text_bbox` instead of `block.bbox`.

    p.set_font(&block.font);
    let font_metrics = p.font_metrics();

    let bbox = block.bbox;

    let old_ts = p.get_transform();

    if let Some(rotate) = block.rotate {
        let ts = usvg::Transform::new_rotate_at(rotate, bbox.x, bbox.y + block.font_ascent);
        p.apply_transform(&ts.to_native());
    }

    let mut line_rect = Rect::new(bbox.x, 0.0, bbox.width, font_metrics.line_width());

    // Draw underline.
    //
    // Should be drawn before/under text.
    if let Some(ref style) = block.decoration.underline {
        line_rect.y = bbox.y + font_metrics.height() - font_metrics.underline_pos();
        draw_line(tree, line_rect, text_bbox, &style.fill, &style.stroke, opt, p);
    }

    // Draw overline.
    //
    // Should be drawn before/under text.
    if let Some(ref style) = block.decoration.overline {
        line_rect.y = bbox.y + font_metrics.height() - font_metrics.overline_pos();
        draw_line(tree, line_rect, text_bbox, &style.fill, &style.stroke, opt, p);
    }

    // Draw text.
    fill::apply(tree, &block.fill, opt, text_bbox, p);
    stroke::apply(tree, &block.stroke, opt, text_bbox, p);

    p.draw_text(bbox.x, bbox.y, &block.text);

    // Draw line-through.
    //
    // Should be drawn after/over text.
    if let Some(ref style) = block.decoration.line_through {
        line_rect.y = bbox.y + font_metrics.ascent() - font_metrics.strikeout_pos();
        draw_line(tree, line_rect, text_bbox, &style.fill, &style.stroke, opt, p);
    }

    p.set_transform(&old_ts);
}

fn init_font(dom_font: &usvg::Font) -> qt::Font {
    let mut font = qt::Font::new();

    font.set_family(&dom_font.family);

    let font_style = match dom_font.style {
        usvg::FontStyle::Normal => qt::FontStyle::Normal,
        usvg::FontStyle::Italic => qt::FontStyle::Italic,
        usvg::FontStyle::Oblique => qt::FontStyle::Oblique,
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

    if let Some(letter_spacing) = dom_font.letter_spacing {
        font.set_letter_spacing(letter_spacing);
    }

    if let Some(word_spacing) = dom_font.word_spacing {
        font.set_word_spacing(word_spacing);
    }

    font.set_size(dom_font.size.value());

    font
}

fn draw_line(
    tree: &usvg::Tree,
    r: Rect,
    text_bbox: Rect,
    fill: &Option<usvg::Fill>,
    stroke: &Option<usvg::Stroke>,
    opt: &Options,
    p: &mut qt::Painter,
) {
    fill::apply(tree, fill, opt, text_bbox, p);
    stroke::apply(tree, stroke, opt, text_bbox, p);
    p.draw_rect(r.x, r.y, r.width + 1.0, r.height);
}
