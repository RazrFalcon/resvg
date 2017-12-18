// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use cairo;

use dom;

use math::{
    Rect,
};

use super::{
    fill,
    stroke,
};


pub fn draw(doc: &dom::Document, elem: &dom::Path, cr: &cairo::Context) {
    for seg in &elem.d {
        match *seg {
            dom::PathSegment::MoveTo { x, y } => {
                cr.new_sub_path();
                cr.move_to(x, y);
            }
            dom::PathSegment::LineTo { x, y } => {
                cr.line_to(x, y);
            }
            dom::PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                cr.curve_to(x1, y1, x2, y2, x, y);
            }
            dom::PathSegment::ClosePath => {
                cr.close_path();
            }
        }
    }

    let bbox = {
        // TODO: set_tolerance(1.0)
        let (mut x1, mut y1, mut x2, mut y2) = cr.fill_extents();

        if elem.stroke.is_some() {
            let (s_x1, s_y1, s_x2, s_y2) = cr.stroke_extents();

            // expand coordinates
            if s_x1 < x1 { x1 = s_x1; }
            if s_y1 < y1 { y1 = s_y1; }
            if s_x2 > x2 { x2 = s_x2; }
            if s_y2 > y2 { y2 = s_y2; }
        }

        Rect::new(x1, y1, x2 - x1, y2 - y1)
    };

    fill::apply(doc, &elem.fill, cr, &bbox);
    cr.fill_preserve();

    stroke::apply(doc, &elem.stroke, cr, &bbox);
    cr.stroke();
}
