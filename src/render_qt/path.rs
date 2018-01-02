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
    elem: &dom::Path,
    p: &qt::Painter,
) -> Rect {
    let mut p_path = qt::PainterPath::new();

    let fill_rule = if let Some(fill) = elem.fill {
        fill.rule
    } else {
        dom::FillRule::NonZero
    };

    convert_path(&elem.d, fill_rule, &mut p_path);

    let bbox: Rect = p_path.bounding_box().into();

    fill::apply(doc, &elem.fill, p, &bbox);
    stroke::apply(doc, &elem.stroke, p, &bbox);

    p.draw_path(&p_path);

    bbox
}

pub fn convert_path(
    list: &[dom::PathSegment],
    rule: dom::FillRule,
    p_path: &mut qt::PainterPath,
) {
    // Qt's QPainterPath automatically closes open subpaths if start and end positions are equal.
    // This is incorrect behaviour according to SVG.
    // So we have to shift the last segment a bit, to disable it.
    //
    // 'A closed path has coinciding start and end points.'
    // https://doc.qt.io/qt-5/qpainterpath.html#details
    //
    // Tested by:
    //  paths-data-10-t.svg
    //  painting-stroke-10-t.svg
    //  painting-control-04-f.svg
    //  shapes-polyline-01-t.svg
    //  shapes-polyline-02-t.svg

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
                if let dom::PathSegment::MoveTo{ .. } = list[i + 1] {
                    true
                } else {
                    false
                }
            }
        };

        match *seg1 {
            dom::PathSegment::MoveTo { x, y } => {
                p_path.move_to(x, y);

                // Remember subpath start position.
                prev_mx = x;
                prev_my = y;
            }
            dom::PathSegment::LineTo { mut x, y } => {
                if is_last_subpath_seg {
                    // No need to use fuzzy compare because Qt doesn't use it too.
                    if x == prev_mx && y == prev_my {
                        // We shift only the X coordinate because that's enough.
                        x -= 0.000001;
                    }
                }

                p_path.line_to(x, y);
            }
            dom::PathSegment::CurveTo { x1, y1, x2, y2, mut x, y } => {
                if is_last_subpath_seg {
                    if x == prev_mx && y == prev_my {
                        x -= 0.000001;
                    }
                }

                p_path.curve_to(x1, y1, x2, y2, x, y);
            }
            dom::PathSegment::ClosePath => {
                p_path.close_path();
            }
        }

        i += 1;
    }

    match rule {
        dom::FillRule::NonZero => p_path.set_fill_rule(qt::FillRule::WindingFill),
        dom::FillRule::EvenOdd => p_path.set_fill_rule(qt::FillRule::OddEvenFill),
    }
}
