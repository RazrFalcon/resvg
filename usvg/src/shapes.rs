// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::rc::Rc;

use svgtypes::Length;

use crate::svgtree::{self, EId, AId};
use crate::{converter, units, FuzzyEq, IsValidLength, PathData, PathSegment, Rect, SharedPathData, Units};

pub(crate) fn convert(
    node: svgtree::Node,
    state: &converter::State,
) -> Option<SharedPathData> {
    match node.tag_name()? {
        EId::Rect => convert_rect(node, state),
        EId::Circle => convert_circle(node, state),
        EId::Ellipse => convert_ellipse(node, state),
        EId::Line => convert_line(node, state),
        EId::Polyline => convert_polyline(node),
        EId::Polygon => convert_polygon(node),
        EId::Path => convert_path(node),
        _ => None,
    }
}

fn convert_path(node: svgtree::Node) -> Option<SharedPathData> {
    node.attribute::<SharedPathData>(AId::D)
}

fn convert_rect(
    node: svgtree::Node,
    state: &converter::State,
) -> Option<SharedPathData> {
    // 'width' and 'height' attributes must be positive and non-zero.
    let width  = node.convert_user_length(AId::Width, state, Length::zero());
    let height = node.convert_user_length(AId::Height, state, Length::zero());
    if !width.is_valid_length() {
        log::warn!("Rect '{}' has an invalid 'width' value. Skipped.", node.element_id());
        return None;
    }
    if !height.is_valid_length() {
        log::warn!("Rect '{}' has an invalid 'height' value. Skipped.", node.element_id());
        return None;
    }

    let x = node.convert_user_length(AId::X, state, Length::zero());
    let y = node.convert_user_length(AId::Y, state, Length::zero());

    let (mut rx, mut ry) = resolve_rx_ry(node, state);

    // Clamp rx/ry to the half of the width/height.
    //
    // Should be done only after resolving.
    if rx > width  / 2.0 { rx = width  / 2.0; }
    if ry > height / 2.0 { ry = height / 2.0; }

    // Conversion according to https://www.w3.org/TR/SVG11/shapes.html#RectElement
    let path = if rx.fuzzy_eq(&0.0) {
        PathData::from_rect(Rect::new(x, y, width, height)?)
    } else {
        let mut p = PathData::with_capacity(16);
        p.push_move_to(x + rx, y);

        p.push_line_to(x + width - rx, y);
        p.push_arc_to(rx, ry, 0.0, false, true, x + width, y + ry);

        p.push_line_to(x + width, y + height - ry);
        p.push_arc_to(rx, ry, 0.0, false, true, x + width - rx, y + height);

        p.push_line_to(x + rx, y + height);
        p.push_arc_to(rx, ry, 0.0, false, true, x, y + height - ry);

        p.push_line_to(x, y + ry);
        p.push_arc_to(rx, ry, 0.0, false, true, x + rx, y);

        p.push_close_path();

        p
    };

    Some(Rc::new(path))
}

fn resolve_rx_ry(
    node: svgtree::Node,
    state: &converter::State,
) -> (f64, f64) {
    let mut rx_opt = node.attribute::<Length>(AId::Rx);
    let mut ry_opt = node.attribute::<Length>(AId::Ry);

    // Remove negative values first.
    if let Some(v) = rx_opt {
        if v.number.is_sign_negative() {
            rx_opt = None;
        }
    }
    if let Some(v) = ry_opt {
        if v.number.is_sign_negative() {
            ry_opt = None;
        }
    }

    // Resolve.
    let (rx, ry) = match (rx_opt, ry_opt) {
        (None,     None)     => (Length::zero(), Length::zero()),
        (Some(rx), None)     => (rx, rx),
        (None,     Some(ry)) => (ry, ry),
        (Some(rx), Some(ry)) => (rx, ry),
    };

    let rx = units::convert_length(rx, node, AId::Rx, Units::UserSpaceOnUse, state);
    let ry = units::convert_length(ry, node, AId::Ry, Units::UserSpaceOnUse, state);

    (rx, ry)
}

fn convert_line(
    node: svgtree::Node,
    state: &converter::State,
) -> Option<SharedPathData> {
    let x1 = node.convert_user_length(AId::X1, state, Length::zero());
    let y1 = node.convert_user_length(AId::Y1, state, Length::zero());
    let x2 = node.convert_user_length(AId::X2, state, Length::zero());
    let y2 = node.convert_user_length(AId::Y2, state, Length::zero());

    let mut path = PathData::new();
    path.push_move_to(x1, y1);
    path.push_line_to(x2, y2);
    Some(Rc::new(path))
}

fn convert_polyline(node: svgtree::Node) -> Option<SharedPathData> {
    points_to_path(node, "Polyline").map(Rc::new)
}

fn convert_polygon(node: svgtree::Node) -> Option<SharedPathData> {
    if let Some(mut path) = points_to_path(node, "Polygon") {
        path.push(PathSegment::ClosePath);
        Some(Rc::new(path))
    } else {
        None
    }
}

fn points_to_path(
    node: svgtree::Node,
    eid: &str,
) -> Option<PathData> {
    use svgtypes::PointsParser;

    let mut path = PathData::new();
    match node.attribute::<&str>(AId::Points) {
        Some(text) => {
            for (x, y) in PointsParser::from(text) {
                if path.is_empty() {
                    path.push_move_to(x, y);
                } else {
                    path.push_line_to(x, y);
                }
            }
        }
        _ => {
            log::warn!("{} '{}' has an invalid 'points' value. Skipped.", eid, node.element_id());
            return None;
        }
    };

    // 'polyline' and 'polygon' elements must contain at least 2 points.
    if path.len() < 2 {
        log::warn!("{} '{}' has less than 2 points. Skipped.", eid, node.element_id());
        return None;
    }

    Some(path)
}

fn convert_circle(
    node: svgtree::Node,
    state: &converter::State,
) -> Option<SharedPathData> {
    let cx = node.convert_user_length(AId::Cx, state, Length::zero());
    let cy = node.convert_user_length(AId::Cy, state, Length::zero());
    let r  = node.convert_user_length(AId::R,  state, Length::zero());

    if !r.is_valid_length() {
        log::warn!("Circle '{}' has an invalid 'r' value. Skipped.", node.element_id());
        return None;
    }

    Some(Rc::new(ellipse_to_path(cx, cy, r, r)))
}

fn convert_ellipse(
    node: svgtree::Node,
    state: &converter::State,
) -> Option<SharedPathData> {
    let cx = node.convert_user_length(AId::Cx, state, Length::zero());
    let cy = node.convert_user_length(AId::Cy, state, Length::zero());
    let (rx, ry) = resolve_rx_ry(node, state);

    if !rx.is_valid_length() {
        log::warn!("Ellipse '{}' has an invalid 'rx' value. Skipped.", node.element_id());
        return None;
    }

    if !ry.is_valid_length() {
        log::warn!("Ellipse '{}' has an invalid 'ry' value. Skipped.", node.element_id());
        return None;
    }

    Some(Rc::new(ellipse_to_path(cx, cy, rx, ry)))
}

fn ellipse_to_path(cx: f64, cy: f64, rx: f64, ry: f64) -> PathData {
    let mut p = PathData::with_capacity(6);
    p.push_move_to(cx + rx, cy);
    p.push_arc_to(rx, ry, 0.0, false, true, cx,      cy + ry);
    p.push_arc_to(rx, ry, 0.0, false, true, cx - rx, cy     );
    p.push_arc_to(rx, ry, 0.0, false, true, cx,      cy - ry);
    p.push_arc_to(rx, ry, 0.0, false, true, cx + rx, cy     );
    p.push_close_path();
    p
}
