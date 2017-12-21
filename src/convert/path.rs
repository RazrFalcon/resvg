// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::f64;

use svgdom;
use dom;

use svgdom::types::path::*;
use svgdom::types::{
    FuzzyEq
};

use short::{
    AId,
};

use traits::{
    GetValue,
};

use math::{
    f64_bound,
};

use {
    Result,
};

use super::{
    fill,
    stroke,
};


pub fn convert(
    defs: &[dom::RefElement],
    node: &svgdom::Node,
    d: Path,
) -> Result<dom::Element> {
    let attrs = node.attributes();

    let fill = fill::convert(defs, &attrs);
    let stroke = stroke::convert(defs, &attrs);
    let d = convert_path(d);

    let ts = attrs.get_transform(AId::Transform).unwrap_or_default();

    let elem = dom::Element {
        id: node.id().clone(),
        kind: dom::ElementKind::Path(dom::Path {
            fill: fill,
            stroke: stroke,
            d: d,
        }),
        transform: ts,
    };

    Ok(elem)
}

fn convert_path(mut path: Path) -> Vec<dom::PathSegment> {
    let mut new_path = Vec::with_capacity(path.d.len());

    path.conv_to_absolute();

    // Previous MoveTo coordinates.
    let mut pmx = 0.0;
    let mut pmy = 0.0;

    // Previous coordinates.
    let mut px = 0.0;
    let mut py = 0.0;

    // Previous SmoothQuadratic coordinates.
    let mut ptx = None;
    let mut pty = None;

    for seg in path.d.iter() {
        match *seg.data() {
            SegmentData::MoveTo { x, y } => {
                new_path.push(dom::PathSegment::MoveTo { x, y });
            }
            SegmentData::LineTo { x, y } => {
                new_path.push(dom::PathSegment::LineTo { x, y });
            }
            SegmentData::HorizontalLineTo { x } => {
                new_path.push(dom::PathSegment::LineTo { x, y: py });
            }
            SegmentData::VerticalLineTo { y } => {
                new_path.push(dom::PathSegment::LineTo { x: px, y });
            }
            SegmentData::CurveTo { x1, y1, x2, y2, x, y } => {
                new_path.push(dom::PathSegment::CurveTo { x1, y1, x2, y2, x, y });
            }
            SegmentData::SmoothCurveTo { x2, y2, x, y } => {
                // 'The first control point is assumed to be the reflection of the second control
                // point on the previous command relative to the current point.
                // (If there is no previous command or if the previous command
                // was not an C, c, S or s, assume the first control point is
                // coincident with the current point.)'
                let new_x1;
                let new_y1;
                if let Some(seg) = new_path.last().cloned() {
                    match seg {
                        dom::PathSegment::CurveTo { x2, y2, x, y, .. } => {
                            new_x1 = x * 2.0 - x2;
                            new_y1 = y * 2.0 - y2;
                        }
                        _ => {
                            new_x1 = px;
                            new_y1 = py;
                        }
                    }

                    new_path.push(dom::PathSegment::CurveTo { x1: new_x1, y1: new_y1, x2, y2, x, y });
                }
            }
            SegmentData::Quadratic { x1, y1, x, y } => {
                // Remember last control point.
                ptx = Some(x * 2.0 - x1);
                pty = Some(y * 2.0 - y1);

                new_path.push(quad_to_curve(px, py, x1, y1, x, y));
            }
            SegmentData::SmoothQuadratic { x, y } => {
                // 'The control point is assumed to be the reflection of
                // the control point on the previous command relative to
                // the current point. (If there is no previous command or
                // if the previous command was not a Q, q, T or t, assume
                // the control point is coincident with the current point.)'
                let new_x1;
                let new_y1;
                if let (Some(tx), Some(ty)) = (ptx, pty) {
                    new_x1 = tx;
                    new_y1 = ty;

                    // Reset control point.
                    ptx = Some(x * 2.0 - tx);
                    pty = Some(y * 2.0 - ty);
                } else {
                    new_x1 = px;
                    new_y1 = py;
                }

                new_path.push(quad_to_curve(px, py, new_x1, new_y1, x, y));
            }
            SegmentData::EllipticalArc { rx, ry, x_axis_rotation, large_arc, sweep, x, y } => {
                arc_to_curve(&mut new_path, px, py, rx, ry, x_axis_rotation, large_arc, sweep, x, y)
            }
            SegmentData::ClosePath => {
                new_path.push(dom::PathSegment::ClosePath);
            }
        }

        // Remember last position.
        if let Some(seg) = new_path.last() {
            match *seg {
                dom::PathSegment::MoveTo { x, y } => {
                    px = x;
                    py = y;
                    pmx = x;
                    pmy = y;
                }
                dom::PathSegment::LineTo { x, y } => {
                    px = x;
                    py = y;
                }
                dom::PathSegment::CurveTo { x, y, .. } => {
                    px = x;
                    py = y;
                }
                dom::PathSegment::ClosePath => {
                    // ClosePath moves us to the last MoveTo coordinate,
                    // not previous.
                    px = pmx;
                    py = pmy;
                }
            }
        }
    }

    new_path
}

fn quad_to_curve(
    px: f64,
    py: f64,
    x1: f64,
    y1: f64,
    x: f64,
    y: f64
) -> dom::PathSegment {
    let nx1 = (px + 2.0 * x1) / 3.0;
    let ny1 = (py + 2.0 * y1) / 3.0;

    let nx2 = (x + 2.0 * x1) / 3.0;
    let ny2 = (y + 2.0 * y1) / 3.0;

    dom::PathSegment::CurveTo { x1: nx1, y1: ny1, x2: nx2, y2: ny2, x, y }
}

// http://www.w3.org/TR/SVG/implnote.html#ArcImplementationNotes
// Based on librsvg implementation.
fn arc_to_curve(
    path: &mut Vec<dom::PathSegment>,
    x1: f64,
    y1: f64,
    mut rx: f64,
    mut ry: f64,
    x_axis_rotation: f64,
    large_arc_flag: bool,
    sweep_flag: bool,
    x2: f64,
    y2: f64
) {
    if x1.fuzzy_eq(&x2) && y1.fuzzy_eq(&y2) {
        return;
    }

    // X-axis
    let f = x_axis_rotation * f64::consts::PI / 180.0;
    let sinf = f.sin();
    let cosf = f.cos();

    rx = rx.abs();
    ry = ry.abs();

    if rx < f64::EPSILON || ry < f64::EPSILON {
        path.push(dom::PathSegment::LineTo { x: x2, y: y2 });
        return;
    }

    let k1 = (x1 - x2) / 2.0;
    let k2 = (y1 - y2) / 2.0;

    let x1_ = cosf * k1 + sinf * k2;
    let y1_ = -sinf * k1 + cosf * k2;

    let gamma = (x1_ * x1_) / (rx * rx) + (y1_ * y1_) / (ry * ry);
    if gamma > 1.0 {
        rx *= gamma.sqrt();
        ry *= gamma.sqrt();
    }

    // compute the center
    let k1 = rx * rx * y1_ * y1_ + ry * ry * x1_ * x1_;
    if k1.fuzzy_eq(&0.0) {
        return;
    }

    let mut k1 = ((rx * rx * ry * ry) / k1 - 1.0).abs().sqrt();
    if sweep_flag == large_arc_flag {
        k1 = -k1;
    }

    let cx_ = k1 * rx * y1_ / ry;
    let cy_ = -k1 * ry * x1_ / rx;

    let cx = cosf * cx_ - sinf * cy_ + (x1 + x2) / 2.0;
    let cy = sinf * cx_ + cosf * cy_ + (y1 + y2) / 2.0;

    // compute start angle
    let k1 = (x1_ - cx_) / rx;
    let k2 = (y1_ - cy_) / ry;
    let k3 = (-x1_ - cx_) / rx;
    let k4 = (-y1_ - cy_) / ry;

    let mut k5 = (k1 * k1 + k2 * k2).abs().sqrt();
    if k5.fuzzy_eq(&0.0) {
        return;
    }

    k5 = k1 / k5;
    k5 = f64_bound(-1.0, k5, 1.0);
    let mut theta1 = k5.acos();
    if k2 < 0.0 {
        theta1 = -theta1;
    }

    // compute delta_theta
    k5 = ((k1 * k1 + k2 * k2) * (k3 * k3 + k4 * k4)).abs().sqrt();
    if k5.fuzzy_eq(&0.0) {
        return;
    }

    k5 = (k1 * k3 + k2 * k4) / k5;
    k5 = f64_bound(-1.0, k5, 1.0);
    let mut delta_theta = k5.acos();
    if k1 * k4 - k3 * k2 < 0.0 {
        delta_theta = -delta_theta;
    }

    if sweep_flag && delta_theta < 0.0 {
        delta_theta += f64::consts::PI * 2.0;
    } else if !sweep_flag && delta_theta > 0.0 {
        delta_theta -= f64::consts::PI * 2.0;
    }

    // gen curves
    let n_segs = (delta_theta / (f64::consts::PI * 0.5 + 0.001)).abs().ceil();

    for i in 0..(n_segs as usize) {
        _arc_to_curve(
            path,
            cx, cy,
            theta1 + i as f64 * delta_theta / n_segs,
            theta1 + (i as f64 + 1.0) * delta_theta / n_segs,
            rx, ry,
            x_axis_rotation
        );
    }
}

fn _arc_to_curve(
    path: &mut Vec<dom::PathSegment>,
    xc: f64,
    yc: f64,
    th0: f64,
    th1: f64,
    rx: f64,
    ry: f64,
    x_axis_rotation: f64
) {
    let f = x_axis_rotation * f64::consts::PI / 180.0;
    let sinf = f.sin();
    let cosf = f.cos();

    let th_half = 0.5 * (th1 - th0);
    let t = (8.0 / 3.0) * (th_half * 0.5).sin() * (th_half * 0.5).sin() / th_half.sin();
    let x1 = rx * (th0.cos() - t * th0.sin());
    let y1 = ry * (th0.sin() + t * th0.cos());
    let x3 = rx * th1.cos();
    let y3 = ry * th1.sin();
    let x2 = x3 + rx * ( t * th1.sin());
    let y2 = y3 + ry * (-t * th1.cos());

    let seg = dom::PathSegment::CurveTo {
        x1: xc + cosf * x1 - sinf * y1, y1: yc + sinf * x1 + cosf * y1,
        x2: xc + cosf * x2 - sinf * y2, y2: yc + sinf * x2 + cosf * y2,
        x:  xc + cosf * x3 - sinf * y3, y:  yc + sinf * x3 + cosf * y3
    };
    path.push(seg);
}
