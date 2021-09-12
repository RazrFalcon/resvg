// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::rc::Rc;

use kurbo::{ParamCurveArclen, ParamCurveExtrema, ParamCurve};

use crate::{Rect, PathBbox, Transform, FuzzyZero};

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

/// A reference-counted `PathData`.
///
/// `PathData` is usually pretty big and it's expensive to clone it,
/// so we are using `Rc`.
pub(crate) type SharedPathData = Rc<PathData>;

impl PathData {
    /// Creates a new path.
    #[inline]
    pub fn new() -> Self {
        PathData(Vec::new())
    }

    /// Creates a new path with a specified capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        PathData(Vec::with_capacity(capacity))
    }

    /// Creates a path from a rect.
    #[inline]
    pub fn from_rect(rect: Rect) -> Self {
        let mut path = PathData::with_capacity(5);
        path.push_rect(rect);
        path
    }

    /// Pushes a MoveTo segment to the path.
    #[inline]
    pub fn push_move_to(&mut self, x: f64, y: f64) {
        self.push(PathSegment::MoveTo { x, y });
    }

    /// Pushes a LineTo segment to the path.
    #[inline]
    pub fn push_line_to(&mut self, x: f64, y: f64) {
        self.push(PathSegment::LineTo { x, y });
    }

    /// Pushes a CurveTo segment to the path.
    #[inline]
    pub fn push_curve_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) {
        self.push(PathSegment::CurveTo { x1, y1, x2, y2, x, y });
    }

    /// Pushes a QuadTo segment to the path.
    ///
    /// Will be converted into cubic curve.
    #[inline]
    pub fn push_quad_to(&mut self, x1: f64, y1: f64, x: f64, y: f64) {
        let (prev_x, prev_y) = self.last_pos();
        self.push(quad_to_curve(prev_x, prev_y, x1, y1, x, y));
    }

    /// Pushes an ArcTo segment to the path.
    ///
    /// Arc will be converted into cubic curves.
    pub fn push_arc_to(
        &mut self,
        rx: f64, ry: f64,
        x_axis_rotation: f64,
        large_arc: bool,
        sweep: bool,
        x: f64, y: f64,
    ) {
        let (prev_x, prev_y) = self.last_pos();

        let svg_arc = kurbo::SvgArc {
            from: kurbo::Point::new(prev_x, prev_y),
            to: kurbo::Point::new(x, y),
            radii: kurbo::Vec2::new(rx, ry),
            x_rotation: x_axis_rotation.to_radians(),
            large_arc,
            sweep,
        };

        match kurbo::Arc::from_svg_arc(&svg_arc) {
            Some(arc) => {
                arc.to_cubic_beziers(0.1, |p1, p2, p| {
                    self.push_curve_to(p1.x, p1.y, p2.x, p2.y, p.x, p.y);
                });
            }
            None => {
                self.push_line_to(x, y);
            }
        }
    }

    /// Pushes a ClosePath segment to the path.
    #[inline]
    pub fn push_close_path(&mut self) {
        self.push(PathSegment::ClosePath);
    }

    /// Pushes a rect to the path.
    #[inline]
    pub fn push_rect(&mut self, rect: Rect) {
        self.extend_from_slice(&[
            PathSegment::MoveTo { x: rect.x(),     y: rect.y() },
            PathSegment::LineTo { x: rect.right(), y: rect.y() },
            PathSegment::LineTo { x: rect.right(), y: rect.bottom() },
            PathSegment::LineTo { x: rect.x(),     y: rect.bottom() },
            PathSegment::ClosePath,
        ]);
    }

    #[inline]
    fn last_pos(&self) -> (f64, f64) {
        let seg = self.last().expect("path must not be empty");
        match seg {
              PathSegment::MoveTo { x, y }
            | PathSegment::LineTo { x, y }
            | PathSegment::CurveTo { x, y, .. } => {
               (*x, *y)
            }
            PathSegment::ClosePath => {
                panic!("the previous segment must be M/L/C")
            }
        }
    }

    /// Calculates path's bounding box.
    ///
    /// This operation is expensive.
    #[inline]
    pub fn bbox(&self) -> Option<PathBbox> {
        calc_bbox(self)
    }

    /// Calculates path's bounding box with a specified transform.
    ///
    /// This operation is expensive.
    #[inline]
    pub fn bbox_with_transform(
        &self,
        ts: Transform,
        stroke: Option<&super::Stroke>,
    ) -> Option<PathBbox> {
        calc_bbox_with_transform(self, ts, stroke)
    }

    /// Checks that path has a bounding box.
    ///
    /// This operation is expensive.
    #[inline]
    pub fn has_bbox(&self) -> bool {
        has_bbox(self)
    }

    /// Calculates path's length.
    ///
    /// Length from the first segment to the first MoveTo, ClosePath or slice end.
    ///
    /// This operation is expensive.
    #[inline]
    pub fn length(&self) -> f64 {
        calc_length(self)
    }

    /// Applies the transform to the path.
    #[inline]
    pub fn transform(&mut self, ts: Transform) {
        transform_path(self, ts);
    }

    /// Applies the transform to the path from the specified offset.
    #[inline]
    pub fn transform_from(&mut self, offset: usize, ts: Transform) {
        transform_path(&mut self[offset..], ts);
    }

    /// Returns an iterator over path subpaths.
    #[inline]
    pub fn subpaths(&self) -> SubPathIter {
        SubPathIter {
            path: self,
            index: 0,
        }
    }
}

impl std::ops::Deref for PathData {
    type Target = Vec<PathSegment>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for PathData {
    #[inline]
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
    #[inline]
    pub fn bbox(&self) -> Option<PathBbox> {
        calc_bbox(self)
    }

    /// Calculates path's bounding box with a specified transform.
    ///
    /// This operation is expensive.
    #[inline]
    pub fn bbox_with_transform(
        &self,
        ts: Transform,
        stroke: Option<&super::Stroke>,
    ) -> Option<PathBbox> {
        calc_bbox_with_transform(self, ts, stroke)
    }

    /// Checks that path has a bounding box.
    ///
    /// This operation is expensive.
    #[inline]
    pub fn has_bbox(&self) -> bool {
        has_bbox(self)
    }

    /// Calculates path's length.
    ///
    /// This operation is expensive.
    #[inline]
    pub fn length(&self) -> f64 {
        calc_length(self)
    }
}

impl std::ops::Deref for SubPathData<'_> {
    type Target = [PathSegment];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}


fn calc_bbox(segments: &[PathSegment]) -> Option<PathBbox> {
    if segments.is_empty() {
        return None;
    }

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
                let curve = kurbo::CubicBez::from_points(prev_x, prev_y, x1, y1, x2, y2, x, y);
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

    PathBbox::new(minx, miny, width, height)
}

fn calc_bbox_with_transform(
    segments: &[PathSegment],
    ts: Transform,
    stroke: Option<&super::Stroke>,
) -> Option<PathBbox> {
    if segments.is_empty() {
        return None;
    }

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
                let curve = kurbo::CubicBez::from_points(prev_x, prev_y, x1, y1, x2, y2, x, y);
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
    if let Some(stroke) = stroke {
        let w = stroke.width.value() / 2.0;
        minx -= w;
        miny -= w;
        maxx += w;
        maxy += w;
    }

    let width = maxx - minx;
    let height = maxy - miny;

    PathBbox::new(minx, miny, width, height)
}

fn has_bbox(segments: &[PathSegment]) -> bool {
    if segments.is_empty() {
        return false;
    }

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
                let curve = kurbo::CubicBez::from_points(prev_x, prev_y, x1, y1, x2, y2, x, y);
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
    if segments.is_empty() {
        return 0.0;
    }

    let (mut prev_mx, mut prev_my, mut prev_x, mut prev_y) = {
        if let PathSegment::MoveTo { x, y } = segments[0] {
            (x, y, x, y)
        } else {
            unreachable!();
        }
    };

    fn create_curve_from_line(px: f64, py: f64, x: f64, y: f64) -> kurbo::CubicBez {
        let line = kurbo::Line::new(kurbo::Point::new(px, py), kurbo::Point::new(x, y));
        let p1 = line.eval(0.33);
        let p2 = line.eval(0.66);
        kurbo::CubicBez::from_points(px, py, p1.x, p1.y, p2.x, p2.y, x, y)
    }

    let mut length = 0.0;
    for seg in segments {
        let curve = match *seg {
            PathSegment::MoveTo { x, y } => {
                prev_mx = x;
                prev_my = y;
                prev_x = x;
                prev_y = y;
                continue;
            }
            PathSegment::LineTo { x, y } => {
                create_curve_from_line(prev_x, prev_y, x, y)
            }
            PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                kurbo::CubicBez::from_points(prev_x, prev_y, x1, y1, x2, y2, x, y)
            }
            PathSegment::ClosePath => {
                create_curve_from_line(prev_x, prev_y, prev_mx, prev_my)
            }
        };

        length += curve.arclen(0.5);
        prev_x = curve.p3.x;
        prev_y = curve.p3.y;
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
    #[inline]
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


#[inline]
fn quad_to_curve(px: f64, py: f64, x1: f64, y1: f64, x: f64, y: f64) -> PathSegment {
    #[inline]
    fn calc(n1: f64, n2: f64) -> f64 {
        (n1 + n2 * 2.0) / 3.0
    }

    PathSegment::CurveTo {
        x1: calc(px, x1), y1: calc(py, y1),
        x2:  calc(x, x1), y2:  calc(y, y1),
        x, y,
    }
}


pub(crate) trait CubicBezExt {
    fn from_points(px: f64, py: f64, x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) -> Self;
}

impl CubicBezExt for kurbo::CubicBez {
    fn from_points(px: f64, py: f64, x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) -> Self {
        kurbo::CubicBez {
            p0: kurbo::Point::new(px, py),
            p1: kurbo::Point::new(x1, y1),
            p2: kurbo::Point::new(x2, y2),
            p3: kurbo::Point::new(x, y),
        }
    }
}
