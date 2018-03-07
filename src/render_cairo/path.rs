// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use cairo;

// self

use tree;
use math::*;
use super::{
    fill,
    stroke,
};
use {
    utils,
    Options,
};


pub fn draw(
    rtree: &tree::RenderTree,
    path: &tree::Path,
    opt: &Options,
    cr: &cairo::Context,
) -> Rect {
    init_path(&path.segments, cr);

    let bbox = utils::path_bbox(&path.segments, None, &tree::Transform::default());

    fill::apply(rtree, &path.fill, opt, bbox, cr);
    if path.stroke.is_some() {
        cr.fill_preserve();

        stroke::apply(rtree, &path.stroke, opt, bbox, cr);
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
