// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Some useful utilities.

// external
use svgdom::{
    Align,
    AspectRatio,
    Transform,
};

// self
use crate::geom::*;
use crate::tree;


/// Converts `viewBox` to `Transform`.
pub fn view_box_to_transform(
    view_box: Rect,
    aspect: AspectRatio,
    img_size: Size,
) -> Transform {
    let vr = view_box;

    let sx = img_size.width() / vr.width();
    let sy = img_size.height() / vr.height();

    let (sx, sy) = if aspect.align == Align::None {
        (sx, sy)
    } else {
        let s = if aspect.slice {
            if sx < sy { sy } else { sx }
        } else {
            if sx > sy { sy } else { sx }
        };

        (s, s)
    };

    let x = -vr.x() * sx;
    let y = -vr.y() * sy;
    let w = img_size.width() - vr.width() * sx;
    let h = img_size.height() - vr.height() * sy;

    let (tx, ty) = aligned_pos(aspect.align, x, y, w, h);
    Transform::new(sx, 0.0, 0.0, sy, tx, ty)
}

/// Returns object aligned position.
pub fn aligned_pos(
    align: Align,
    x: f64, y: f64, w: f64, h: f64,
) -> (f64, f64) {
    match align {
        Align::None     => (x,           y          ),
        Align::XMinYMin => (x,           y          ),
        Align::XMidYMin => (x + w / 2.0, y          ),
        Align::XMaxYMin => (x + w,       y          ),
        Align::XMinYMid => (x,           y + h / 2.0),
        Align::XMidYMid => (x + w / 2.0, y + h / 2.0),
        Align::XMaxYMid => (x + w,       y + h / 2.0),
        Align::XMinYMax => (x,           y + h      ),
        Align::XMidYMax => (x + w / 2.0, y + h      ),
        Align::XMaxYMax => (x + w,       y + h      ),
    }
}

/// Converts `rect` to path segments.
pub fn rect_to_path(
    rect: Rect,
) -> Vec<tree::PathSegment> {
    vec![
        tree::PathSegment::MoveTo {
            x: rect.x(), y: rect.y()
        },
        tree::PathSegment::LineTo {
            x: rect.right(), y: rect.y()
        },
        tree::PathSegment::LineTo {
            x: rect.right(), y: rect.bottom()
        },
        tree::PathSegment::LineTo {
            x: rect.x(), y: rect.bottom()
        },
        tree::PathSegment::ClosePath,
    ]
}

/// Calculates path's bounding box.
///
/// Width and/or height can be zero.
pub fn path_bbox(
    segments: &[tree::PathSegment],
    stroke: Option<&tree::Stroke>,
    ts: Option<tree::Transform>,
) -> Option<Rect> {
    debug_assert!(!segments.is_empty());

    let ts = match ts {
        Some(ts) => ts,
        None => tree::Transform::default(),
    };

    let mut prev_x = 0.0;
    let mut prev_y = 0.0;
    let mut minx = 0.0;
    let mut miny = 0.0;
    let mut maxx = 0.0;
    let mut maxy = 0.0;

    if let Some(tree::PathSegment::MoveTo { x, y }) = TransformedPath::new(segments, ts).next() {
        let x = x as f32;
        let y = y as f32;

        prev_x = x;
        prev_y = y;
        minx = x;
        miny = y;
        maxx = x;
        maxy = y;
    }

    for seg in TransformedPath::new(segments, ts) {
        match seg {
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
    }

    // TODO: find a better way
    // It's an approximation, but it's better than nothing.
    if let Some(ref stroke) = stroke {
        let w = (stroke.width.value() / 2.0) as f32;
        minx -= w;
        miny -= w;
        maxx += w;
        maxy += w;
    }

    let width = maxx - minx;
    let height = maxy - miny;

    Rect::new(minx as f64, miny as f64, width as f64, height as f64)
}

/// An iterator over transformed path segments.
#[allow(missing_debug_implementations)]
pub struct TransformedPath<'a> {
    segments: &'a [tree::PathSegment],
    ts: tree::Transform,
    idx: usize,
}

impl<'a> TransformedPath<'a> {
    /// Creates a new `TransformedPath` iterator.
    pub fn new(segments: &'a [tree::PathSegment], ts: tree::Transform) -> Self {
        TransformedPath { segments, ts, idx: 0 }
    }
}

impl<'a> Iterator for TransformedPath<'a> {
    type Item = tree::PathSegment;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx == self.segments.len() {
            return None;
        }

        if self.ts.is_default() {
            self.idx += 1;
            return self.segments.get(self.idx - 1).cloned();
        }

        let seg = match self.segments[self.idx] {
            tree::PathSegment::MoveTo { x, y } => {
                let (x, y) = self.ts.apply(x, y);
                tree::PathSegment::MoveTo { x, y }
            }
            tree::PathSegment::LineTo { x, y } => {
                let (x, y) = self.ts.apply(x, y);
                tree::PathSegment::LineTo { x, y }
            }
            tree::PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                let (x1, y1) = self.ts.apply(x1, y1);
                let (x2, y2) = self.ts.apply(x2, y2);
                let (x,  y)  = self.ts.apply(x, y);
                tree::PathSegment::CurveTo { x1, y1, x2, y2, x, y }
            }
            tree::PathSegment::ClosePath => tree::PathSegment::ClosePath,
        };

        self.idx += 1;

        Some(seg)
    }
}

/// Calculates path's length.
///
/// Length from the first segment to the first MoveTo, ClosePath or slice end.
pub fn path_length(
    segments: &[tree::PathSegment],
) -> f64 {
    debug_assert!(!segments.is_empty());

    let (mut prev_x, mut prev_y) = {
        if let tree::PathSegment::MoveTo { x, y } = segments[0] {
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
            tree::PathSegment::MoveTo { .. } => {
                if !is_first_seg {
                    break;
                }
            }
            tree::PathSegment::LineTo { x, y } => {
                length += Line::new(prev_x as f64, prev_y as f64, x, y).length();

                prev_x = x as f32;
                prev_y = y as f32;
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

                length += curve.approximate_length(1.0) as f64;

                prev_x = x;
                prev_y = y;
            }
            tree::PathSegment::ClosePath => {
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
pub fn transform_path(
    segments: &mut [tree::PathSegment],
    ts: &tree::Transform,
) {
    for seg in segments {
        match seg {
            tree::PathSegment::MoveTo { x, y } => {
                ts.apply_to(x, y);
            }
            tree::PathSegment::LineTo { x, y } => {
                ts.apply_to(x, y);
            }
            tree::PathSegment::CurveTo { x1, y1, x2, y2, x,  y } => {
                ts.apply_to(x1, y1);
                ts.apply_to(x2, y2);
                ts.apply_to(x, y);
            }
            tree::PathSegment::ClosePath => {}
        }
    }
}
