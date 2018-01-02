// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.


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

#[inline]
fn f64_max(v1: f64, v2: f64) -> f64 {
    if v1 > v2 {
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


/// Size representation.
#[allow(missing_docs)]
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Size {
    pub w: f64,
    pub h: f64,
}

impl Default for Size {
    fn default() -> Size {
        Size {
            w: 0.0,
            h: 0.0,
        }
    }
}

impl From<(f64, f64)> for Size {
    fn from(v: (f64, f64)) -> Self {
        Self::new(v.0, v.1)
    }
}

impl Size {
    /// Creates a new `Size`.
    pub fn new(w: f64, h: f64) -> Size {
        debug_assert!(w.is_sign_positive());
        debug_assert!(h.is_sign_positive());

        Size {
            w,
            h,
        }
    }
}


/// Rect representation.
#[allow(missing_docs)]
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Default for Rect {
    fn default() -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        }
    }
}

impl From<(f64, f64, f64, f64)> for Rect {
    fn from(v: (f64, f64, f64, f64)) -> Self {
        Self::new(v.0, v.1, v.2, v.3)
    }
}

impl Rect {
    /// Creates a new `Rect`.
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Rect {
        debug_assert!(w.is_sign_positive());
        debug_assert!(h.is_sign_positive());

        Rect {
            x,
            y,
            w,
            h,
        }
    }

    /// Returns the size of the `Rect`.
    pub fn size(&self) -> Size {
        Size {
            w: self.w,
            h: self.h,
        }
    }

    /// Expands the `Rect` to the specified size.
    pub fn expand(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.x = f64_min(self.x, x);
        self.y = f64_min(self.y, y);
        self.w = f64_max(self.w, w);
        self.h = f64_max(self.h, h);
    }

    /// Expands the `Rect` to the specified size.
    pub fn expand_from_rect(&mut self, r: &Rect) {
        self.expand(r.x, r.y, r.w, r.h);
    }
}
