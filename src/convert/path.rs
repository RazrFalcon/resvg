// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::f64;

// external
use svgdom::path::*;
use svgdom;
use lyon_geom;

// self
use tree::prelude::*;
use short::{
    AId,
};
use traits::{
    GetValue,
};

use super::{
    fill,
    stroke,
};


pub(super) fn convert(
    node: &svgdom::Node,
    d: Path,
    parent: tree::NodeId,
    rtree: &mut tree::RenderTree,
) {
    let attrs = node.attributes();

    let fill = fill::convert(rtree, &attrs);
    let stroke = stroke::convert(rtree, &attrs);
    let d = convert_path(d);
    let transform = attrs.get_transform(AId::Transform).unwrap_or_default();

    rtree.append_child(parent, tree::NodeKind::Path(tree::Path {
        id: node.id().clone(),
        transform,
        fill,
        stroke,
        segments: d,
    }));
}

fn convert_path(mut path: Path) -> Vec<tree::PathSegment> {
    let mut new_path = Vec::with_capacity(path.len());

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

    for seg in path.iter() {
        match *seg.data() {
            SegmentData::MoveTo { x, y } => {
                new_path.push(tree::PathSegment::MoveTo { x, y });
            }
            SegmentData::LineTo { x, y } => {
                new_path.push(tree::PathSegment::LineTo { x, y });
            }
            SegmentData::HorizontalLineTo { x } => {
                new_path.push(tree::PathSegment::LineTo { x, y: py });
            }
            SegmentData::VerticalLineTo { y } => {
                new_path.push(tree::PathSegment::LineTo { x: px, y });
            }
            SegmentData::CurveTo { x1, y1, x2, y2, x, y } => {
                new_path.push(tree::PathSegment::CurveTo { x1, y1, x2, y2, x, y });
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
                        tree::PathSegment::CurveTo { x2, y2, x, y, .. } => {
                            new_x1 = x * 2.0 - x2;
                            new_y1 = y * 2.0 - y2;
                        }
                        _ => {
                            new_x1 = px;
                            new_y1 = py;
                        }
                    }

                    new_path.push(tree::PathSegment::CurveTo { x1: new_x1, y1: new_y1, x2, y2, x, y });
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
                let arc = lyon_geom::SvgArc {
                    from: [px as f32, py as f32].into(),
                    to: [x as f32, y as f32].into(),
                    radii: [rx as f32, ry as f32].into(),
                    x_rotation: lyon_geom::math::Angle::degrees(x_axis_rotation as f32),
                    flags: lyon_geom::ArcFlags { large_arc, sweep },
                };

                arc.for_each_quadratic_bezier(&mut |quad| {
                    let cubic = quad.to_cubic();
                    let curve = tree::PathSegment::CurveTo {
                        x1: cubic.ctrl1.x as f64, y1: cubic.ctrl1.y as f64,
                        x2: cubic.ctrl2.x as f64, y2: cubic.ctrl2.y as f64,
                        x:  cubic.to.x as f64,    y:  cubic.to.y as f64,
                    };

                    new_path.push(curve);
                });
            }
            SegmentData::ClosePath => {
                new_path.push(tree::PathSegment::ClosePath);
            }
        }

        // Remember last position.
        if let Some(seg) = new_path.last() {
            match *seg {
                tree::PathSegment::MoveTo { x, y } => {
                    px = x;
                    py = y;
                    pmx = x;
                    pmy = y;
                }
                tree::PathSegment::LineTo { x, y } => {
                    px = x;
                    py = y;
                }
                tree::PathSegment::CurveTo { x, y, .. } => {
                    px = x;
                    py = y;
                }
                tree::PathSegment::ClosePath => {
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
) -> tree::PathSegment {
    let quad = lyon_geom::QuadraticBezierSegment {
        from: [px as f32, py as f32].into(),
        ctrl: [x1 as f32, y1 as f32].into(),
        to:   [x as f32,   y as f32].into(),
    };

    let cubic = quad.to_cubic();

    tree::PathSegment::CurveTo {
        x1: cubic.ctrl1.x as f64, y1: cubic.ctrl1.y as f64,
        x2: cubic.ctrl2.x as f64, y2: cubic.ctrl2.y as f64,
        x:  cubic.to.x as f64,    y:  cubic.to.y as f64,
    }
}
