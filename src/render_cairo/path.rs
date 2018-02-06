// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use cairo;

// self

use tree;
use math::{
    Rect,
};
use super::{
    fill,
    stroke,
};


pub fn draw(
    rtree: &tree::RenderTree,
    elem: &tree::Path,
    cr: &cairo::Context,
) -> Rect {
    init_path(&elem.d, cr);

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

    fill::apply(rtree, &elem.fill, cr, &bbox);
    if elem.stroke.is_some() {
        cr.fill_preserve();

        stroke::apply(rtree, &elem.stroke, cr, &bbox);
        cr.stroke();
    } else {
        cr.fill();
    }

    bbox
}

pub fn init_path(
    list: &[tree::PathSegment],
    cr: &cairo::Context,
) {
    for seg in list {
        match *seg {
            tree::PathSegment::MoveTo { x, y } => {
                cr.new_sub_path();
                cr.move_to(x, y);
            }
            tree::PathSegment::LineTo { x, y } => {
                cr.line_to(x, y);
            }
            tree::PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                cr.curve_to(x1, y1, x2, y2, x, y);
            }
            tree::PathSegment::ClosePath => {
                cr.close_path();
            }
        }
    }
}
