// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom;

// self
use super::prelude::*;


pub fn convert(node: &svgdom::Node) -> Option<svgdom::Path> {
    match node.tag_id().unwrap() {
        EId::Rect =>     convert_rect(node),
        EId::Line =>     convert_line(node),
        EId::Polyline => convert_polyline(node),
        EId::Polygon =>  convert_polygon(node),
        EId::Circle =>   convert_circle(node),
        EId::Ellipse =>  convert_ellipse(node),
        _ => unreachable!(),
    }
}

fn convert_rect(node: &svgdom::Node) -> Option<svgdom::Path> {
    let attrs = node.attributes();

    // 'width' and 'height' attributes must be positive and non-zero.
    let width  = attrs.get_number_or(AId::Width, 0.0);
    let height = attrs.get_number_or(AId::Height, 0.0);
    if !(width > 0.0) {
        warn!("Rect '{}' has an invalid 'width' value. Skipped.", node.id());
        return None;
    }
    if !(height > 0.0) {
        warn!("Rect '{}' has an invalid 'height' value. Skipped.", node.id());
        return None;
    }


    let x = attrs.get_number_or(AId::X, 0.0);
    let y = attrs.get_number_or(AId::Y, 0.0);


    // Resolve rx, ry.
    let mut rx_opt = attrs.get_number(AId::Rx);
    let mut ry_opt = attrs.get_number(AId::Ry);

    // Remove negative values first.
    if let Some(v) = rx_opt {
        if v.is_sign_negative() {
            rx_opt = None;
        }
    }
    if let Some(v) = ry_opt {
        if v.is_sign_negative() {
            ry_opt = None;
        }
    }

    // Resolve.
    let (mut rx, mut ry) = match (rx_opt, ry_opt) {
        (None,     None)     => (0.0, 0.0),
        (Some(rx), None)     => (rx, rx),
        (None,     Some(ry)) => (ry, ry),
        (Some(rx), Some(ry)) => (rx, ry),
    };

    // Clamp rx/ry to the half of the width/height.
    //
    // Should be done only after resolving.
    if rx > width  / 2.0 { rx = width  / 2.0; }
    if ry > height / 2.0 { ry = height / 2.0; }


    // Conversion according to https://www.w3.org/TR/SVG11/shapes.html#RectElement
    let path = if rx.fuzzy_eq(&0.0) {
        svgdom::PathBuilder::with_capacity(5)
            .move_to(x, y)
            .hline_to(x + width)
            .vline_to(y + height)
            .hline_to(x)
            .close_path()
            .finalize()
    } else {
        svgdom::PathBuilder::with_capacity(9)
            .move_to(x + rx, y)
            .line_to(x + width - rx, y)
            .arc_to(rx, ry, 0.0, false, true, x + width, y + ry)
            .line_to(x + width, y + height - ry)
            .arc_to(rx, ry, 0.0, false, true, x + width - rx, y + height)
            .line_to(x + rx, y + height)
            .arc_to(rx, ry, 0.0, false, true, x, y + height - ry)
            .line_to(x, y + ry)
            .arc_to(rx, ry, 0.0, false, true, x + rx, y)
            .finalize()
    };

    Some(path)
}

fn convert_line(node: &svgdom::Node) -> Option<svgdom::Path> {
    let attrs = node.attributes();

    let x1 = attrs.get_number_or(AId::X1, 0.0);
    let y1 = attrs.get_number_or(AId::Y1, 0.0);
    let x2 = attrs.get_number_or(AId::X2, 0.0);
    let y2 = attrs.get_number_or(AId::Y2, 0.0);

    let path = svgdom::PathBuilder::new()
        .move_to(x1, y1)
        .line_to(x2, y2)
        .finalize();

    Some(path)
}

fn convert_polyline(node: &svgdom::Node) -> Option<svgdom::Path> {
    points_to_path(node, "Polyline")
}

fn convert_polygon(node: &svgdom::Node) -> Option<svgdom::Path> {
    if let Some(mut path) = points_to_path(node, "Polygon") {
        path.push(svgdom::PathSegment::ClosePath { abs: true } );
        Some(path)
    } else {
        None
    }
}

fn points_to_path(node: &svgdom::Node, eid: &str) -> Option<svgdom::Path> {
    let attrs = node.attributes();
    let points = if let Some(p) = attrs.get_points(AId::Points) {
        p
    } else {
        warn!("{} '{}' has an invalid 'points' value. Skipped.", eid, node.id());
        return None;
    };

    // 'polyline' and 'polygon' elements must contain at least 2 points.
    if points.len() < 2 {
        warn!("{} '{}' has less than 2 points. Skipped.", eid, node.id());
        return None;
    }

    let mut path = svgdom::Path::new();
    for (i, &(x, y)) in points.iter().enumerate() {
        let seg = if i == 0 {
            svgdom::PathSegment::MoveTo { abs: true, x, y }
        } else {
            svgdom::PathSegment::LineTo { abs: true, x, y }
        };
        path.push(seg);
    }

    Some(path)
}

fn convert_circle(node: &svgdom::Node) -> Option<svgdom::Path> {
    let attrs = node.attributes();

    let cx = attrs.get_number_or(AId::Cx, 0.0);
    let cy = attrs.get_number_or(AId::Cy, 0.0);
    let r  = attrs.get_number_or(AId::R, 0.0);

    if !(r > 0.0) {
        warn!("Circle '{}' has an invalid 'r' value. Skipped.", node.id());
        return None;
    }

    Some(ellipse_to_path(cx, cy, r, r))
}

fn convert_ellipse(node: &svgdom::Node) -> Option<svgdom::Path> {
    let attrs = node.attributes();

    let cx = attrs.get_number_or(AId::Cx, 0.0);
    let cy = attrs.get_number_or(AId::Cy, 0.0);
    let rx = attrs.get_number_or(AId::Rx, 0.0);
    let ry = attrs.get_number_or(AId::Ry, 0.0);

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

fn ellipse_to_path(cx: f64, cy: f64, rx: f64, ry: f64) -> svgdom::Path {
    svgdom::PathBuilder::with_capacity(6)
        .move_to(cx + rx, cy)
        .arc_to(rx, ry, 0.0, false, true, cx,      cy + ry)
        .arc_to(rx, ry, 0.0, false, true, cx - rx, cy)
        .arc_to(rx, ry, 0.0, false, true, cx,      cy - ry)
        .arc_to(rx, ry, 0.0, false, true, cx + rx, cy)
        .close_path()
        .finalize()
}
