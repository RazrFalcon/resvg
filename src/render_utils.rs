// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use tree;

use math::{
    Size,
    Rect,
};

use {
    FitTo,
};


pub fn fit_to(size: &Size, fit: FitTo) -> Size {
    match fit {
        FitTo::Original => {
            *size
        }
        FitTo::Width(w) => {
            let h = (w as f64 * size.h / size.w).ceil();
            Size::new(w as f64, h)
        }
        FitTo::Height(h) => {
            let w = (h as f64 * size.w / size.h).ceil();
            Size::new(w, h as f64)
        }
        FitTo::Zoom(z) => {
            Size::new(size.w * z as f64, size.h * z as f64)
        }
    }
}

pub fn view_box_transform(view_box: &Rect, img_view: &Rect) -> (f64, f64, f64, f64) {
    let sx = img_view.w / view_box.w;
    let sy = img_view.h / view_box.h;

    // Use proportional scaling for now.
    let s = if sx > sy { sy } else { sx };

    let dx = -view_box.x * s + (img_view.w - view_box.w * s) / 2.0 + img_view.x;
    let dy = -view_box.y * s + (img_view.h - view_box.h * s) / 2.0 + img_view.y;

    (dx, dy, s, s)
}

pub fn process_text_anchor(x: f64, a: tree::TextAnchor, text_width: f64) -> f64 {
    match a {
        tree::TextAnchor::Start =>  x, // Nothing.
        tree::TextAnchor::Middle => x - text_width / 2.0,
        tree::TextAnchor::End =>    x - text_width,
    }
}
