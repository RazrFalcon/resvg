// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom;

// self
use crate::tree;
use crate::utils;
use super::prelude::*;
use super::{
    path,
    units,
};


pub fn convert(
    node: &svgdom::Node,
    state: &State,
) -> Option<Vec<tree::PathSegment>> {
    match node.tag_id()? {
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

pub fn convert_path(
    node: &svgdom::Node,
) -> Option<Vec<tree::PathSegment>> {
    if let Some(AValue::Path(path)) = node.attributes().get_value(AId::D).cloned() {
        let new_path = super::path::convert_path(path);
        if new_path.len() >= 2 {
            return Some(new_path);
        }
    }

    None
}

pub fn convert_rect(
    node: &svgdom::Node,
    state: &State,
) -> Option<Vec<tree::PathSegment>> {
    let attrs = node.attributes();

    // 'width' and 'height' attributes must be positive and non-zero.
    let width  = node.convert_user_length(AId::Width, state, Length::zero());
    let height = node.convert_user_length(AId::Height, state, Length::zero());
    if !(width > 0.0) {
        warn!("Rect '{}' has an invalid 'width' value. Skipped.", node.id());
        return None;
    }
    if !(height > 0.0) {
        warn!("Rect '{}' has an invalid 'height' value. Skipped.", node.id());
        return None;
    }


    let x = node.convert_user_length(AId::X, state, Length::zero());
    let y = node.convert_user_length(AId::Y, state, Length::zero());


    // Resolve rx, ry.
    let mut rx_opt = attrs.get_length(AId::Rx);
    let mut ry_opt = attrs.get_length(AId::Ry);

    // Remove negative values first.
    if let Some(v) = rx_opt {
        if v.num.is_sign_negative() {
            rx_opt = None;
        }
    }
    if let Some(v) = ry_opt {
        if v.num.is_sign_negative() {
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

    let mut rx = units::convert_length(rx, node, AId::Rx, tree::Units::UserSpaceOnUse, state);
    let mut ry = units::convert_length(ry, node, AId::Ry, tree::Units::UserSpaceOnUse, state);

    // Clamp rx/ry to the half of the width/height.
    //
    // Should be done only after resolving.
    if rx > width  / 2.0 { rx = width  / 2.0; }
    if ry > height / 2.0 { ry = height / 2.0; }


    // Conversion according to https://www.w3.org/TR/SVG11/shapes.html#RectElement
    let path = if rx.fuzzy_eq(&0.0) {
        utils::rect_to_path(Rect::new(x, y, width, height)?)
    } else {
        let p = svgdom::PathBuilder::with_capacity(9)
            .move_to(x + rx, y)
            .line_to(x + width - rx, y)
            .arc_to(rx, ry, 0.0, false, true, x + width, y + ry)
            .line_to(x + width, y + height - ry)
            .arc_to(rx, ry, 0.0, false, true, x + width - rx, y + height)
            .line_to(x + rx, y + height)
            .arc_to(rx, ry, 0.0, false, true, x, y + height - ry)
            .line_to(x, y + ry)
            .arc_to(rx, ry, 0.0, false, true, x + rx, y)
            .finalize();

        path::convert_path(p)
    };

    Some(path)
}

pub fn convert_line(
    node: &svgdom::Node,
    state: &State,
) -> Option<Vec<tree::PathSegment>> {
    let x1 = node.convert_user_length(AId::X1, state, Length::zero());
    let y1 = node.convert_user_length(AId::Y1, state, Length::zero());
    let x2 = node.convert_user_length(AId::X2, state, Length::zero());
    let y2 = node.convert_user_length(AId::Y2, state, Length::zero());

    let path = vec![
        tree::PathSegment::MoveTo { x: x1, y: y1 },
        tree::PathSegment::LineTo { x: x2, y: y2 },
    ];

    Some(path)
}

pub fn convert_polyline(
    node: &svgdom::Node,
) -> Option<Vec<tree::PathSegment>> {
    points_to_path(node, "Polyline")
}

pub fn convert_polygon(
    node: &svgdom::Node,
) -> Option<Vec<tree::PathSegment>> {
    if let Some(mut path) = points_to_path(node, "Polygon") {
        path.push(tree::PathSegment::ClosePath);
        Some(path)
    } else {
        None
    }
}

fn points_to_path(
    node: &svgdom::Node,
    eid: &str,
) -> Option<Vec<tree::PathSegment>> {
    let attrs = node.attributes();

    let points = match attrs.get_value(AId::Points) {
        Some(&AValue::Points(ref points)) => {
            points
        }
        _ => {
            warn!("{} '{}' has an invalid 'points' value. Skipped.", eid, node.id());
            return None;
        }
    };

    // 'polyline' and 'polygon' elements must contain at least 2 points.
    if points.len() < 2 {
        warn!("{} '{}' has less than 2 points. Skipped.", eid, node.id());
        return None;
    }

    let mut path = Vec::with_capacity(points.len());
    for (i, &(x, y)) in points.iter().enumerate() {
        let seg = if i == 0 {
            tree::PathSegment::MoveTo { x, y }
        } else {
            tree::PathSegment::LineTo { x, y }
        };
        path.push(seg);
    }

    Some(path)
}

pub fn convert_circle(
    node: &svgdom::Node,
    state: &State,
) -> Option<Vec<tree::PathSegment>> {
    let cx = node.convert_user_length(AId::Cx, state, Length::zero());
    let cy = node.convert_user_length(AId::Cy, state, Length::zero());
    let r  = node.convert_user_length(AId::R,  state, Length::zero());

    if !(r > 0.0) {
        warn!("Circle '{}' has an invalid 'r' value. Skipped.", node.id());
        return None;
    }

    Some(ellipse_to_path(cx, cy, r, r))
}

pub fn convert_ellipse(
    node: &svgdom::Node,
    state: &State,
) -> Option<Vec<tree::PathSegment>> {
    let cx = node.convert_user_length(AId::Cx, state, Length::zero());
    let cy = node.convert_user_length(AId::Cy, state, Length::zero());
    let rx = node.convert_user_length(AId::Rx, state, Length::zero());
    let ry = node.convert_user_length(AId::Ry, state, Length::zero());

    if !(rx > 0.0) {
        warn!("Ellipse '{}' has an invalid 'rx' value. Skipped.", node.id());
        return None;
    }

    if !(ry > 0.0) {
        warn!("Ellipse '{}' has an invalid 'ry' value. Skipped.", node.id());
        return None;
    }

    Some(ellipse_to_path(cx, cy, rx, ry))
}

fn ellipse_to_path(cx: f64, cy: f64, rx: f64, ry: f64) -> Vec<tree::PathSegment> {
    path::convert_path(svgdom::PathBuilder::with_capacity(6)
        .move_to(cx + rx, cy)
        .arc_to(rx, ry, 0.0, false, true, cx,      cy + ry)
        .arc_to(rx, ry, 0.0, false, true, cx - rx, cy)
        .arc_to(rx, ry, 0.0, false, true, cx,      cy - ry)
        .arc_to(rx, ry, 0.0, false, true, cx + rx, cy)
        .close_path()
        .finalize())
}
