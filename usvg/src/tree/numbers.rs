// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.


use std::fmt;

use svgtypes::{FuzzyEq, FuzzyZero};

use crate::IsValidLength;
use crate::geom::f64_bound;


macro_rules! wrap {
    ($name:ident) => {
        impl From<f64> for $name {
            #[inline]
            fn from(n: f64) -> Self {
                $name::new(n)
            }
        }

        impl PartialEq for $name {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                self.0.fuzzy_eq(&other.0)
            }
        }
    };
}


/// A normalized value.
///
/// Just like `f64` but immutable and guarantee to be in a 0..1 range.
#[derive(Clone, Copy, Debug)]
pub struct NormalizedValue(f64);

impl NormalizedValue {
    /// Creates a new `NormalizedValue` value.
    #[inline]
    pub fn new(n: f64) -> Self {
        debug_assert!(n.is_finite());
        debug_assert!(n >= 0.0 && n <= 1.0);
        NormalizedValue(f64_bound(0.0, n, 1.0))
    }

    /// Returns an underlying value.
    #[inline]
    pub fn value(&self) -> f64 {
        self.0
    }
}

impl std::ops::Mul<NormalizedValue> for NormalizedValue {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: NormalizedValue) -> Self::Output {
        NormalizedValue::new(self.0 * rhs.0)
    }
}

impl Default for NormalizedValue {
    #[inline]
    fn default() -> Self {
        NormalizedValue::new(1.0)
    }
}

wrap!(NormalizedValue);

/// An alias to `NormalizedValue`.
pub type Opacity = NormalizedValue;

/// An alias to `NormalizedValue`.
pub type StopOffset = NormalizedValue;


/// A `stroke-width` value.
///
/// Just like `f64` but immutable and guarantee to be >0.0.
#[derive(Clone, Copy, Debug)]
pub struct StrokeWidth(f64);

impl StrokeWidth {
    /// Creates a new `StrokeWidth` value.
    #[inline]
    pub fn new(n: f64) -> Self {
        debug_assert!(n.is_finite());
        debug_assert!(n.is_valid_length());

        // Fallback to `1.0` when value is invalid.
        let n = if !n.is_valid_length() { 1.0 } else { n };

        StrokeWidth(n)
    }

    /// Returns an underlying value.
    #[inline]
    pub fn value(&self) -> f64 {
        self.0
    }
}

impl Default for StrokeWidth {
    #[inline]
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
    #[inline]
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
    #[inline]
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
    #[inline]
    pub fn new(n: f64) -> Self {
        debug_assert!(n.is_finite());
        debug_assert!(n > 0.0);

        // Fallback to `12.0` when value is invalid.
        let n = if !n.is_valid_length() { 12.0 } else { n };

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
    #[inline]
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
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.0.is_fuzzy_zero()
    }
}

wrap!(PositiveNumber);

impl fmt::Display for PositiveNumber {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}


/// A non-zero `f64`.
///
/// Just like `f64` but immutable and guarantee to never be zero.
#[derive(Clone, Copy, Debug)]
pub struct NonZeroF64(f64);

impl NonZeroF64 {
    /// Creates a new `NonZeroF64` value.
    #[inline]
    pub fn new(n: f64) -> Option<Self> {
        if n.is_fuzzy_zero() {
            None
        } else {
            Some(NonZeroF64(n))
        }
    }

    /// Returns an underlying value.
    #[inline]
    pub fn value(&self) -> f64 {
        self.0
    }
}
