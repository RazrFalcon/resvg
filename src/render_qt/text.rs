// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use qt;

use dom;

use math::{
    Rect,
};

use super::{
    fill,
    stroke,
};


pub fn draw(
    doc: &dom::Document,
    elem: &dom::Text,
    p: &qt::Painter,
) {
    if elem.children.is_empty() {
        return;
    }

    for chunk in &elem.children {
        let mut chunk_width = 0.0;

        for tspan in &chunk.children {
            let font = init_font(&tspan.font);
            p.set_font(&font);
            let font_metrics = p.font_metrics();
            chunk_width += font_metrics.width(&tspan.text);
        }

        let (mut x, y) = (chunk.x, chunk.y);

        x = process_text_anchor(x, chunk.anchor, chunk_width);

        for tspan in &chunk.children {
            x += draw_tspan(doc, tspan, x, y, p);
        }
    }
}

fn draw_tspan(
    doc: &dom::Document,
    tspan: &dom::TSpan,
    x: f64,
    mut y: f64,
    p: &qt::Painter,
) -> f64
{
    let font = init_font(&tspan.font);

    p.set_font(&font);
    let font_metrics = p.font_metrics();

    let baseline_offset = font_metrics.ascent();
    y -= baseline_offset;

    // Contains layout width including leading and trailing spaces.
    let layout_width = font_metrics.width(&tspan.text);

    let mut line_rect = Rect {
        x: x,
        y: 0.0,
        w: layout_width,
        h: font_metrics.line_width(),
    };

    // Draw underline.
    //
    // Should be drawn before/under text.
    if let Some(ref style) = tspan.decoration.underline {
        line_rect.y = y + font_metrics.height() - font_metrics.underline_pos();
        draw_line(doc, &style.fill, &style.stroke, line_rect, p);
    }

    // Draw overline.
    //
    // Should be drawn before/under text.
    if let Some(ref style) = tspan.decoration.overline {
        line_rect.y = y + font_metrics.height() - font_metrics.overline_pos();
        draw_line(doc, &style.fill, &style.stroke, line_rect, p);
    }

    // Draw text.
    fill::apply(doc, &tspan.fill, p);
    stroke::apply(doc, &tspan.stroke, p);

    p.draw_text(x, y, &tspan.text);

    // Draw line-through.
    //
    // Should be drawn after/over text.
    if let Some(ref style) = tspan.decoration.line_through {
        line_rect.y = y + baseline_offset - font_metrics.strikeout_pos();
        draw_line(doc, &style.fill, &style.stroke, line_rect, p);
    }

    layout_width
}

fn init_font(dom_font: &dom::Font) -> qt::Font {
    let mut font = qt::Font::new();

    font.set_family(&dom_font.family);

    let font_style = match dom_font.style {
        dom::FontStyle::Normal => qt::FontStyle::StyleNormal,
        dom::FontStyle::Italic => qt::FontStyle::StyleItalic,
        dom::FontStyle::Oblique => qt::FontStyle::StyleOblique,
    };
    font.set_style(font_style);

    if dom_font.variant == dom::FontVariant::SmallCaps {
        font.set_small_caps(true);
    }

    let font_weight = match dom_font.weight {
        dom::FontWeight::W100       => qt::FontWeight::Thin,
        dom::FontWeight::W200       => qt::FontWeight::ExtraLight,
        dom::FontWeight::W300       => qt::FontWeight::Light,
        dom::FontWeight::W400       => qt::FontWeight::Normal,
        dom::FontWeight::W500       => qt::FontWeight::Medium,
        dom::FontWeight::W600       => qt::FontWeight::DemiBold,
        dom::FontWeight::W700       => qt::FontWeight::Bold,
        dom::FontWeight::W800       => qt::FontWeight::ExtraBold,
        dom::FontWeight::W900       => qt::FontWeight::Black,
        dom::FontWeight::Normal     => qt::FontWeight::Normal,
        dom::FontWeight::Bold       => qt::FontWeight::Bold,
        dom::FontWeight::Bolder     => qt::FontWeight::ExtraBold,
        dom::FontWeight::Lighter    => qt::FontWeight::Light,
    };
    font.set_weight(font_weight);

    let font_stretch = match dom_font.stretch {
        dom::FontStretch::Normal         => qt::FontStretch::Unstretched,
        dom::FontStretch::Narrower |
        dom::FontStretch::Condensed      => qt::FontStretch::Condensed,
        dom::FontStretch::UltraCondensed => qt::FontStretch::UltraCondensed,
        dom::FontStretch::ExtraCondensed => qt::FontStretch::ExtraCondensed,
        dom::FontStretch::SemiCondensed  => qt::FontStretch::SemiCondensed,
        dom::FontStretch::SemiExpanded   => qt::FontStretch::SemiExpanded,
        dom::FontStretch::Wider |
        dom::FontStretch::Expanded       => qt::FontStretch::Expanded,
        dom::FontStretch::ExtraExpanded  => qt::FontStretch::ExtraExpanded,
        dom::FontStretch::UltraExpanded  => qt::FontStretch::UltraExpanded,
    };
    font.set_stretch(font_stretch);


    font.set_size(dom_font.size);

    font
}

fn draw_line(
    doc: &dom::Document,
    fill: &Option<dom::Fill>,
    stroke: &Option<dom::Stroke>,
    line_bbox: Rect,
    p: &qt::Painter,
) {
    let mut p_path = qt::PainterPath::new();

    p_path.move_to(line_bbox.x, line_bbox.y);
    p_path.line_to(line_bbox.x + line_bbox.w, line_bbox.y);
    p_path.line_to(line_bbox.x + line_bbox.w, line_bbox.y + line_bbox.h);
    p_path.line_to(line_bbox.x, line_bbox.y + line_bbox.h);
    p_path.close_path();

    fill::apply(doc, fill, p);
    stroke::apply(doc, stroke, p);

    p.draw_path(p_path);
}

fn process_text_anchor(x: f64, a: dom::TextAnchor, text_width: f64) -> f64 {
    match a {
        dom::TextAnchor::Start =>  x, // Nothing.
        dom::TextAnchor::Middle => x - text_width / 2.0,
        dom::TextAnchor::End =>    x - text_width,
    }
}
