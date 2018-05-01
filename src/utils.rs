// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Some useful utilities.

use std::f64;

// external
use usvg::tree;
use usvg::tree::prelude::*;
pub use usvg::utils::*;

// self
use geom::*;
use {
    FitTo,
};


/// Returns `size` preprocessed according to `FitTo`.
pub fn fit_to(size: ScreenSize, fit: FitTo) -> ScreenSize {
    let sizef = size.to_f64();

    match fit {
        FitTo::Original => {
            size
        }
        FitTo::Width(w) => {
            let h = (w as f64 * sizef.height / sizef.width).ceil();
            ScreenSize::new(w, h as u32)
        }
        FitTo::Height(h) => {
            let w = (h as f64 * sizef.width / sizef.height).ceil();
            ScreenSize::new(w as u32, h)
        }
        FitTo::Zoom(z) => {
            Size::new(sizef.width * z as f64, sizef.height * z as f64).to_screen_size()
        }
    }
}

pub(crate) fn process_text_anchor(x: f64, a: tree::TextAnchor, text_width: f64) -> f64 {
    match a {
        tree::TextAnchor::Start =>  x, // Nothing.
        tree::TextAnchor::Middle => x - text_width / 2.0,
        tree::TextAnchor::End =>    x - text_width,
    }
}

pub(crate) fn apply_view_box(vb: &tree::ViewBox, img_size: ScreenSize) -> ScreenSize {
    if vb.aspect.align == tree::Align::None {
        vb.rect.size.to_screen_size()
    } else {
        if vb.aspect.slice {
            img_size.expand_to(vb.rect.size.to_screen_size())
        } else {
            img_size.scale_to(vb.rect.size.to_screen_size())
        }
    }
}

/// Returns node's absolute transform.
pub fn abs_transform(
    node: &tree::Node,
) -> tree::Transform {
    let mut ts_list = Vec::new();
    for p in node.ancestors().skip(1) {
        ts_list.push(p.transform());
    }

    let mut root_ts = tree::Transform::default();
    for ts in ts_list.iter().rev() {
        root_ts.append(ts);
    }

    root_ts
}

/// Calculates path's bounding box.
///
/// Minimum size is 1x1.
pub fn path_bbox(
    segments: &[tree::PathSegment],
    stroke: Option<&tree::Stroke>,
    ts: &tree::Transform,
) -> Rect {
    debug_assert!(!segments.is_empty());

    use lyon_geom;

    let mut path_buf = Vec::new();
    let new_path = if !ts.is_default() {
        // Allocate only when transform is required.
        path_buf.reserve(segments.len());
        for seg in segments {
            match *seg {
                tree::PathSegment::MoveTo { x, y } => {
                    let (x, y) = ts.apply(x, y);
                    path_buf.push(tree::PathSegment::MoveTo { x, y });
                }
                tree::PathSegment::LineTo { x, y } => {
                    let (x, y) = ts.apply(x, y);
                    path_buf.push(tree::PathSegment::LineTo { x, y });
                }
                tree::PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                    let (x1, y1) = ts.apply(x1, y1);
                    let (x2, y2) = ts.apply(x2, y2);
                    let (x, y) = ts.apply(x, y);
                    path_buf.push(tree::PathSegment::CurveTo { x1, y1, x2, y2, x, y });
                }
                tree::PathSegment::ClosePath => {
                    path_buf.push(tree::PathSegment::ClosePath);
                }
            }
        }

        &path_buf
    } else {
        segments
    };

    let (mut prev_x, mut prev_y, mut minx, mut miny, mut maxx, mut maxy) = {
        if let tree::PathSegment::MoveTo { x, y } = new_path[0] {
            (x as f32, y as f32, x as f32, y as f32, x as f32, y as f32)
        } else {
            unreachable!();
        }
    };

    for seg in new_path {
        match *seg {
              tree::PathSegment::MoveTo { x, y }
            | tree::PathSegment::LineTo { x, y } => {
                let x = x as f32;
                let y = y as f32;
                prev_x = x;
                prev_y = y;

                if x > maxx {
                    maxx = x;
                } else if x < minx {
                    minx = x;
                }

                if y > maxy {
                    maxy = y;
                } else if y < miny {
                    miny = y;
                }
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
    }

    // TODO: find a better way
    // It's an approximation, but it's better than nothing.
    if let Some(ref stroke) = stroke {
        let w = (stroke.width / 2.0) as f32;
        minx -= w;
        miny -= w;
        maxx += w;
        maxy += w;
    }

    let mut width = maxx - minx;
    if width < 1.0 { width = 1.0; }

    let mut height = maxy - miny;
    if height < 1.0 { height = 1.0; }

    Rect::from_xywh(minx as f64, miny as f64, width as f64, height as f64)
}

/// Converts `rect` to path segments.
pub fn rect_to_path(rect: Rect) -> Vec<tree::PathSegment> {
    vec![
        tree::PathSegment::MoveTo {
            x: rect.x(), y: rect.y()
        },
        tree::PathSegment::LineTo {
            x: rect.x() + rect.width(), y: rect.y()
        },
        tree::PathSegment::LineTo {
            x: rect.x() + rect.width(), y: rect.y() + rect.height()
        },
        tree::PathSegment::LineTo {
            x: rect.x(), y: rect.y() + rect.height()
        },
        tree::PathSegment::ClosePath,
    ]
}

/// Converts an image to an alpha mask.
pub(crate) fn image_to_mask(
    data: &mut [u8],
    img_size: ScreenSize,
    opacity: Option<tree::Opacity>,
) {
    let width = img_size.width;
    let height = img_size.height;
    let stride = width * 4;

    let coeff_r = 0.2125 / 255.0;
    let coeff_g = 0.7154 / 255.0;
    let coeff_b = 0.0721 / 255.0;

    for y in 0..height {
        for x in 0..width {
            let idx = (y * stride + x * 4) as usize;

            let r = data[idx + 2] as f64;
            let g = data[idx + 1] as f64;
            let b = data[idx + 0] as f64;

            let mut luma = r * coeff_r + g * coeff_g + b * coeff_b;
            if let Some(opacity) = opacity {
                luma *= opacity.value();
            }

            data[idx + 0] = 0;
            data[idx + 1] = 0;
            data[idx + 2] = 0;
            data[idx + 3] = f64_bound(0.0, luma * 255.0, 255.0) as u8;
        }
    }
}
