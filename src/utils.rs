// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Some useful utilities.

use std::f64;

// external
pub use usvg::utils::*;

// self
use super::prelude::*;
use crate::FitTo;


/// Returns `size` preprocessed according to `FitTo`.
pub fn fit_to(size: ScreenSize, fit: FitTo) -> Option<ScreenSize> {
    let sizef = size.to_size();

    match fit {
        FitTo::Original => {
            Some(size)
        }
        FitTo::Width(w) => {
            let h = (w as f64 * sizef.height() / sizef.width()).ceil();
            ScreenSize::new(w, h as u32)
        }
        FitTo::Height(h) => {
            let w = (h as f64 * sizef.width() / sizef.height()).ceil();
            ScreenSize::new(w as u32, h)
        }
        FitTo::Zoom(z) => {
            Size::new(sizef.width() * z as f64, sizef.height() * z as f64)
                 .map(|s| s.to_screen_size())
        }
    }
}

pub(crate) fn apply_view_box(vb: &usvg::ViewBox, img_size: ScreenSize) -> ScreenSize {
    let s = vb.rect.to_screen_size();

    if vb.aspect.align == usvg::Align::None {
        s
    } else {
        if vb.aspect.slice {
            img_size.expand_to(s)
        } else {
            img_size.scale_to(s)
        }
    }
}

/// Returns node's absolute transform.
///
/// Does not include the node's transform itself.
pub fn abs_transform(
    node: &usvg::Node,
) -> usvg::Transform {
    let mut ts_list = Vec::new();
    for p in node.ancestors().skip(1) {
        ts_list.push(p.transform());
    }

    let mut root_ts = usvg::Transform::default();
    for ts in ts_list.iter().rev() {
        root_ts.append(ts);
    }

    root_ts
}

/// Calculates path's length.
///
/// Length from the first segment to the first MoveTo, ClosePath or slice end.
pub fn path_length(segments: &[usvg::PathSegment]) -> f64 {
    debug_assert!(!segments.is_empty());

    use crate::lyon_geom;

    let (mut prev_x, mut prev_y) = {
        if let usvg::PathSegment::MoveTo { x, y } = segments[0] {
            (x as f32, y as f32)
        } else {
            panic!("first segment must be MoveTo");
        }
    };

    let start_x = prev_x;
    let start_y = prev_y;

    let mut is_first_seg = true;
    let mut length = 0.0f64;
    for seg in segments {
        match *seg {
            usvg::PathSegment::MoveTo { .. } => {
                if !is_first_seg {
                    break;
                }
            }
            usvg::PathSegment::LineTo { x, y } => {
                length += Line::new(prev_x as f64, prev_y as f64, x, y).length();

                prev_x = x as f32;
                prev_y = y as f32;
            }
            usvg::PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                let x = x as f32;
                let y = y as f32;

                let curve = lyon_geom::CubicBezierSegment {
                    from: lyon_geom::math::Point::new(prev_x, prev_y),
                    ctrl1: lyon_geom::math::Point::new(x1 as f32, y1 as f32),
                    ctrl2: lyon_geom::math::Point::new(x2 as f32, y2 as f32),
                    to: lyon_geom::math::Point::new(x, y),
                };

                length += curve.approximate_length(1.0) as f64;

                prev_x = x;
                prev_y = y;
            }
            usvg::PathSegment::ClosePath => {
                length += Line::new(prev_x as f64, prev_y as f64,
                                    start_x as f64, start_y as f64).length();
                break;
            }
        }

        is_first_seg = false;
    }

    length
}

/// Applies the transform to the path segments.
pub fn transform_path(segments: &mut [usvg::PathSegment], ts: &usvg::Transform) {
    for seg in segments {
        match *seg {
            usvg::PathSegment::MoveTo { ref mut x, ref mut y } => {
                ts.apply_to(x, y);
            }
            usvg::PathSegment::LineTo { ref mut x, ref mut y } => {
                ts.apply_to(x, y);
            }
            usvg::PathSegment::CurveTo { ref mut x1, ref mut y1, ref mut x2,
                                         ref mut y2, ref mut x, ref mut y } => {
                ts.apply_to(x1, y1);
                ts.apply_to(x2, y2);
                ts.apply_to(x, y);
            }
            usvg::PathSegment::ClosePath => {}
        }
    }
}
