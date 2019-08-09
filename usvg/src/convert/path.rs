// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::tree;


pub fn convert(
    mut path: svgtypes::Path,
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
            svgtypes::PathSegment::MoveTo { x, y, .. } => {
                new_path.push(tree::PathSegment::MoveTo { x, y });
            }
            svgtypes::PathSegment::LineTo { x, y, .. } => {
                new_path.push(tree::PathSegment::LineTo { x, y });
            }
            svgtypes::PathSegment::HorizontalLineTo { x, .. } => {
                new_path.push(tree::PathSegment::LineTo { x, y: py });
            }
            svgtypes::PathSegment::VerticalLineTo { y, .. } => {
                new_path.push(tree::PathSegment::LineTo { x: px, y });
            }
            svgtypes::PathSegment::CurveTo { x1, y1, x2, y2, x, y, .. } => {
                new_path.push(tree::PathSegment::CurveTo { x1, y1, x2, y2, x, y });
            }
            svgtypes::PathSegment::SmoothCurveTo { x2, y2, x, y, .. } => {
                // 'The first control point is assumed to be the reflection of the second control
                // point on the previous command relative to the current point.
                // (If there is no previous command or if the previous command
                // was not an C, c, S or s, assume the first control point is
                // coincident with the current point.)'
                if let Some(prev_seg) = path.get(idx - 1).cloned() {
                    let (x1, y1) = match prev_seg {
                        svgtypes::PathSegment::CurveTo { x2, y2, x, y, .. } |
                        svgtypes::PathSegment::SmoothCurveTo { x2, y2, x, y, .. } => {
                            (x * 2.0 - x2, y * 2.0 - y2)
                        }
                        _ => {
                            (px, py)
                        }
                    };

                    new_path.push(tree::PathSegment::CurveTo { x1, y1, x2, y2, x, y });
                }
            }
            svgtypes::PathSegment::Quadratic { x1, y1, x, y, .. } => {
                new_path.push(quad_to_curve(px, py, x1, y1, x, y));
            }
            svgtypes::PathSegment::SmoothQuadratic { x, y, .. } => {
                // 'The control point is assumed to be the reflection of
                // the control point on the previous command relative to
                // the current point. (If there is no previous command or
                // if the previous command was not a Q, q, T or t, assume
                // the control point is coincident with the current point.)'
                if let Some(prev_seg) = path.get(idx - 1).cloned() {
                    let (x1, y1) = match prev_seg {
                        svgtypes::PathSegment::Quadratic { x1, y1, x, y, .. } => {
                            (x * 2.0 - x1, y * 2.0 - y1)
                        }
                        svgtypes::PathSegment::SmoothQuadratic { x, y, .. } => {
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
            svgtypes::PathSegment::EllipticalArc { rx, ry, x_axis_rotation, large_arc, sweep, x, y, .. } => {
                let svg_arc = kurbo::SvgArc {
                    from: kurbo::Vec2::new(px, py),
                    to: kurbo::Vec2::new(x, y),
                    radii: kurbo::Vec2::new(rx, ry),
                    x_rotation: x_axis_rotation.to_radians(),
                    large_arc,
                    sweep,
                };

                match kurbo::Arc::from_svg_arc(&svg_arc) {
                    Some(arc) => {
                        arc.to_cubic_beziers(0.1, |p1, p2, p| {
                            new_path.push(tree::PathSegment::CurveTo {
                                x1: p1.x, y1: p1.y,
                                x2: p2.x, y2: p2.y,
                                x: p.x, y: p.y,
                            });
                        });
                    }
                    None => {
                        new_path.push(tree::PathSegment::LineTo { x, y });
                    }
                }
            }
            svgtypes::PathSegment::ClosePath { .. } => {
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

fn quad_to_curve(px: f64, py: f64, x1: f64, y1: f64, x: f64, y: f64) -> tree::PathSegment {
    #[inline]
    fn calc(n1: f64, n2: f64) -> f64 {
        (n1 + n2 * 2.0) / 3.0
    }

    tree::PathSegment::CurveTo {
        x1: calc(px, x1), y1: calc(py, y1),
        x2:  calc(x, x1), y2:  calc(y, y1),
        x, y,
    }
}
