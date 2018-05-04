// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use cairo;
use usvg;

// self
use geom::*;
use super::{
    fill,
    stroke,
};
use {
    utils,
    Options,
};


pub fn draw(
    tree: &usvg::Tree,
    path: &usvg::Path,
    opt: &Options,
    cr: &cairo::Context,
) -> Rect {
    init_path(&path.segments, cr);

    let bbox = utils::path_bbox(&path.segments, None, &usvg::Transform::default());

    fill::apply(tree, &path.fill, opt, bbox, cr);
    if path.stroke.is_some() {
        cr.fill_preserve();

        stroke::apply(tree, &path.stroke, opt, bbox, cr);
        cr.stroke();
    } else {
        cr.fill();
    }

    bbox
}

pub fn init_path(
    list: &[usvg::PathSegment],
    cr: &cairo::Context,
) {
    for seg in list {
        match *seg {
            usvg::PathSegment::MoveTo { x, y } => {
                cr.new_sub_path();
                cr.move_to(x, y);
            }
            usvg::PathSegment::LineTo { x, y } => {
                cr.line_to(x, y);
            }
            usvg::PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                cr.curve_to(x1, y1, x2, y2, x, y);
            }
            usvg::PathSegment::ClosePath => {
                cr.close_path();
            }
        }
    }
}
