// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use qt;
use usvg::tree;

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
    tree: &tree::Tree,
    path: &tree::Path,
    opt: &Options,
    p: &qt::Painter,
) -> Rect {
    let mut p_path = qt::PainterPath::new();

    let fill_rule = if let Some(fill) = path.fill {
        fill.rule
    } else {
        tree::FillRule::NonZero
    };

    convert_path(&path.segments, fill_rule, &mut p_path);

    let bbox = utils::path_bbox(&path.segments, None, &tree::Transform::default());

    fill::apply(tree, &path.fill, opt, bbox, p);
    stroke::apply(tree, &path.stroke, opt, bbox, p);

    p.draw_path(&p_path);

    bbox
}

pub fn convert_path(
    list: &[tree::PathSegment],
    rule: tree::FillRule,
    p_path: &mut qt::PainterPath,
) {
    // Qt's QPainterPath automatically closes open subpaths if start and end positions are equal.
    // This is incorrect behaviour according to SVG.
    // So we have to shift the last segment a bit, to disable it.
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
                if let tree::PathSegment::MoveTo{ .. } = list[i + 1] {
                    true
                } else {
                    false
                }
            }
        };

        match *seg1 {
            tree::PathSegment::MoveTo { x, y } => {
                p_path.move_to(x, y);

                // Remember subpath start position.
                prev_mx = x;
                prev_my = y;
            }
            tree::PathSegment::LineTo { mut x, y } => {
                if is_last_subpath_seg {
                    // No need to use fuzzy compare because Qt doesn't use it too.
                    if x == prev_mx && y == prev_my {
                        // We shift only the X coordinate because that's enough.
                        x -= 0.000001;
                    }
                }

                p_path.line_to(x, y);
            }
            tree::PathSegment::CurveTo { x1, y1, x2, y2, mut x, y } => {
                if is_last_subpath_seg {
                    if x == prev_mx && y == prev_my {
                        x -= 0.000001;
                    }
                }

                p_path.curve_to(x1, y1, x2, y2, x, y);
            }
            tree::PathSegment::ClosePath => {
                p_path.close_path();
            }
        }

        i += 1;
    }

    // a-fill-rule-001.svg
    // a-fill-rule-002.svg
    match rule {
        tree::FillRule::NonZero => p_path.set_fill_rule(qt::FillRule::WindingFill),
        tree::FillRule::EvenOdd => p_path.set_fill_rule(qt::FillRule::OddEvenFill),
    }
}
