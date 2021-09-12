// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use float_cmp::ApproxEqUlps;

use crate::{Align, AspectRatio};


/// A trait for fuzzy/approximate equality comparisons of float numbers.
pub trait FuzzyEq<Rhs: ?Sized = Self> {
    /// Returns `true` if values are approximately equal.
    fn fuzzy_eq(&self, other: &Rhs) -> bool;

    /// Returns `true` if values are not approximately equal.
    #[inline]
    fn fuzzy_ne(&self, other: &Rhs) -> bool {
        !self.fuzzy_eq(other)
    }
}

impl<T: FuzzyEq> FuzzyEq for Vec<T> {
    fn fuzzy_eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }

        for (a, b) in self.iter().zip(other.iter()) {
            if a.fuzzy_ne(b) {
                return false;
            }
        }

        true
    }
}

/// A trait for fuzzy/approximate comparisons of float numbers.
pub trait FuzzyZero: FuzzyEq {
    /// Returns `true` if the number is approximately zero.
    fn is_fuzzy_zero(&self) -> bool;
}

impl FuzzyEq for f32 {
    #[inline]
    fn fuzzy_eq(&self, other: &f32) -> bool {
        self.approx_eq_ulps(other, 4)
    }
}

impl FuzzyEq for f64 {
    #[inline]
    fn fuzzy_eq(&self, other: &f64) -> bool {
        self.approx_eq_ulps(other, 4)
    }
}

impl FuzzyZero for f32 {
    #[inline]
    fn is_fuzzy_zero(&self) -> bool {
        self.fuzzy_eq(&0.0)
    }
}

impl FuzzyZero for f64 {
    #[inline]
    fn is_fuzzy_zero(&self) -> bool {
        self.fuzzy_eq(&0.0)
    }
}


/// Checks that the current number is > 0.
pub trait IsValidLength {
    /// Checks that the current number is > 0.
    fn is_valid_length(&self) -> bool;
}

impl IsValidLength for f64 {
    #[inline]
    fn is_valid_length(&self) -> bool {
        *self > 0.0
    }
}


/// Converts `Rect` into bbox `Transform`.
pub trait TransformFromBBox: Sized {
    /// Converts `Rect` into bbox `Transform`.
    fn from_bbox(bbox: Rect) -> Self;
}

impl TransformFromBBox for Transform {
    #[inline]
    fn from_bbox(bbox: Rect) -> Self {
        Self::new(bbox.width(), 0.0, 0.0, bbox.height(), bbox.x(), bbox.y())
    }
}


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


// TODO: remove
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

impl<T: std::fmt::Display> std::fmt::Debug for Point<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Point({} {})", self.x, self.y)
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Point<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
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
    pub fn fit_view_box(&self, vb: &ViewBox) -> Self {
        let s = vb.rect.size();

        if vb.aspect.align == Align::None {
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
            std::cmp::max(1, self.width().round() as u32),
            std::cmp::max(1, self.height().round() as u32),
        ).unwrap()
    }

    /// Converts the current size to `Rect` at provided position.
    #[inline]
    pub fn to_rect(&self, x: f64, y: f64) -> Rect {
        Rect::new(x, y, self.width, self.height).unwrap()
    }
}

impl std::fmt::Debug for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Size({} {})", self.width, self.height)
    }
}

impl std::fmt::Display for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
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
    pub fn fit_view_box(&self, vb: &ViewBox) -> Self {
        let s = vb.rect.to_screen_size();

        if vb.aspect.align == Align::None {
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

impl std::fmt::Debug for ScreenSize {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "ScreenSize({} {})", self.width, self.height)
    }
}

impl std::fmt::Display for ScreenSize {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
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


/// A path bbox representation.
///
/// The same as [`Rect`], but width or height are allowed to be zero
/// to represent horizontal or vertical lines.
#[derive(Clone, Copy)]
pub struct PathBbox {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl PathBbox {
    /// Creates a new `PathBbox` from values.
    #[inline]
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Option<Self> {
        if width.is_valid_length() || height.is_valid_length() {
            Some(PathBbox { x, y, width, height })
        } else {
            None
        }
    }

    /// Creates a new `PathBbox` for bounding box calculation.
    ///
    /// Shorthand for `PathBbox::new(f64::MAX, f64::MAX, 1.0, 1.0)`.
    #[inline]
    pub fn new_bbox() -> Self {
        PathBbox::new(f64::MAX, f64::MAX, 1.0, 1.0).unwrap()
    }

    /// Returns X position.
    #[inline]
    pub fn x(&self) -> f64 {
        self.x
    }

    /// Returns Y position.
    #[inline]
    pub fn y(&self) -> f64 {
        self.y
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

    /// Returns left edge position.
    #[inline]
    pub fn left(&self) -> f64 {
        self.x
    }

    /// Returns right edge position.
    #[inline]
    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    /// Returns top edge position.
    #[inline]
    pub fn top(&self) -> f64 {
        self.y
    }

    /// Returns bottom edge position.
    #[inline]
    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    /// Expands the `PathBbox` to the provided size.
    #[inline]
    pub fn expand(&self, r: PathBbox) -> Self {
        if self.fuzzy_eq(&PathBbox::new_bbox()) {
            r
        } else {
            let x1 = self.x().min(r.x());
            let y1 = self.y().min(r.y());

            let x2 = self.right().max(r.right());
            let y2 = self.bottom().max(r.bottom());

            PathBbox::new(x1, y1, x2 - x1, y2 - y1).unwrap()
        }
    }

    /// Transforms the `PathBbox` using the provided `bbox`.
    pub fn bbox_transform(&self, bbox: Rect) -> Self {
        let x = self.x() * bbox.width() + bbox.x();
        let y = self.y() * bbox.height() + bbox.y();
        let w = self.width() * bbox.width();
        let h = self.height() * bbox.height();
        PathBbox::new(x, y, w, h).unwrap()
    }

    /// Transforms the `PathBbox` using the provided `Transform`.
    ///
    /// This method is expensive.
    pub fn transform(&self, ts: &Transform) -> Option<Self> {
        use crate::pathdata::{PathSegment, SubPathData};

        if !ts.is_default() {
            let path = &[
                PathSegment::MoveTo {
                    x: self.x(), y: self.y()
                },
                PathSegment::LineTo {
                    x: self.right(), y: self.y()
                },
                PathSegment::LineTo {
                    x: self.right(), y: self.bottom()
                },
                PathSegment::LineTo {
                    x: self.x(), y: self.bottom()
                },
                PathSegment::ClosePath,
            ];

            SubPathData(path).bbox_with_transform(*ts, None)
        } else {
            Some(*self)
        }
    }

    /// Converts into a [`Rect`].
    pub fn to_rect(&self) -> Option<Rect> {
        Rect::new(self.x, self.y, self.width, self.height)
    }
}

impl FuzzyEq for PathBbox {
    #[inline]
    fn fuzzy_eq(&self, other: &Self) -> bool {
           self.x.fuzzy_eq(&other.x)
        && self.y.fuzzy_eq(&other.y)
        && self.width.fuzzy_eq(&other.width)
        && self.height.fuzzy_eq(&other.height)
    }
}

impl std::fmt::Debug for PathBbox {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "PathBbox({} {} {} {})", self.x, self.y, self.width, self.height)
    }
}

impl std::fmt::Display for PathBbox {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
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
        if self.fuzzy_eq(&Rect::new_bbox()) {
            r
        } else {
            let x1 = self.x().min(r.x());
            let y1 = self.y().min(r.y());

            let x2 = self.right().max(r.right());
            let y2 = self.bottom().max(r.bottom());

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
    pub fn transform(&self, ts: &Transform) -> Option<Self> {
        use crate::pathdata::{PathSegment, SubPathData};

        if !ts.is_default() {
            let path = &[
                PathSegment::MoveTo {
                    x: self.x(), y: self.y()
                },
                PathSegment::LineTo {
                    x: self.right(), y: self.y()
                },
                PathSegment::LineTo {
                    x: self.right(), y: self.bottom()
                },
                PathSegment::LineTo {
                    x: self.x(), y: self.bottom()
                },
                PathSegment::ClosePath,
            ];

            SubPathData(path).bbox_with_transform(*ts, None).and_then(|r| r.to_rect())
        } else {
            Some(*self)
        }
    }

    /// Returns rect's size in screen units.
    #[inline]
    pub fn to_path_bbox(&self) -> PathBbox {
        // Never fails, because `Rect` is more strict than `PathBbox`.
        PathBbox::new(self.x, self.y, self.width, self.height).unwrap()
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
            std::cmp::max(1, self.width().round() as u32),
            std::cmp::max(1, self.height().round() as u32),
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

impl std::fmt::Debug for Rect {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Rect({} {} {} {})", self.x, self.y, self.width, self.height)
    }
}

impl std::fmt::Display for Rect {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
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
            r.width = std::cmp::max(1, bounds.width as i32 - r.x) as u32;
        }

        if r.bottom() > bounds.height as i32 {
            r.height = std::cmp::max(1, bounds.height as i32 - r.y) as u32;
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

impl std::fmt::Debug for ScreenRect {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "ScreenRect({} {} {} {})", self.x, self.y, self.width, self.height)
    }
}

impl std::fmt::Display for ScreenRect {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}


/// Representation of the [`<transform>`] type.
///
/// [`<transform>`]: https://www.w3.org/TR/SVG2/coords.html#InterfaceSVGTransform
#[derive(Clone, Copy, PartialEq, Debug)]
#[allow(missing_docs)]
pub struct Transform {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl From<svgtypes::Transform> for Transform {
    fn from(ts: svgtypes::Transform) -> Self {
        Transform::new(ts.a, ts.b, ts.c, ts.d, ts.e, ts.f)
    }
}

impl Transform {
    /// Constructs a new transform.
    #[inline]
    pub fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Transform { a, b, c, d, e, f, }
    }

    /// Constructs a new translate transform.
    #[inline]
    pub fn new_translate(x: f64, y: f64) -> Self {
        Transform::new(1.0, 0.0, 0.0, 1.0, x, y)
    }

    /// Constructs a new scale transform.
    #[inline]
    pub fn new_scale(sx: f64, sy: f64) -> Self {
        Transform::new(sx, 0.0, 0.0, sy, 0.0, 0.0)
    }

    /// Constructs a new rotate transform.
    #[inline]
    pub fn new_rotate(angle: f64) -> Self {
        let v = angle.to_radians();
        let a =  v.cos();
        let b =  v.sin();
        let c = -b;
        let d =  a;
        Transform::new(a, b, c, d, 0.0, 0.0)
    }

    /// Translates the current transform.
    #[inline]
    pub fn translate(&mut self, x: f64, y: f64) {
        self.append(&Transform::new_translate(x, y));
    }

    /// Scales the current transform.
    #[inline]
    pub fn scale(&mut self, sx: f64, sy: f64) {
        self.append(&Transform::new_scale(sx, sy));
    }

    /// Rotates the current transform.
    #[inline]
    pub fn rotate(&mut self, angle: f64) {
        self.append(&Transform::new_rotate(angle));
    }

    /// Rotates the current transform at the specified position.
    #[inline]
    pub fn rotate_at(&mut self, angle: f64, x: f64, y: f64) {
        self.translate(x, y);
        self.rotate(angle);
        self.translate(-x, -y);
    }

    /// Appends transform to the current transform.
    #[inline]
    pub fn append(&mut self, other: &Transform) {
        let ts = multiply(self, other);
        self.a = ts.a;
        self.b = ts.b;
        self.c = ts.c;
        self.d = ts.d;
        self.e = ts.e;
        self.f = ts.f;
    }

    /// Prepends transform to the current transform.
    #[inline]
    pub fn prepend(&mut self, other: &Transform) {
        let ts = multiply(other, self);
        self.a = ts.a;
        self.b = ts.b;
        self.c = ts.c;
        self.d = ts.d;
        self.e = ts.e;
        self.f = ts.f;
    }

    /// Returns `true` if the transform is default, aka `(1 0 0 1 0 0)`.
    pub fn is_default(&self) -> bool {
           self.a.fuzzy_eq(&1.0)
        && self.b.fuzzy_eq(&0.0)
        && self.c.fuzzy_eq(&0.0)
        && self.d.fuzzy_eq(&1.0)
        && self.e.fuzzy_eq(&0.0)
        && self.f.fuzzy_eq(&0.0)
    }

    /// Returns transform's translate part.
    #[inline]
    pub fn get_translate(&self) -> (f64, f64) {
        (self.e, self.f)
    }

    /// Returns transform's scale part.
    #[inline]
    pub fn get_scale(&self) -> (f64, f64) {
        let x_scale = (self.a * self.a + self.c * self.c).sqrt();
        let y_scale = (self.b * self.b + self.d * self.d).sqrt();
        (x_scale, y_scale)
    }

    /// Applies transform to selected coordinates.
    #[inline]
    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        let new_x = self.a * x + self.c * y + self.e;
        let new_y = self.b * x + self.d * y + self.f;
        (new_x, new_y)
    }

    /// Applies transform to selected coordinates.
    #[inline]
    pub fn apply_to(&self, x: &mut f64, y: &mut f64) {
        let tx = *x;
        let ty = *y;
        *x = self.a * tx + self.c * ty + self.e;
        *y = self.b * tx + self.d * ty + self.f;
    }
}

#[inline(never)]
fn multiply(ts1: &Transform, ts2: &Transform) -> Transform {
    Transform {
        a: ts1.a * ts2.a + ts1.c * ts2.b,
        b: ts1.b * ts2.a + ts1.d * ts2.b,
        c: ts1.a * ts2.c + ts1.c * ts2.d,
        d: ts1.b * ts2.c + ts1.d * ts2.d,
        e: ts1.a * ts2.e + ts1.c * ts2.f + ts1.e,
        f: ts1.b * ts2.e + ts1.d * ts2.f + ts1.f,
    }
}

impl Default for Transform {
    #[inline]
    fn default() -> Transform {
        Transform::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
    }
}

impl FuzzyEq for Transform {
    fn fuzzy_eq(&self, other: &Self) -> bool {
           self.a.fuzzy_eq(&other.a)
        && self.b.fuzzy_eq(&other.b)
        && self.c.fuzzy_eq(&other.c)
        && self.d.fuzzy_eq(&other.d)
        && self.e.fuzzy_eq(&other.e)
        && self.f.fuzzy_eq(&other.f)
    }
}


/// View box.
#[derive(Clone, Copy, Debug)]
pub struct ViewBox {
    /// Value of the `viewBox` attribute.
    pub rect: Rect,

    /// Value of the `preserveAspectRatio` attribute.
    pub aspect: AspectRatio,
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
