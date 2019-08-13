// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgtypes::FuzzyZero;

use kurbo::{ParamCurveArclen, ParamCurveExtrema};

use crate::{Rect, Line};
use super::Transform;

/// A path's absolute segment.
///
/// Unlike the SVG spec, can contain only `M`, `L`, `C` and `Z` segments.
/// All other segments will be converted into this one.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug)]
pub enum PathSegment {
    MoveTo {
        x: f64,
        y: f64,
    },
    LineTo {
        x: f64,
        y: f64,
    },
    CurveTo {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x: f64,
        y: f64,
    },
    ClosePath,
}


/// An SVG path data container.
///
/// All segments are in absolute coordinates.
#[derive(Clone, Default, Debug)]
pub struct PathData(pub Vec<PathSegment>);

impl PathData {
    /// Creates a new path.
    pub fn new() -> Self {
        PathData(Vec::new())
    }

    /// Creates a new path with a specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        PathData(Vec::with_capacity(capacity))
    }

    /// Creates a path from a rect.
    pub fn from_rect(rect: Rect) -> Self {
        PathData(vec![
            PathSegment::MoveTo {
                x: rect.x(), y: rect.y()
            },
            PathSegment::LineTo {
                x: rect.right(), y: rect.y()
            },
            PathSegment::LineTo {
                x: rect.right(), y: rect.bottom()
            },
            PathSegment::LineTo {
                x: rect.x(), y: rect.bottom()
            },
            PathSegment::ClosePath,
        ])
    }

    /// Pushes a MoveTo segment to the path.
    pub fn push_move_to(&mut self, x: f64, y: f64) {
        self.push(PathSegment::MoveTo { x, y });
    }

    /// Pushes a LineTo segment to the path.
    pub fn push_line_to(&mut self, x: f64, y: f64) {
        self.push(PathSegment::LineTo { x, y });
    }

    /// Pushes a CurveTo segment to the path.
    pub fn push_curve_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) {
        self.push(PathSegment::CurveTo { x1, y1, x2, y2, x, y });
    }

    /// Pushes a ClosePath segment to the path.
    pub fn push_close_path(&mut self) {
        self.push(PathSegment::ClosePath);
    }

    /// Calculates path's bounding box.
    ///
    /// This operation is expensive.
    pub fn bbox(&self) -> Option<Rect> {
        calc_bbox(self)
    }

    /// Calculates path's bounding box with a specified transform.
    ///
    /// This operation is expensive.
    pub fn bbox_with_transform(
        &self,
        ts: Transform,
        stroke: Option<&super::Stroke>,
    ) -> Option<Rect> {
        calc_bbox_with_transform(self, ts, stroke)
    }

    /// Checks that path has a bounding box.
    ///
    /// This operation is expensive.
    pub fn has_bbox(&self) -> bool {
        has_bbox(self)
    }

    /// Calculates path's length.
    ///
    /// Length from the first segment to the first MoveTo, ClosePath or slice end.
    ///
    /// This operation is expensive.
    pub fn length(&self) -> f64 {
        calc_length(self)
    }

    /// Applies the transform to the path.
    pub fn transform(&mut self, ts: Transform) {
        transform_path(self, ts);
    }

    /// Applies the transform to the path from the specified offset.
    pub fn transform_from(&mut self, offset: usize, ts: Transform) {
        transform_path(&mut self[offset..], ts);
    }

    /// Returns an iterator over path subpaths.
    pub fn subpaths(&self) -> SubPathIter {
        SubPathIter {
            path: self,
            index: 0,
        }
    }
}

impl std::ops::Deref for PathData {
    type Target = Vec<PathSegment>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for PathData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}


/// An iterator over `PathData` subpaths.
#[allow(missing_debug_implementations)]
pub struct SubPathIter<'a> {
    path: &'a [PathSegment],
    index: usize,
}

impl<'a> Iterator for SubPathIter<'a> {
    type Item = SubPathData<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.path.len() {
            return None;
        }

        let mut i = self.index;
        while i < self.path.len() {
            match self.path[i] {
                PathSegment::MoveTo { .. } => {
                    if i != self.index {
                        break;
                    }
                }
                PathSegment::ClosePath => {
                    i += 1;
                    break;
                }
                _ => {}
            }

            i += 1;
        }

        let start = self.index;
        self.index = i;

        Some(SubPathData(&self.path[start..i]))
    }
}


/// A reference to a `PathData` subpath.
#[derive(Clone, Copy, Debug)]
pub struct SubPathData<'a>(pub &'a [PathSegment]);

impl<'a> SubPathData<'a> {
    /// Calculates path's bounding box.
    ///
    /// This operation is expensive.
    pub fn bbox(&self) -> Option<Rect> {
        calc_bbox(self)
    }

    /// Calculates path's bounding box with a specified transform.
    ///
    /// This operation is expensive.
    pub fn bbox_with_transform(
        &self,
        ts: Transform,
        stroke: Option<&super::Stroke>,
    ) -> Option<Rect> {
        calc_bbox_with_transform(self, ts, stroke)
    }

    /// Checks that path has a bounding box.
    ///
    /// This operation is expensive.
    pub fn has_bbox(&self) -> bool {
        has_bbox(self)
    }

    /// Calculates path's length.
    ///
    /// This operation is expensive.
    pub fn length(&self) -> f64 {
        calc_length(self)
    }
}

impl std::ops::Deref for SubPathData<'_> {
    type Target = [PathSegment];

    fn deref(&self) -> &Self::Target {
        self.0
    }
}


fn calc_bbox(segments: &[PathSegment]) -> Option<Rect> {
    debug_assert!(!segments.is_empty());

    let mut prev_x = 0.0;
    let mut prev_y = 0.0;
    let mut minx = 0.0;
    let mut miny = 0.0;
    let mut maxx = 0.0;
    let mut maxy = 0.0;

    if let PathSegment::MoveTo { x, y } = segments[0].clone() {
        prev_x = x;
        prev_y = y;
        minx = x;
        miny = y;
        maxx = x;
        maxy = y;
    }

    for seg in segments.iter().cloned() {
        match seg {
            PathSegment::MoveTo { x, y }
            | PathSegment::LineTo { x, y } => {
                prev_x = x;
                prev_y = y;

                if x > maxx { maxx = x; }
                else if x < minx { minx = x; }

                if y > maxy { maxy = y; }
                else if y < miny { miny = y; }
            }
            PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                let curve = kurbo::CubicBez {
                    p0: kurbo::Vec2::new(prev_x, prev_y),
                    p1: kurbo::Vec2::new(x1, y1),
                    p2: kurbo::Vec2::new(x2, y2),
                    p3: kurbo::Vec2::new(x, y),
                };

                let r = curve.bounding_box();

                if r.x0 < minx { minx = r.x0; }
                if r.x1 > maxx { maxx = r.x1; }
                if r.y0 < miny { miny = r.y0; }
                if r.y1 > maxy { maxy = r.y1; }
            }
            PathSegment::ClosePath => {}
        }
    }

    let width = maxx - minx;
    let height = maxy - miny;

    Rect::new(minx, miny, width, height)
}

fn calc_bbox_with_transform(
    segments: &[PathSegment],
    ts: Transform,
    stroke: Option<&super::Stroke>,
) -> Option<Rect> {
    debug_assert!(!segments.is_empty());

    let mut prev_x = 0.0;
    let mut prev_y = 0.0;
    let mut minx = 0.0;
    let mut miny = 0.0;
    let mut maxx = 0.0;
    let mut maxy = 0.0;

    if let Some(PathSegment::MoveTo { x, y }) = TransformedPath::new(segments, ts).next() {
        prev_x = x;
        prev_y = y;
        minx = x;
        miny = y;
        maxx = x;
        maxy = y;
    }

    for seg in TransformedPath::new(segments, ts) {
        match seg {
            PathSegment::MoveTo { x, y }
            | PathSegment::LineTo { x, y } => {
                prev_x = x;
                prev_y = y;

                if x > maxx { maxx = x; }
                else if x < minx { minx = x; }

                if y > maxy { maxy = y; }
                else if y < miny { miny = y; }
            }
            PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                let curve = kurbo::CubicBez {
                    p0: kurbo::Vec2::new(prev_x, prev_y),
                    p1: kurbo::Vec2::new(x1, y1),
                    p2: kurbo::Vec2::new(x2, y2),
                    p3: kurbo::Vec2::new(x, y),
                };

                let r = curve.bounding_box();

                if r.x0 < minx { minx = r.x0; }
                if r.x1 > maxx { maxx = r.x1; }
                if r.y0 < miny { miny = r.y0; }
                if r.y1 > maxy { maxy = r.y1; }
            }
            PathSegment::ClosePath => {}
        }
    }

    // TODO: find a better way
    // It's an approximation, but it's better than nothing.
    if let Some(ref stroke) = stroke {
        let w = stroke.width.value() / 2.0;
        minx -= w;
        miny -= w;
        maxx += w;
        maxy += w;
    }

    let width = maxx - minx;
    let height = maxy - miny;

    Rect::new(minx, miny, width, height)
}

fn has_bbox(segments: &[PathSegment]) -> bool {
    debug_assert!(!segments.is_empty());

    let mut prev_x = 0.0;
    let mut prev_y = 0.0;
    let mut minx = 0.0;
    let mut miny = 0.0;
    let mut maxx = 0.0;
    let mut maxy = 0.0;

    if let PathSegment::MoveTo { x, y } = segments[0] {
        prev_x = x;
        prev_y = y;
        minx = x;
        miny = y;
        maxx = x;
        maxy = y;
    }

    for seg in segments {
        match *seg {
              PathSegment::MoveTo { x, y }
            | PathSegment::LineTo { x, y } => {
                prev_x = x;
                prev_y = y;

                if x > maxx { maxx = x; }
                else if x < minx { minx = x; }

                if y > maxy { maxy = y; }
                else if y < miny { miny = y; }
            }
            PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                let curve = kurbo::CubicBez {
                    p0: kurbo::Vec2::new(prev_x, prev_y),
                    p1: kurbo::Vec2::new(x1, y1),
                    p2: kurbo::Vec2::new(x2, y2),
                    p3: kurbo::Vec2::new(x, y),
                };

                let r = curve.bounding_box();

                if r.x0 < minx { minx = r.x0; }
                if r.x1 > maxx { maxx = r.x1; }
                if r.x0 < miny { miny = r.y0; }
                if r.y1 > maxy { maxy = r.y1; }
            }
            PathSegment::ClosePath => {}
        }

        let width = (maxx - minx) as f64;
        let height = (maxy - miny) as f64;
        if !(width.is_fuzzy_zero() || height.is_fuzzy_zero()) {
            return true;
        }
    }

    false
}

fn calc_length(segments: &[PathSegment]) -> f64 {
    debug_assert!(!segments.is_empty());

    let (mut prev_x, mut prev_y) = {
        if let PathSegment::MoveTo { x, y } = segments[0] {
            (x, y)
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
            PathSegment::MoveTo { .. } => {
                if !is_first_seg {
                    break;
                }
            }
            PathSegment::LineTo { x, y } => {
                length += Line::new(prev_x, prev_y, x, y).length();

                prev_x = x;
                prev_y = y;
            }
            PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                let curve = kurbo::CubicBez {
                    p0: kurbo::Vec2::new(prev_x, prev_y),
                    p1: kurbo::Vec2::new(x1, y1),
                    p2: kurbo::Vec2::new(x2, y2),
                    p3: kurbo::Vec2::new(x, y),
                };

                length += curve.arclen(1.0);

                prev_x = x;
                prev_y = y;
            }
            PathSegment::ClosePath => {
                length += Line::new(prev_x, prev_y, start_x, start_y).length();
                break;
            }
        }

        is_first_seg = false;
    }

    length
}

fn transform_path(segments: &mut [PathSegment], ts: Transform) {
    for seg in segments {
        match seg {
            PathSegment::MoveTo { x, y } => {
                ts.apply_to(x, y);
            }
            PathSegment::LineTo { x, y } => {
                ts.apply_to(x, y);
            }
            PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                ts.apply_to(x1, y1);
                ts.apply_to(x2, y2);
                ts.apply_to(x, y);
            }
            PathSegment::ClosePath => {}
        }
    }
}


/// An iterator over transformed path segments.
#[allow(missing_debug_implementations)]
pub struct TransformedPath<'a> {
    segments: &'a [PathSegment],
    ts: Transform,
    idx: usize,
}

impl<'a> TransformedPath<'a> {
    /// Creates a new `TransformedPath` iterator.
    pub fn new(segments: &'a [PathSegment], ts: Transform) -> Self {
        TransformedPath { segments, ts, idx: 0 }
    }
}

impl<'a> Iterator for TransformedPath<'a> {
    type Item = PathSegment;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx == self.segments.len() {
            return None;
        }

        let seg = match self.segments[self.idx] {
            PathSegment::MoveTo { x, y } => {
                let (x, y) = self.ts.apply(x, y);
                PathSegment::MoveTo { x, y }
            }
            PathSegment::LineTo { x, y } => {
                let (x, y) = self.ts.apply(x, y);
                PathSegment::LineTo { x, y }
            }
            PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                let (x1, y1) = self.ts.apply(x1, y1);
                let (x2, y2) = self.ts.apply(x2, y2);
                let (x,  y)  = self.ts.apply(x, y);
                PathSegment::CurveTo { x1, y1, x2, y2, x, y }
            }
            PathSegment::ClosePath => PathSegment::ClosePath,
        };

        self.idx += 1;

        Some(seg)
    }
}
