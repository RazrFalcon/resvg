// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! 2D geometric primitives.

use std::cmp;
use std::f64;
use std::fmt;

// external
use usvg;
pub use usvg::{
    Line,
    Rect,
    Size,
};

pub(crate) use usvg::{
    f64_bound,
};

// self
use crate::utils;


/// A 2D screen size representation.
///
/// Width and height are guarantee to be > 0.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq)]
pub struct ScreenSize {
    width: u32,
    height: u32,
}

impl ScreenSize {
    /// Creates a new `ScreenSize` from values.
    pub fn new(width: u32, height: u32) -> Option<Self> {
        if width > 0 && height > 0 {
            Some(ScreenSize { width, height })
        } else {
            None
        }
    }

    /// Returns width.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns height.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Scales current size to specified size.
    pub fn scale_to(&self, to: Self) -> Self {
        size_scale(*self, to, false)
    }

    /// Expands current size to specified size.
    pub fn expand_to(&self, to: Self) -> Self {
        size_scale(*self, to, true)
    }

    /// Converts the current `ScreenSize` to `Size`.
    pub fn to_size(&self) -> Size {
        // Can't fail, because `ScreenSize` is always valid.
        Size::new(self.width as f64, self.height as f64).unwrap()
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
        ScreenSize::new(
            cmp::max(1, self.width().round() as u32),
            cmp::max(1, self.height().round() as u32),
        ).unwrap()
    }
}

fn size_scale(s1: ScreenSize, s2: ScreenSize, expand: bool) -> ScreenSize {
    let rw = (s2.height as f64 * s1.width as f64 / s1.height as f64).ceil() as u32;
    let with_h = if expand { rw <= s2.width } else { rw >= s2.width };
    if !with_h {
        ScreenSize::new(rw, s2.height).unwrap()
    } else {
        let h = (s2.width as f64 * s1.height as f64 / s1.width as f64).ceil() as u32;
        ScreenSize::new(s2.width, h).unwrap()
    }
}


/// Additional `Rect` methods.
pub trait RectExt: Sized {
    /// Transforms the `Rect` using the provided `bbox`.
    fn bbox_transform(&self, bbox: Rect) -> Self;

    /// Transforms the `Rect` using the provided `Transform`.
    ///
    /// This method is expensive.
    fn transform(&self, ts: &usvg::Transform) -> Option<Self>;

    /// Returns rect's size in screen units.
    fn to_screen_size(&self) -> ScreenSize;

    /// Returns rect in screen units.
    fn to_screen_rect(&self) -> ScreenRect;
}

impl RectExt for Rect {
    fn bbox_transform(&self, bbox: Rect) -> Self {
        let x = self.x() * bbox.width() + bbox.x();
        let y = self.y() * bbox.height() + bbox.y();
        let w = self.width() * bbox.width();
        let h = self.height() * bbox.height();
        Rect::new(x, y, w, h).unwrap()
    }

    fn transform(&self, ts: &usvg::Transform) -> Option<Self> {
        if !ts.is_default() {
            let path = &[
                usvg::PathSegment::MoveTo {
                    x: self.x(), y: self.y()
                },
                usvg::PathSegment::LineTo {
                    x: self.right(), y: self.y()
                },
                usvg::PathSegment::LineTo {
                    x: self.right(), y: self.bottom()
                },
                usvg::PathSegment::LineTo {
                    x: self.x(), y: self.bottom()
                },
                usvg::PathSegment::ClosePath,
            ];

            utils::path_bbox(path, None, Some(*ts))
        } else {
            Some(*self)
        }
    }

    fn to_screen_size(&self) -> ScreenSize {
        self.size().to_screen_size()
    }

    fn to_screen_rect(&self) -> ScreenRect {
        ScreenRect::new(
            self.x() as i32,
            self.y() as i32,
            cmp::max(1, self.width().round() as u32),
            cmp::max(1, self.height().round() as u32),
        ).unwrap()
    }
}


/// A 2D screen rect representation.
///
/// Width and height are guarantee to be > 0.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq)]
pub struct ScreenRect {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl ScreenRect {
    /// Creates a new `Rect` from values.
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Option<Self> {
        if width > 0 && height > 0 {
            Some(ScreenRect { x, y, width, height })
        } else {
            None
        }
    }

    /// Returns rect's size.
    pub fn size(&self) -> ScreenSize {
        // Can't fail, because `ScreenSize` is always valid.
        ScreenSize::new(self.width, self.height).unwrap()
    }

    /// Returns rect's X position.
    pub fn x(&self) -> i32 {
        self.x
    }

    /// Returns rect's Y position.
    pub fn y(&self) -> i32 {
        self.y
    }

    /// Returns rect's width.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns rect's height.
    pub fn height(&self) -> u32 {
        self.height
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

    /// Translates the rect by the specified offset.
    pub fn translate(&self, tx: i32, ty: i32) -> Self {
        ScreenRect {
            x: self.x + tx,
            y: self.y + ty,
            width: self.width,
            height: self.height,
        }
    }

    /// Translates the rect to the specified position.
    pub fn translate_to(&self, x: i32, y: i32) -> Self {
        ScreenRect {
            x,
            y,
            width: self.width,
            height: self.height,
        }
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
            r.width = cmp::max(1, bounds.width as i32 - r.x) as u32;
        }

        if r.bottom() > bounds.height as i32 {
            r.height = cmp::max(1, bounds.height as i32 - r.y) as u32;
        }

        r
    }

    /// Converts into `Rect`.
    pub fn to_rect(&self) -> Rect {
        // Can't fail, because `ScreenRect` is always valid.
        Rect::new(self.x as f64, self.y as f64, self.width as f64, self.height as f64).unwrap()
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


#[cfg(test)]
mod tests {
    use super::*;
    use usvg::FuzzyEq;

    #[test]
    fn bbox_transform_1() {
        let r = Rect::new(10.0, 20.0, 30.0, 40.0).unwrap();
        assert!(r.bbox_transform(Rect::new(0.2, 0.3, 0.4, 0.5).unwrap())
                 .fuzzy_eq(&Rect::new(4.2, 10.3, 12.0, 20.0).unwrap()));
    }
}
