// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::f64;
use std::fmt;

use svgdom::FuzzyEq;

use crate::IsValidLength;


/// Bounds `f64` number.
#[inline]
pub fn f64_bound(min: f64, val: f64, max: f64) -> f64 {
    debug_assert!(min.is_finite());
    debug_assert!(val.is_finite());
    debug_assert!(max.is_finite());

    if val > max {
        return max;
    } else if val < min {
        return min;
    }

    val
}


/// Line representation.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug)]
pub struct Line {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

impl Line {
    /// Creates a new line.
    pub fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Line {
        Line { x1, y1, x2, y2 }
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
    pub fn new(width: f64, height: f64) -> Option<Self> {
        if width.is_valid_length() && height.is_valid_length() {
            Some(Size { width, height })
        } else {
            None
        }
    }

    /// Returns width.
    pub fn width(&self) -> f64 {
        self.width
    }

    /// Returns height.
    pub fn height(&self) -> f64 {
        self.height
    }

    /// Converts the current size to `Rect` at provided position.
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
    fn fuzzy_eq(&self, other: &Self) -> bool {
           self.width.fuzzy_eq(&other.width)
        && self.height.fuzzy_eq(&other.height)
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
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Option<Self> {
        if width.is_valid_length() && height.is_valid_length() {
            Some(Rect { x, y, width, height })
        } else {
            None
        }
    }

    /// Returns rect's size.
    pub fn size(&self) -> Size {
        Size::new(self.width, self.height).unwrap()
    }

    /// Returns rect's X position.
    pub fn x(&self) -> f64 {
        self.x
    }

    /// Returns rect's Y position.
    pub fn y(&self) -> f64 {
        self.y
    }

    /// Returns rect's width.
    pub fn width(&self) -> f64 {
        self.width
    }

    /// Returns rect's height.
    pub fn height(&self) -> f64 {
        self.height
    }

    /// Returns rect's left edge position.
    pub fn left(&self) -> f64 {
        self.x
    }

    /// Returns rect's right edge position.
    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    /// Returns rect's top edge position.
    pub fn top(&self) -> f64 {
        self.y
    }

    /// Returns rect's bottom edge position.
    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    /// Translates the rect by the specified offset.
    pub fn translate(&self, tx: f64, ty: f64) -> Self {
        Rect {
            x: self.x + tx,
            y: self.y + ty,
            width: self.width,
            height: self.height,
        }
    }

    /// Translates the rect to the specified position.
    pub fn translate_to(&self, x: f64, y: f64) -> Self {
        Rect {
            x,
            y,
            width: self.width,
            height: self.height,
        }
    }

    /// Checks that the rect contains a point.
    pub fn contains(&self, x: f64, y: f64) -> bool {
        if x < self.x || x > self.x + self.width - 1.0 {
            return false;
        }

        if y < self.y || y > self.y + self.height - 1.0 {
            return false;
        }

        true
    }
}

impl FuzzyEq for Rect {
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
