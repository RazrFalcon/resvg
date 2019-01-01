// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! 2D geometric primitives.

use std::cmp;
use std::f64;
use std::fmt;

// external
use usvg::{
    FuzzyEq
};
pub use usvg::{
    Line,
    Point,
    Rect,
    Size,
};

pub(crate) use usvg::{
    f64_bound,
};

// self
use utils;


/// A 2D screen size representation.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq)]
pub struct ScreenSize {
    pub width: u32,
    pub height: u32,
}

impl ScreenSize {
    /// Creates a new `ScreenSize` from values.
    pub fn new(width: u32, height: u32) -> Self {
        ScreenSize { width, height }
    }

    /// Scales current size to specified size.
    pub fn scale_to(&self, to: Self) -> Self {
        size_scale(*self, to, false)
    }

    /// Expands current size to specified size.
    pub fn expand_to(&self, to: ScreenSize) -> ScreenSize {
        size_scale(*self, to, true)
    }

    /// Converts the current `ScreenSize` to `Size`.
    pub fn to_size(&self) -> Size {
        Size::new(self.width as f64, self.height as f64)
    }
}

impl From<(u32, u32)> for ScreenSize {
    fn from(v: (u32, u32)) -> Self {
        ScreenSize::new(v.0, v.1)
    }
}

impl fmt::Debug for ScreenSize {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ScreenSize({} {})", self.width, self.height)
    }
}

impl fmt::Display for ScreenSize {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}


/// Additional `Size` methods.
pub trait SizeExt {
    /// Converts `Size` to `ScreenSize`.
    fn to_screen_size(&self) -> ScreenSize;
}

impl SizeExt for Size {
    fn to_screen_size(&self) -> ScreenSize {
        ScreenSize::new(self.width.round() as u32, self.height.round() as u32)
    }
}

fn size_scale(s1: ScreenSize, s2: ScreenSize, expand: bool) -> ScreenSize {
    let rw = (s2.height as f64 * s1.width as f64 / s1.height as f64).ceil() as u32;
    let with_h = if expand { rw <= s2.width } else { rw >= s2.width };
    if !with_h {
        ScreenSize::new(rw, s2.height)
    } else {
        let h = (s2.width as f64 * s1.height as f64 / s1.width as f64).ceil() as u32;
        ScreenSize::new(s2.width, h)
    }
}


/// Additional `Rect` methods.
pub trait RectExt: Sized {
    /// Creates a new `Rect` for bounding box calculation.
    ///
    /// Shorthand for `Rect::new(f64::MAX, f64::MAX, 1.0, 1.0)`.
    fn new_bbox() -> Self;

    /// Expands the `Rect` to the provided size.
    fn expand(&mut self, r: Rect);

    /// Transforms the `Rect` using the provided `bbox`.
    fn bbox_transform(&self, bbox: Rect) -> Option<Self>;

    /// Transforms the `Rect` using the provided `Transform`.
    ///
    /// This method is expensive.
    fn transform(&self, ts: &usvg::Transform) -> Self;

    /// Returns rect's size in screen units.
    fn to_screen_size(&self) -> ScreenSize;

    /// Returns rect in screen units.
    fn to_screen_rect(&self) -> ScreenRect;
}

impl RectExt for Rect {
    fn new_bbox() -> Self {
        (f64::MAX, f64::MAX, 1.0, 1.0).into()
    }

    fn expand(&mut self, r: Rect) {
        if !r.is_valid() {
            return;
        }

        if self.fuzzy_eq(&Rect::new_bbox()) {
            *self = r;
        } else {
            let x1 = f64_min(self.x, r.x);
            let y1 = f64_min(self.y, r.y);

            let x2 = f64_max(self.right(), r.right());
            let y2 = f64_max(self.bottom(), r.bottom());

            self.x = x1;
            self.y = y1;
            self.width = x2 - x1;
            self.height = y2 - y1;
        }
    }

    fn bbox_transform(&self, bbox: Rect) -> Option<Self> {
        if bbox.is_valid() {
            let x = self.x * bbox.width + bbox.x;
            let y = self.y * bbox.height + bbox.y;
            let w = self.width * bbox.width;
            let h = self.height * bbox.height;

            Some(Self::new(x, y, w, h))
        } else {
            None
        }
    }

    fn transform(&self, ts: &usvg::Transform) -> Self {
        if !ts.is_default() {
            let path = &[
                usvg::PathSegment::MoveTo {
                    x: self.x, y: self.y
                },
                usvg::PathSegment::LineTo {
                    x: self.right(), y: self.y
                },
                usvg::PathSegment::LineTo {
                    x: self.right(), y: self.bottom()
                },
                usvg::PathSegment::LineTo {
                    x: self.x, y: self.bottom()
                },
                usvg::PathSegment::ClosePath,
            ];

            utils::path_bbox(path, None, ts)
        } else {
            *self
        }
    }

    fn to_screen_size(&self) -> ScreenSize {
        self.size().to_screen_size()
    }

    fn to_screen_rect(&self) -> ScreenRect {
        ScreenRect::new(self.x as i32, self.y as i32,
                        cmp::max(0, self.width as i32) as u32,
                        cmp::max(0, self.height as i32) as u32)
    }
}


/// A 2D screen rect representation.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq)]
pub struct ScreenRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl ScreenRect {
    /// Creates a new `Rect` from values.
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        ScreenRect { x, y, width, height }
    }

    /// Returns rect's size.
    pub fn size(&self) -> ScreenSize {
        ScreenSize::new(self.width, self.height)
    }

    /// Returns rect's left edge position.
    pub fn left(&self) -> i32 {
        self.x
    }

    /// Returns rect's right edge position.
    pub fn right(&self) -> i32 {
        self.x + self.width as i32
    }

    /// Returns rect's top edge position.
    pub fn top(&self) -> i32 {
        self.y
    }

    /// Returns rect's bottom edge position.
    pub fn bottom(&self) -> i32 {
        self.y + self.height as i32
    }

    /// Checks that rect contains a point.
    pub fn contains(&self, x: i32, y: i32) -> bool {
        if x < self.x || x > self.x + self.width as i32 - 1 {
            return false;
        }

        if y < self.y || y > self.y + self.height as i32 - 1 {
            return false;
        }

        true
    }

    /// Fits the current rect into the specified bounds.
    pub fn fit_to_rect(&self, bounds: ScreenRect) -> Self {
        let mut r = *self;

        if r.x < 0 { r.x = 0; }
        if r.y < 0 { r.y = 0; }

        if r.right() > bounds.width as i32 {
            r.width = cmp::max(0, bounds.width as i32 - r.x) as u32;
        }

        if r.bottom() > bounds.height as i32 {
            r.height = cmp::max(0, bounds.height as i32 - r.y) as u32;
        }

        r
    }

    /// Converts into `Rect`.
    pub fn to_rect(&self) -> Rect {
        Rect::new(self.x as f64, self.y as f64, self.width as f64, self.height as f64)
    }

    /// Checks that the rect has a valid size.
    pub fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0
    }
}

impl fmt::Debug for ScreenRect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ScreenRect({} {} {} {})", self.x, self.y, self.width, self.height)
    }
}

impl fmt::Display for ScreenRect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<(i32, i32, u32, u32)> for ScreenRect {
    fn from(v: (i32, i32, u32, u32)) -> Self {
        Self::new(v.0, v.1, v.2, v.3)
    }
}


#[inline]
fn f64_min(v1: f64, v2: f64) -> f64 {
    if v1 < v2 { v1 } else { v2 }
}

#[inline]
fn f64_max(v1: f64, v2: f64) -> f64 {
    if v1 > v2 { v1 } else { v2 }
}


#[cfg(test)]
mod tests {
    use super::*;
    use usvg::FuzzyEq;

    #[test]
    fn bbox_transform_1() {
        let r = Rect::new(10.0, 20.0, 30.0, 40.0);
        assert!(r.bbox_transform(Rect::new(0.2, 0.3, 0.4, 0.5))
                 .fuzzy_eq(&Rect::new(4.2, 10.3, 12.0, 20.0)));
    }
}
