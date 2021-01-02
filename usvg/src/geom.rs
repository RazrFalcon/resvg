// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::{cmp, f64, fmt};

use svgtypes::FuzzyEq;

use crate::{tree, IsValidLength};


/// Line representation.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct Line {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

impl Line {
    /// Creates a new line.
    #[inline]
    pub fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Line {
        Line { x1, y1, x2, y2 }
    }

    /// Calculates the line length.
    #[inline]
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


/// A 2D point representation.
#[derive(Clone, Copy)]
pub struct Point<T> {
    /// Position along the X-axis.
    pub x: T,

    /// Position along the Y-axis.
    pub y: T,
}

impl<T> Point<T> {
    /// Create a new point.
    pub fn new(x: T, y: T) -> Self {
        Point { x, y }
    }
}

impl<T: fmt::Display> fmt::Debug for Point<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Point({} {})", self.x, self.y)
    }
}

impl<T: fmt::Display> fmt::Display for Point<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}


/// A 2D size representation.
///
/// Width and height are guarantee to be > 0.
#[derive(Clone, Copy)]
pub struct Size {
    width: f64,
    height: f64,
}

impl Size {
    /// Creates a new `Size` from values.
    #[inline]
    pub fn new(width: f64, height: f64) -> Option<Self> {
        if width.is_valid_length() && height.is_valid_length() {
            Some(Size { width, height })
        } else {
            None
        }
    }

    /// Returns width.
    #[inline]
    pub fn width(&self) -> f64 {
        self.width
    }

    /// Returns height.
    #[inline]
    pub fn height(&self) -> f64 {
        self.height
    }

    /// Scales current size to specified size.
    #[inline]
    pub fn scale_to(&self, to: Self) -> Self {
        size_scale_f64(*self, to, false)
    }

    /// Expands current size to specified size.
    #[inline]
    pub fn expand_to(&self, to: Self) -> Self {
        size_scale_f64(*self, to, true)
    }

    /// Fits size into a viewbox.
    pub fn fit_view_box(&self, vb: &tree::ViewBox) -> Self {
        let s = vb.rect.size();

        if vb.aspect.align == tree::Align::None {
            s
        } else {
            if vb.aspect.slice {
                self.expand_to(s)
            } else {
                self.scale_to(s)
            }
        }
    }

    /// Converts `Size` to `ScreenSize`.
    #[inline]
    pub fn to_screen_size(&self) -> ScreenSize {
        ScreenSize::new(
            cmp::max(1, self.width().round() as u32),
            cmp::max(1, self.height().round() as u32),
        ).unwrap()
    }

    /// Converts the current size to `Rect` at provided position.
    #[inline]
    pub fn to_rect(&self, x: f64, y: f64) -> Rect {
        Rect::new(x, y, self.width, self.height).unwrap()
    }
}

impl fmt::Debug for Size {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Size({} {})", self.width, self.height)
    }
}

impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl FuzzyEq for Size {
    #[inline]
    fn fuzzy_eq(&self, other: &Self) -> bool {
           self.width.fuzzy_eq(&other.width)
        && self.height.fuzzy_eq(&other.height)
    }
}


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
    #[inline]
    pub fn new(width: u32, height: u32) -> Option<Self> {
        if width > 0 && height > 0 {
            Some(ScreenSize { width, height })
        } else {
            None
        }
    }

    /// Returns width.
    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns height.
    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Returns width and height as a tuple.
    #[inline]
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Scales current size to specified size.
    #[inline]
    pub fn scale_to(&self, to: Self) -> Self {
        size_scale(*self, to, false)
    }

    /// Expands current size to specified size.
    #[inline]
    pub fn expand_to(&self, to: Self) -> Self {
        size_scale(*self, to, true)
    }

    /// Fits size into a viewbox.
    pub fn fit_view_box(&self, vb: &tree::ViewBox) -> Self {
        let s = vb.rect.to_screen_size();

        if vb.aspect.align == tree::Align::None {
            s
        } else {
            if vb.aspect.slice {
                self.expand_to(s)
            } else {
                self.scale_to(s)
            }
        }
    }

    /// Converts the current `ScreenSize` to `Size`.
    #[inline]
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

fn size_scale(
    s1: ScreenSize,
    s2: ScreenSize,
    expand: bool,
) -> ScreenSize {
    let rw = (s2.height as f64 * s1.width as f64 / s1.height as f64).ceil() as u32;
    let with_h = if expand { rw <= s2.width } else { rw >= s2.width };
    if !with_h {
        ScreenSize::new(rw, s2.height).unwrap()
    } else {
        let h = (s2.width as f64 * s1.height as f64 / s1.width as f64).ceil() as u32;
        ScreenSize::new(s2.width, h).unwrap()
    }
}

fn size_scale_f64(
    s1: Size,
    s2: Size,
    expand: bool,
) -> Size {
    let rw = s2.height * s1.width / s1.height;
    let with_h = if expand { rw <= s2.width } else { rw >= s2.width };
    if !with_h {
        Size::new(rw, s2.height).unwrap()
    } else {
        let h = s2.width * s1.height / s1.width;
        Size::new(s2.width, h).unwrap()
    }
}


/// A rect representation.
///
/// Width and height are guarantee to be > 0.
#[derive(Clone, Copy)]
pub struct Rect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl Rect {
    /// Creates a new `Rect` from values.
    #[inline]
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Option<Self> {
        if width.is_valid_length() && height.is_valid_length() {
            Some(Rect { x, y, width, height })
        } else {
            None
        }
    }

    /// Creates a new `Rect` for bounding box calculation.
    ///
    /// Shorthand for `Rect::new(f64::MAX, f64::MAX, 1.0, 1.0)`.
    #[inline]
    pub fn new_bbox() -> Self {
        Rect::new(f64::MAX, f64::MAX, 1.0, 1.0).unwrap()
    }

    /// Returns rect's size.
    #[inline]
    pub fn size(&self) -> Size {
        Size::new(self.width, self.height).unwrap()
    }

    /// Returns rect's X position.
    #[inline]
    pub fn x(&self) -> f64 {
        self.x
    }

    /// Returns rect's Y position.
    #[inline]
    pub fn y(&self) -> f64 {
        self.y
    }

    /// Returns rect's width.
    #[inline]
    pub fn width(&self) -> f64 {
        self.width
    }

    /// Returns rect's height.
    #[inline]
    pub fn height(&self) -> f64 {
        self.height
    }

    /// Returns rect's left edge position.
    #[inline]
    pub fn left(&self) -> f64 {
        self.x
    }

    /// Returns rect's right edge position.
    #[inline]
    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    /// Returns rect's top edge position.
    #[inline]
    pub fn top(&self) -> f64 {
        self.y
    }

    /// Returns rect's bottom edge position.
    #[inline]
    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    /// Translates the rect by the specified offset.
    #[inline]
    pub fn translate(&self, tx: f64, ty: f64) -> Self {
        Rect {
            x: self.x + tx,
            y: self.y + ty,
            width: self.width,
            height: self.height,
        }
    }

    /// Translates the rect to the specified position.
    #[inline]
    pub fn translate_to(&self, x: f64, y: f64) -> Self {
        Rect {
            x,
            y,
            width: self.width,
            height: self.height,
        }
    }

    /// Checks that the rect contains a point.
    #[inline]
    pub fn contains(&self, x: f64, y: f64) -> bool {
        if x < self.x || x > self.x + self.width - 1.0 {
            return false;
        }

        if y < self.y || y > self.y + self.height - 1.0 {
            return false;
        }

        true
    }

    /// Expands the `Rect` to the provided size.
    #[inline]
    pub fn expand(&self, r: Rect) -> Self {
        #[inline]
        fn f64_min(v1: f64, v2: f64) -> f64 {
            if v1 < v2 { v1 } else { v2 }
        }

        #[inline]
        fn f64_max(v1: f64, v2: f64) -> f64 {
            if v1 > v2 { v1 } else { v2 }
        }

        if self.fuzzy_eq(&Rect::new_bbox()) {
            r
        } else {
            let x1 = f64_min(self.x(), r.x());
            let y1 = f64_min(self.y(), r.y());

            let x2 = f64_max(self.right(), r.right());
            let y2 = f64_max(self.bottom(), r.bottom());

            Rect::new(x1, y1, x2 - x1, y2 - y1).unwrap()
        }
    }

    /// Transforms the `Rect` using the provided `bbox`.
    pub fn bbox_transform(&self, bbox: Rect) -> Self {
        let x = self.x() * bbox.width() + bbox.x();
        let y = self.y() * bbox.height() + bbox.y();
        let w = self.width() * bbox.width();
        let h = self.height() * bbox.height();
        Rect::new(x, y, w, h).unwrap()
    }

    /// Transforms the `Rect` using the provided `Transform`.
    ///
    /// This method is expensive.
    pub fn transform(&self, ts: &tree::Transform) -> Option<Self> {
        if !ts.is_default() {
            let path = &[
                tree::PathSegment::MoveTo {
                    x: self.x(), y: self.y()
                },
                tree::PathSegment::LineTo {
                    x: self.right(), y: self.y()
                },
                tree::PathSegment::LineTo {
                    x: self.right(), y: self.bottom()
                },
                tree::PathSegment::LineTo {
                    x: self.x(), y: self.bottom()
                },
                tree::PathSegment::ClosePath,
            ];

            tree::SubPathData(path).bbox_with_transform(*ts, None)
        } else {
            Some(*self)
        }
    }

    /// Returns rect's size in screen units.
    #[inline]
    pub fn to_screen_size(&self) -> ScreenSize {
        self.size().to_screen_size()
    }

    /// Returns rect in screen units.
    #[inline]
    pub fn to_screen_rect(&self) -> ScreenRect {
        ScreenRect::new(
            self.x() as i32,
            self.y() as i32,
            cmp::max(1, self.width().round() as u32),
            cmp::max(1, self.height().round() as u32),
        ).unwrap()
    }
}

impl FuzzyEq for Rect {
    #[inline]
    fn fuzzy_eq(&self, other: &Self) -> bool {
           self.x.fuzzy_eq(&other.x)
        && self.y.fuzzy_eq(&other.y)
        && self.width.fuzzy_eq(&other.width)
        && self.height.fuzzy_eq(&other.height)
    }
}

impl fmt::Debug for Rect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Rect({} {} {} {})", self.x, self.y, self.width, self.height)
    }
}

impl fmt::Display for Rect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
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
    #[inline]
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Option<Self> {
        if width > 0 && height > 0 {
            Some(ScreenRect { x, y, width, height })
        } else {
            None
        }
    }

    /// Returns rect's size.
    #[inline]
    pub fn size(&self) -> ScreenSize {
        // Can't fail, because `ScreenSize` is always valid.
        ScreenSize::new(self.width, self.height).unwrap()
    }

    /// Returns rect's X position.
    #[inline]
    pub fn x(&self) -> i32 {
        self.x
    }

    /// Returns rect's Y position.
    #[inline]
    pub fn y(&self) -> i32 {
        self.y
    }

    /// Returns rect's width.
    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns rect's height.
    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Returns rect's left edge position.
    #[inline]
    pub fn left(&self) -> i32 {
        self.x
    }

    /// Returns rect's right edge position.
    #[inline]
    pub fn right(&self) -> i32 {
        self.x + self.width as i32
    }

    /// Returns rect's top edge position.
    #[inline]
    pub fn top(&self) -> i32 {
        self.y
    }

    /// Returns rect's bottom edge position.
    #[inline]
    pub fn bottom(&self) -> i32 {
        self.y + self.height as i32
    }

    /// Translates the rect by the specified offset.
    #[inline]
    pub fn translate(&self, tx: i32, ty: i32) -> Self {
        ScreenRect {
            x: self.x + tx,
            y: self.y + ty,
            width: self.width,
            height: self.height,
        }
    }

    /// Translates the rect to the specified position.
    #[inline]
    pub fn translate_to(&self, x: i32, y: i32) -> Self {
        ScreenRect {
            x,
            y,
            width: self.width,
            height: self.height,
        }
    }

    /// Checks that rect contains a point.
    #[inline]
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
    #[inline]
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
    #[inline]
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

    #[test]
    fn bbox_transform_1() {
        let r = Rect::new(10.0, 20.0, 30.0, 40.0).unwrap();
        assert!(r.bbox_transform(Rect::new(0.2, 0.3, 0.4, 0.5).unwrap())
            .fuzzy_eq(&Rect::new(4.2, 10.3, 12.0, 20.0).unwrap()));
    }
}
