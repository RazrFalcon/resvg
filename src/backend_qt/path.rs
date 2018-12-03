// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use qt;
use usvg;

// self
use super::prelude::*;
use super::{
    fill,
    stroke,
};


pub fn draw(
    tree: &usvg::Tree,
    path: &usvg::Path,
    opt: &Options,
    p: &mut qt::Painter,
) -> Rect {
    let mut p_path = qt::PainterPath::new();

    let fill_rule = if let Some(ref fill) = path.fill {
        fill.rule
    } else {
        usvg::FillRule::NonZero
    };

    convert_path(&path.segments, fill_rule, &mut p_path);

    let bbox = utils::path_bbox(&path.segments, None, &usvg::Transform::default());

    if path.visibility != usvg::Visibility::Visible {
        return bbox;
    }

    fill::apply(tree, &path.fill, opt, bbox, p);
    stroke::apply(tree, &path.stroke, opt, bbox, p);

    p.draw_path(&p_path);

    bbox
}

fn convert_path(
    list: &[usvg::PathSegment],
    rule: usvg::FillRule,
    p_path: &mut qt::PainterPath,
) {
    // Qt's QPainterPath automatically closes open subpaths if start and end positions are equal.
    // This is incorrect behaviour according to the SVG.
    // So we have to shift the last segment a bit, to prevent such behaviour.
    //
    // 'A closed path has coinciding start and end points.'
    // https://doc.qt.io/qt-5/qpainterpath.html#details

    let mut prev_mx = 0.0;
    let mut prev_my = 0.0;

    let len = list.len();
    let mut i = 0;
    while i < len {
        let ref seg1 = list[i];

        // Check that current segment is the last segment of the subpath.
        let is_last_subpath_seg = {
            if i == len - 1 {
                true
            } else {
                if let usvg::PathSegment::MoveTo{ .. } = list[i + 1] {
                    true
                } else {
                    false
                }
            }
        };

        match *seg1 {
            usvg::PathSegment::MoveTo { x, y } => {
                p_path.move_to(x, y);

                // Remember subpath start position.
                prev_mx = x;
                prev_my = y;
            }
            usvg::PathSegment::LineTo { mut x, y } => {
                if is_last_subpath_seg {
                    // No need to use fuzzy compare because Qt doesn't use it too.
                    if x == prev_mx && y == prev_my {
                        // We shift only the X coordinate because that's enough.
                        x -= 0.000001;
                    }
                }

                p_path.line_to(x, y);
            }
            usvg::PathSegment::CurveTo { x1, y1, x2, y2, mut x, y } => {
                if is_last_subpath_seg {
                    if x == prev_mx && y == prev_my {
                        x -= 0.000001;
                    }
                }

                p_path.curve_to(x1, y1, x2, y2, x, y);
            }
            usvg::PathSegment::ClosePath => {
                p_path.close_path();
            }
        }

        i += 1;
    }

    match rule {
        usvg::FillRule::NonZero => p_path.set_fill_rule(qt::FillRule::Winding),
        usvg::FillRule::EvenOdd => p_path.set_fill_rule(qt::FillRule::OddEven),
    }
}
