// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::f64;

// external
use euclid;

// self
use tree;


/// Bounds `f64` number.
#[inline]
pub fn f64_bound(min: f64, val: f64, max: f64) -> f64 {
    if val > max {
        return max;
    } else if val < min {
        return min;
    }

    val
}

#[inline]
fn f64_min(v1: f64, v2: f64) -> f64 {
    if v1 < v2 {
        v1
    } else {
        v2
    }
}


/// Line representation.
#[allow(missing_docs)]
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Line {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

impl Line {
    /// Creates a new line.
    pub fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Line {
        Line {
            x1,
            y1,
            x2,
            y2,
        }
    }

    /// Calculates the line length.
    pub fn length(&self) -> f64 {
        let x = self.x2 - self.x1;
        let y = self.y2 - self.y1;
        (x*x + y*y).sqrt()
    }

    /// Sets the line length.
    pub fn set_length(&mut self, len: f64) {
        let x = self.x2 - self.x1;
        let y = self.y2 - self.y1;
        let len2 = (x*x + y*y).sqrt();
        let line = Line {
            x1: self.x1, y1: self.y1,
            x2: self.x1 + x/len2, y2: self.y1 + y/len2
        };

        self.x2 = self.x1 + (line.x2 - line.x1) * len;
        self.y2 = self.y1 + (line.y2 - line.y1) * len;
    }
}

/// Alias for euclid::Point2D<f64>.
pub type Point = euclid::Point2D<f64>;

/// Alias for euclid::Size2D<f64>.
pub type Size = euclid::Size2D<f64>;

/// Alias for euclid::Size2D<u32>.
pub type ScreenSize = euclid::Size2D<u32>;

/// Alias for euclid::Rect<f64>.
pub type Rect = euclid::Rect<f64>;

/// Additional `Size` methods.
pub trait SizeExt {
    /// Converts `Size` to `ScreenSize`.
    fn to_screen_size(&self) -> ScreenSize;
}

impl SizeExt for Size {
    fn to_screen_size(&self) -> ScreenSize {
        ScreenSize::new(self.width as u32, self.height as u32)
    }
}

/// Additional `Rect` methods.
pub trait RectExt {
    /// Creates `Rect` from values.
    fn from_xywh(x: f64, y: f64, w: f64, h: f64) -> Self;

    /// Creates a new `Rect` for bounding box calculation.
    ///
    /// Shorthand for `Rect::from_xywh(f64::MAX, f64::MAX, 1.0, 1.0)`.
    fn new_bbox() -> Self;

    /// Returns `x` position.
    fn x(&self) -> f64;

    /// Returns `y` position.
    fn y(&self) -> f64;

    /// Returns width.
    fn width(&self) -> f64;

    /// Returns height.
    fn height(&self) -> f64;

    /// Expands the `Rect` to the specified size.
    fn expand(&mut self, r: Rect);

    /// Returns transformed rect.
    fn transform(&self, ts: tree::Transform) -> Self;

    /// Returns rect's size in screen units.
    fn to_screen_size(&self) -> ScreenSize;
}

impl RectExt for Rect {
    fn from_xywh(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self::new(Point::new(x, y), Size::new(w, h))
    }

    fn new_bbox() -> Self {
        Self::from_xywh(f64::MAX, f64::MAX, 1.0, 1.0)
    }

    fn x(&self) -> f64 {
        self.origin.x
    }

    fn y(&self) -> f64 {
        self.origin.y
    }

    fn width(&self) -> f64 {
        self.size.width
    }

    fn height(&self) -> f64 {
        self.size.height
    }

    fn expand(&mut self, r: Rect) {
        if r.width() <= 0.0 || r.height() <= 0.0 {
            return;
        }

        self.origin.x = f64_min(self.x(), r.x());
        self.origin.y = f64_min(self.y(), r.y());

        if self.x() + self.width() < r.x() + r.width() {
            self.size.width = r.x() + r.width() - self.x();
        }

        if self.y() + self.height() < r.y() + r.height() {
            self.size.height = r.y() + r.height() - self.y();
        }
    }

    fn transform(&self, ts: tree::Transform) -> Self {
        let (x, y) = ts.apply(self.x(), self.y());
        let (sx, sy) = ts.get_scale();
        let (w, h) = (self.width() * sx, self.height() * sy);
        Self::from_xywh(x, y, w, h)
    }

    fn to_screen_size(&self) -> ScreenSize {
        self.size.to_screen_size()
    }
}
