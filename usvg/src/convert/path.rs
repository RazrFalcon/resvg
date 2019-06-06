// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::f64;

// external
use lyon_geom;
use svgdom;

// self
use crate::tree;
use super::prelude::*;


pub fn convert(
    mut path: svgdom::Path,
) -> Vec<tree::PathSegment> {
    let mut new_path = Vec::with_capacity(path.len());

    path.conv_to_absolute();

    // Previous MoveTo coordinates.
    let mut pmx = 0.0;
    let mut pmy = 0.0;

    // Previous coordinates.
    let mut px = 0.0;
    let mut py = 0.0;

    // Previous SmoothQuadratic coordinates.
    let mut ptx = 0.0;
    let mut pty = 0.0;

    for (idx, seg) in path.iter().enumerate() {
        match *seg {
            svgdom::PathSegment::MoveTo { x, y, .. } => {
                new_path.push(tree::PathSegment::MoveTo { x, y });
            }
            svgdom::PathSegment::LineTo { x, y, .. } => {
                new_path.push(tree::PathSegment::LineTo { x, y });
            }
            svgdom::PathSegment::HorizontalLineTo { x, .. } => {
                new_path.push(tree::PathSegment::LineTo { x, y: py });
            }
            svgdom::PathSegment::VerticalLineTo { y, .. } => {
                new_path.push(tree::PathSegment::LineTo { x: px, y });
            }
            svgdom::PathSegment::CurveTo { x1, y1, x2, y2, x, y, .. } => {
                new_path.push(tree::PathSegment::CurveTo { x1, y1, x2, y2, x, y });
            }
            svgdom::PathSegment::SmoothCurveTo { x2, y2, x, y, .. } => {
                // 'The first control point is assumed to be the reflection of the second control
                // point on the previous command relative to the current point.
                // (If there is no previous command or if the previous command
                // was not an C, c, S or s, assume the first control point is
                // coincident with the current point.)'
                if let Some(prev_seg) = path.get(idx - 1).cloned() {
                    let (x1, y1) = match prev_seg {
                        svgdom::PathSegment::CurveTo { x2, y2, x, y, .. } |
                        svgdom::PathSegment::SmoothCurveTo { x2, y2, x, y, .. } => {
                            (x * 2.0 - x2, y * 2.0 - y2)
                        }
                        _ => {
                            (px, py)
                        }
                    };

                    new_path.push(tree::PathSegment::CurveTo { x1, y1, x2, y2, x, y });
                }
            }
            svgdom::PathSegment::Quadratic { x1, y1, x, y, .. } => {
                new_path.push(quad_to_curve(px, py, x1, y1, x, y));
            }
            svgdom::PathSegment::SmoothQuadratic { x, y, .. } => {
                // 'The control point is assumed to be the reflection of
                // the control point on the previous command relative to
                // the current point. (If there is no previous command or
                // if the previous command was not a Q, q, T or t, assume
                // the control point is coincident with the current point.)'
                if let Some(prev_seg) = path.get(idx - 1).cloned() {
                    let (x1, y1) = match prev_seg {
                        svgdom::PathSegment::Quadratic { x1, y1, x, y, .. } => {
                            (x * 2.0 - x1, y * 2.0 - y1)
                        }
                        svgdom::PathSegment::SmoothQuadratic { x, y, .. } => {
                            (x * 2.0 - ptx, y * 2.0 - pty)
                        }
                        _ => {
                            (px, py)
                        }
                    };

                    ptx = x1;
                    pty = y1;

                    new_path.push(quad_to_curve(px, py, x1, y1, x, y));
                }
            }
            svgdom::PathSegment::EllipticalArc { rx, ry, x_axis_rotation, large_arc, sweep, x, y, .. } => {
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
            svgdom::PathSegment::ClosePath { .. } => {
                if let Some(tree::PathSegment::ClosePath) = new_path.last() {
                    // Do not add sequential ClosePath segments.
                } else {
                    new_path.push(tree::PathSegment::ClosePath);
                }
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

//    // TODO: find a better way
//    if stroke.is_some() {
//        // If the controls point coordinate is too close to the end point
//        // we have to snap it to the end point. Otherwise, it will produce rendering errors.
//
//        // Just a magic/heuristic number.
//        let sw = 0.25;
//
//        for seg in &mut new_path {
//            if let &mut tree::PathSegment::CurveTo
//                { ref mut x1, ref mut y1,ref mut x2, ref mut y2, x, y } = seg
//            {
//                if (x - *x1).abs() < sw { *x1 = x; }
//                if (y - *y1).abs() < sw { *y1 = y; }
//                if (x - *x2).abs() < sw { *x2 = x; }
//                if (y - *y2).abs() < sw { *y2 = y; }
//            }
//        }
//    }

    new_path
}

fn quad_to_curve(
    px: f64,
    py: f64,
    x1: f64,
    y1: f64,
    x: f64,
    y: f64,
) -> tree::PathSegment {
    let quad = lyon_geom::QuadraticBezierSegment {
        from: [px as f32, py as f32].into(),
        ctrl: [x1 as f32, y1 as f32].into(),
        to:   [x  as f32,  y as f32].into(),
    };

    let cubic = quad.to_cubic();

    tree::PathSegment::CurveTo {
        x1: cubic.ctrl1.x as f64, y1: cubic.ctrl1.y as f64,
        x2: cubic.ctrl2.x as f64, y2: cubic.ctrl2.y as f64,
        x:  cubic.to.x as f64,    y:  cubic.to.y as f64,
    }
}

pub fn has_bbox(segments: &[tree::PathSegment]) -> bool {
    debug_assert!(!segments.is_empty());

    let (mut prev_x, mut prev_y, mut minx, mut miny, mut maxx, mut maxy) = {
        if let tree::PathSegment::MoveTo { x, y } = segments[0] {
            (x as f32, y as f32, x as f32, y as f32, x as f32, y as f32)
        } else {
            unreachable!();
        }
    };

    for seg in segments {
        match *seg {
            tree::PathSegment::MoveTo { x, y }
            | tree::PathSegment::LineTo { x, y } => {
                let x = x as f32;
                let y = y as f32;
                prev_x = x;
                prev_y = y;

                if x > maxx { maxx = x; }
                else if x < minx { minx = x; }

                if y > maxy { maxy = y; }
                else if y < miny { miny = y; }
            }
            tree::PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                let x = x as f32;
                let y = y as f32;

                let curve = lyon_geom::CubicBezierSegment {
                    from: lyon_geom::math::Point::new(prev_x, prev_y),
                    ctrl1: lyon_geom::math::Point::new(x1 as f32, y1 as f32),
                    ctrl2: lyon_geom::math::Point::new(x2 as f32, y2 as f32),
                    to: lyon_geom::math::Point::new(x, y),
                };

                prev_x = x;
                prev_y = y;

                let r = curve.bounding_rect();

                let right = r.max_x();
                let bottom = r.max_y();
                if r.min_x() < minx { minx = r.min_x(); }
                if right > maxx { maxx = right; }
                if r.min_y() < miny { miny = r.min_y(); }
                if bottom > maxy { maxy = bottom; }
            }
            tree::PathSegment::ClosePath => {}
        }

        let width = (maxx - minx) as f64;
        let height = (maxy - miny) as f64;
        if !(width.is_fuzzy_zero() || height.is_fuzzy_zero()) {
            return true;
        }
    }

    false
}
