// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//use std::ops::Deref;

// external
use svgdom::{
    FuzzyEq,
    FuzzyZero,
};

// self
use geom::f64_bound;


macro_rules! wrap {
    ($name:ident) => {
        impl From<f64> for $name {
            fn from(n: f64) -> Self {
                $name::new(n)
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                self.0.fuzzy_eq(&other.0)
            }
        }
    };
}


/// An opacity value.
///
/// Just like `f64` but immutable and guarantee to be in the 0..1 range.
#[derive(Clone, Copy, Debug)]
pub struct Opacity(f64);

impl Opacity {
    /// Creates a new `Opacity` value.
    pub fn new(n: f64) -> Self {
        debug_assert!(n.is_finite());
        debug_assert!(n >= 0.0 && n <= 1.0);
        Opacity(f64_bound(0.0, n, 1.0))
    }

    /// Returns an underlying value.
    #[inline]
    pub fn value(&self) -> f64 {
        self.0
    }
}

impl Default for Opacity {
    fn default() -> Self {
        Opacity::new(1.0)
    }
}

wrap!(Opacity);


/// An alias to `Opacity`.
pub type StopOffset = Opacity;

/// An alias to `Opacity`.
pub type CompositingCoefficient = Opacity;


/// A `stroke-width` value.
///
/// Just like `f64` but immutable and guarantee to be >0.0.
#[derive(Clone, Copy, Debug)]
pub struct StrokeWidth(f64);

impl StrokeWidth {
    /// Creates a new `StrokeWidth` value.
    pub fn new(n: f64) -> Self {
        debug_assert!(n.is_finite());
        debug_assert!(n > 0.0);

        // Fallback to `1.0` when value is invalid.
        let n = if !(n > 0.0) { 1.0 } else { n };

        StrokeWidth(n)
    }

    /// Returns an underlying value.
    #[inline]
    pub fn value(&self) -> f64 {
        self.0
    }
}

impl Default for StrokeWidth {
    fn default() -> Self {
        StrokeWidth::new(1.0)
    }
}

wrap!(StrokeWidth);


/// A `stroke-miterlimit` value.
///
/// Just like `f64` but immutable and guarantee to be >=1.0.
#[derive(Clone, Copy, Debug)]
pub struct StrokeMiterlimit(f64);

impl StrokeMiterlimit {
    /// Creates a new `StrokeMiterlimit` value.
    pub fn new(n: f64) -> Self {
        debug_assert!(n.is_finite());
        debug_assert!(n >= 1.0);

        let n = if !(n >= 1.0) { 1.0 } else { n };

        StrokeMiterlimit(n)
    }

    /// Returns an underlying value.
    #[inline]
    pub fn value(&self) -> f64 {
        self.0
    }
}

impl Default for StrokeMiterlimit {
    fn default() -> Self {
        StrokeMiterlimit::new(4.0)
    }
}

wrap!(StrokeMiterlimit);


/// A `font-size` value.
///
/// Just like `f64` but immutable and guarantee to be >0.0.
#[derive(Clone, Copy, Debug)]
pub struct FontSize(f64);

impl FontSize {
    /// Creates a new `FontSize` value.
    pub fn new(n: f64) -> Self {
        debug_assert!(n.is_finite());
        debug_assert!(n > 0.0);

        // Fallback to `12.0` when value is invalid.
        let n = if !(n > 0.0) { 12.0 } else { n };

        FontSize(n)
    }

    /// Returns an underlying value.
    #[inline]
    pub fn value(&self) -> f64 {
        self.0
    }
}

wrap!(FontSize);


/// A positive number.
///
/// Just like `f64` but immutable and guarantee to be >=0.0
#[derive(Clone, Copy, Debug)]
pub struct PositiveNumber(f64);

impl PositiveNumber {
    /// Creates a new `PositiveNumber` value.
    pub fn new(n: f64) -> Self {
        debug_assert!(n.is_finite());
        debug_assert!(!n.is_sign_negative());

        // Fallback to 0.0 when value is invalid.
        let n = if n.is_sign_negative() { 0.0 } else { n };

        PositiveNumber(n)
    }

    /// Returns an underlying value.
    #[inline]
    pub fn value(&self) -> f64 {
        self.0
    }

    /// Checks that the current number is zero.
    pub fn is_zero(&self) -> bool {
        self.0.is_fuzzy_zero()
    }
}

wrap!(PositiveNumber);
