// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{FuzzyEq, FuzzyZero};
use crate::utils::f64_bound;


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
        debug_assert!((0.0..=1.0).contains(&n));
        NormalizedValue(f64_bound(0.0, n, 1.0))
    }

    /// Returns an underlying value.
    #[inline]
    pub fn value(&self) -> f64 {
        self.0
    }

    /// Converts an underlying value into a 0..255 range.
    #[inline]
    pub fn to_u8(&self) -> u8 {
        (self.0 * 255.0).ceil() as u8
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

impl std::fmt::Display for PositiveNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
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
