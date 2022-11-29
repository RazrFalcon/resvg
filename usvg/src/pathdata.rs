// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::rc::Rc;

use kurbo::{ParamCurve, ParamCurveArclen, ParamCurveExtrema};

use crate::{FuzzyZero, PathBbox, Rect, Transform};

/// A path command.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PathCommand {
    MoveTo,
    LineTo,
    CurveTo,
    ClosePath,
}

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
pub struct PathData {
    commands: Vec<PathCommand>,
    points: Vec<f64>,
}

/// A reference-counted `PathData`.
///
/// `PathData` is usually pretty big and it's expensive to clone it,
/// so we are using `Rc`.
pub(crate) type SharedPathData = Rc<PathData>;

impl PathData {
    /// Creates a new path.
    #[inline]
    pub fn new() -> Self {
        PathData::default()
    }

    /// Returns `true` if the path contains no segment.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Returns the number of segments in the path.
    #[inline]
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Returns a slice of the path commands.
    #[inline]
    pub fn commands(&self) -> &[PathCommand] {
        &self.commands
    }

    /// Returns a slice of the path points.
    #[inline]
    pub fn points(&self) -> &[f64] {
        &self.points
    }

    /// Clears the path.
    pub fn clear(&mut self) {
        self.commands.clear();
        self.points.clear();
    }

    /// Shrinks the capacity of the path as much as possible.
    pub fn shrink_to_fit(&mut self) {
        self.commands.shrink_to_fit();
        self.points.shrink_to_fit();
    }

    /// Creates a path from a rect.
    #[inline]
    pub fn from_rect(rect: Rect) -> Self {
        let mut path = PathData::default();
        path.push_rect(rect);
        path
    }

    /// Pushes a MoveTo segment to the path.
    #[inline]
    pub fn push_move_to(&mut self, x: f64, y: f64) {
        self.commands.push(PathCommand::MoveTo);
        self.points.push(x);
        self.points.push(y);
    }

    /// Pushes a LineTo segment to the path.
    #[inline]
    pub fn push_line_to(&mut self, x: f64, y: f64) {
        self.commands.push(PathCommand::LineTo);
        self.points.push(x);
        self.points.push(y);
    }

    /// Pushes a QuadTo segment to the path.
    ///
    /// Will be converted into cubic curve.
    #[inline]
    pub fn push_quad_to(&mut self, x1: f64, y1: f64, x: f64, y: f64) {
        #[inline]
        fn calc(n1: f64, n2: f64) -> f64 {
            (n1 + n2 * 2.0) / 3.0
        }

        let (px, py) = self.last_pos();
        self.push_curve_to(calc(px, x1), calc(py, y1), calc(x, x1), calc(y, y1), x, y)
    }

    /// Pushes a CurveTo segment to the path.
    #[inline]
    pub fn push_curve_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) {
        self.commands.push(PathCommand::CurveTo);
        self.points.push(x1);
        self.points.push(y1);
        self.points.push(x2);
        self.points.push(y2);
        self.points.push(x);
        self.points.push(y);
    }

    /// Pushes an ArcTo segment to the path.
    ///
    /// Arc will be converted into cubic curves.
    pub fn push_arc_to(
        &mut self,
        rx: f64,
        ry: f64,
        x_axis_rotation: f64,
        large_arc: bool,
        sweep: bool,
        x: f64,
        y: f64,
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
        self.commands.push(PathCommand::ClosePath);
    }

    /// Pushes a rect to the path.
    #[inline]
    pub fn push_rect(&mut self, rect: Rect) {
        self.push_move_to(rect.x(), rect.y());
        self.push_line_to(rect.right(), rect.y());
        self.push_line_to(rect.right(), rect.bottom());
        self.push_line_to(rect.x(), rect.bottom());
        self.push_close_path();
    }

    /// Pushes a path to the path.
    #[inline]
    pub fn push_path(&mut self, path: &PathData) {
        self.commands.extend_from_slice(&path.commands);
        self.points.extend_from_slice(&path.points);
    }

    #[inline]
    fn last_pos(&self) -> (f64, f64) {
        let seg = self.commands.last().expect("path must not be empty");
        match seg {
            PathCommand::ClosePath => {
                panic!("the previous segment must be M/L/C")
            }
            _ => {
                let index = self.points.len() - 2;
                (self.points[index], self.points[index + 1])
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
        transform_path(&mut self.points, ts);
    }

    /// Applies the transform to the path from the specified offset.
    #[inline]
    pub fn transform_from(&mut self, offset: usize, ts: Transform) {
        let mut points_offset = 0;
        for command in self.commands().iter().take(offset) {
            match command {
                PathCommand::MoveTo | PathCommand::LineTo => points_offset += 2,
                PathCommand::CurveTo => points_offset += 6,
                PathCommand::ClosePath => {}
            }
        }

        transform_path(&mut self.points[points_offset..], ts);
    }

    /// Returns an iterator over path segments.
    #[inline]
    pub fn segments(&self) -> PathSegmentsIter {
        PathSegmentsIter {
            path: self,
            cmd_index: 0,
            points_index: 0,
        }
    }
}

/// A path segments iterator.
#[allow(missing_debug_implementations)]
#[derive(Clone)]
pub struct PathSegmentsIter<'a> {
    path: &'a PathData,
    cmd_index: usize,
    points_index: usize,
}

impl<'a> Iterator for PathSegmentsIter<'a> {
    type Item = PathSegment;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cmd_index < self.path.commands.len() {
            let verb = self.path.commands[self.cmd_index];
            self.cmd_index += 1;

            match verb {
                PathCommand::MoveTo => {
                    self.points_index += 2;
                    Some(PathSegment::MoveTo {
                        x: self.path.points[self.points_index - 2],
                        y: self.path.points[self.points_index - 1],
                    })
                }
                PathCommand::LineTo => {
                    self.points_index += 2;
                    Some(PathSegment::LineTo {
                        x: self.path.points[self.points_index - 2],
                        y: self.path.points[self.points_index - 1],
                    })
                }
                PathCommand::CurveTo => {
                    self.points_index += 6;
                    Some(PathSegment::CurveTo {
                        x1: self.path.points[self.points_index - 6],
                        y1: self.path.points[self.points_index - 5],
                        x2: self.path.points[self.points_index - 4],
                        y2: self.path.points[self.points_index - 3],
                        x: self.path.points[self.points_index - 2],
                        y: self.path.points[self.points_index - 1],
                    })
                }
                PathCommand::ClosePath => Some(PathSegment::ClosePath),
            }
        } else {
            None
        }
    }
}

fn calc_bbox(path: &PathData) -> Option<PathBbox> {
    if path.is_empty() {
        return None;
    }

    let mut prev_x = path.points[0];
    let mut prev_y = path.points[1];
    let mut minx = prev_x;
    let mut miny = prev_y;
    let mut maxx = prev_x;
    let mut maxy = prev_y;

    for seg in path.segments() {
        match seg {
            PathSegment::MoveTo { x, y } | PathSegment::LineTo { x, y } => {
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
            PathSegment::CurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                let curve = kurbo::CubicBez::from_points(prev_x, prev_y, x1, y1, x2, y2, x, y);
                let r = curve.bounding_box();

                if r.x0 < minx {
                    minx = r.x0;
                }
                if r.x1 > maxx {
                    maxx = r.x1;
                }
                if r.y0 < miny {
                    miny = r.y0;
                }
                if r.y1 > maxy {
                    maxy = r.y1;
                }

                prev_x = x;
                prev_y = y;
            }
            PathSegment::ClosePath => {}
        }
    }

    let width = maxx - minx;
    let height = maxy - miny;

    PathBbox::new(minx, miny, width, height)
}

fn calc_bbox_with_transform(
    path: &PathData,
    ts: Transform,
    stroke: Option<&super::Stroke>,
) -> Option<PathBbox> {
    if path.is_empty() {
        return None;
    }

    let mut prev_x = 0.0;
    let mut prev_y = 0.0;
    let mut minx = 0.0;
    let mut miny = 0.0;
    let mut maxx = 0.0;
    let mut maxy = 0.0;

    if let Some(PathSegment::MoveTo { x, y }) = TransformedPath::new(path, ts).next() {
        prev_x = x;
        prev_y = y;
        minx = x;
        miny = y;
        maxx = x;
        maxy = y;
    }

    for seg in TransformedPath::new(path, ts) {
        match seg {
            PathSegment::MoveTo { x, y } | PathSegment::LineTo { x, y } => {
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
            PathSegment::CurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                let curve = kurbo::CubicBez::from_points(prev_x, prev_y, x1, y1, x2, y2, x, y);
                let r = curve.bounding_box();

                if r.x0 < minx {
                    minx = r.x0;
                }
                if r.x1 > maxx {
                    maxx = r.x1;
                }
                if r.y0 < miny {
                    miny = r.y0;
                }
                if r.y1 > maxy {
                    maxy = r.y1;
                }

                prev_x = x;
                prev_y = y;
            }
            PathSegment::ClosePath => {}
        }
    }

    // TODO: find a better way
    // It's an approximation, but it's better than nothing.
    if let Some(stroke) = stroke {
        let w = stroke.width.get()
            / if ts.is_default() {
                2.0
            } else {
                2.0 / (ts.a * ts.d - ts.b * ts.c).abs().sqrt()
            };
        minx -= w;
        miny -= w;
        maxx += w;
        maxy += w;
    }

    let width = maxx - minx;
    let height = maxy - miny;

    PathBbox::new(minx, miny, width, height)
}

fn has_bbox(path: &PathData) -> bool {
    if path.is_empty() {
        return false;
    }

    let mut prev_x = path.points[0];
    let mut prev_y = path.points[1];
    let mut minx = prev_x;
    let mut miny = prev_y;
    let mut maxx = prev_x;
    let mut maxy = prev_y;

    for seg in path.segments() {
        match seg {
            PathSegment::MoveTo { x, y } | PathSegment::LineTo { x, y } => {
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
            PathSegment::CurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                let curve = kurbo::CubicBez::from_points(prev_x, prev_y, x1, y1, x2, y2, x, y);
                let r = curve.bounding_box();

                if r.x0 < minx {
                    minx = r.x0;
                }
                if r.x1 > maxx {
                    maxx = r.x1;
                }
                if r.x0 < miny {
                    miny = r.y0;
                }
                if r.y1 > maxy {
                    maxy = r.y1;
                }

                prev_x = x;
                prev_y = y;
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

fn calc_length(path: &PathData) -> f64 {
    if path.is_empty() {
        return 0.0;
    }

    let mut prev_mx = path.points[0];
    let mut prev_my = path.points[1];
    let mut prev_x = prev_mx;
    let mut prev_y = prev_my;

    fn create_curve_from_line(px: f64, py: f64, x: f64, y: f64) -> kurbo::CubicBez {
        let line = kurbo::Line::new(kurbo::Point::new(px, py), kurbo::Point::new(x, y));
        let p1 = line.eval(0.33);
        let p2 = line.eval(0.66);
        kurbo::CubicBez::from_points(px, py, p1.x, p1.y, p2.x, p2.y, x, y)
    }

    let mut length = 0.0;
    for seg in path.segments() {
        let curve = match seg {
            PathSegment::MoveTo { x, y } => {
                prev_mx = x;
                prev_my = y;
                prev_x = x;
                prev_y = y;
                continue;
            }
            PathSegment::LineTo { x, y } => create_curve_from_line(prev_x, prev_y, x, y),
            PathSegment::CurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => kurbo::CubicBez::from_points(prev_x, prev_y, x1, y1, x2, y2, x, y),
            PathSegment::ClosePath => create_curve_from_line(prev_x, prev_y, prev_mx, prev_my),
        };

        length += curve.arclen(0.5);
        prev_x = curve.p3.x;
        prev_y = curve.p3.y;
    }

    length
}

// TODO: port tiny-skia logic
fn transform_path(points: &mut [f64], ts: Transform) {
    if points.is_empty() {
        return;
    }

    if ts.is_default() {
        return;
    }

    for p in points.chunks_exact_mut(2) {
        let (x, y) = ts.apply(p[0], p[1]);
        p[0] = x;
        p[1] = y;
    }
}

/// An iterator over transformed path segments.
#[allow(missing_debug_implementations)]
pub struct TransformedPath<'a> {
    iter: PathSegmentsIter<'a>,
    ts: Transform,
}

impl<'a> TransformedPath<'a> {
    /// Creates a new `TransformedPath` iterator.
    #[inline]
    pub fn new(path: &'a PathData, ts: Transform) -> Self {
        TransformedPath {
            iter: path.segments(),
            ts,
        }
    }
}

impl<'a> Iterator for TransformedPath<'a> {
    type Item = PathSegment;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|segment| match segment {
            PathSegment::MoveTo { x, y } => {
                let (x, y) = self.ts.apply(x, y);
                PathSegment::MoveTo { x, y }
            }
            PathSegment::LineTo { x, y } => {
                let (x, y) = self.ts.apply(x, y);
                PathSegment::LineTo { x, y }
            }
            PathSegment::CurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                let (x1, y1) = self.ts.apply(x1, y1);
                let (x2, y2) = self.ts.apply(x2, y2);
                let (x, y) = self.ts.apply(x, y);
                PathSegment::CurveTo {
                    x1,
                    y1,
                    x2,
                    y2,
                    x,
                    y,
                }
            }
            PathSegment::ClosePath => PathSegment::ClosePath,
        })
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
