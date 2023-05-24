// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use strict_num::ApproxEqUlps;
pub use tiny_skia_path::{NonZeroRect, Rect, Size, Transform};

use crate::AspectRatio;

/// Approximate zero equality comparisons.
pub trait ApproxZeroUlps: ApproxEqUlps {
    /// Checks if the number is approximately zero.
    fn approx_zero_ulps(&self, ulps: <Self::Flt as strict_num::Ulps>::U) -> bool;
}

impl ApproxZeroUlps for f32 {
    fn approx_zero_ulps(&self, ulps: i32) -> bool {
        self.approx_eq_ulps(&0.0, ulps)
    }
}

impl ApproxZeroUlps for f64 {
    fn approx_zero_ulps(&self, ulps: i64) -> bool {
        self.approx_eq_ulps(&0.0, ulps)
    }
}

/// Checks that the current number is > 0.
pub trait IsValidLength {
    /// Checks that the current number is > 0.
    fn is_valid_length(&self) -> bool;
}

impl IsValidLength for f32 {
    #[inline]
    fn is_valid_length(&self) -> bool {
        *self > 0.0 && self.is_finite()
    }
}

impl IsValidLength for f64 {
    #[inline]
    fn is_valid_length(&self) -> bool {
        *self > 0.0 && self.is_finite()
    }
}

/// View box.
#[derive(Clone, Copy, Debug)]
pub struct ViewBox {
    /// Value of the `viewBox` attribute.
    pub rect: NonZeroRect,

    /// Value of the `preserveAspectRatio` attribute.
    pub aspect: AspectRatio,
}

/// A bounding box calculator.
#[derive(Clone, Copy, Debug)]
pub struct BBox {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

impl From<Rect> for BBox {
    fn from(r: Rect) -> Self {
        Self {
            left: r.left(),
            top: r.top(),
            right: r.right(),
            bottom: r.bottom(),
        }
    }
}

impl From<NonZeroRect> for BBox {
    fn from(r: NonZeroRect) -> Self {
        Self {
            left: r.left(),
            top: r.top(),
            right: r.right(),
            bottom: r.bottom(),
        }
    }
}

impl Default for BBox {
    fn default() -> Self {
        Self {
            left: f32::MAX,
            top: f32::MAX,
            right: f32::MIN,
            bottom: f32::MIN,
        }
    }
}

impl BBox {
    /// Checks if the bounding box is default, i.e. invalid.
    pub fn is_default(&self) -> bool {
        self.left == f32::MAX
            && self.top == f32::MAX
            && self.right == f32::MIN
            && self.bottom == f32::MIN
    }

    /// Expand the bounding box to the specified bounds.
    #[must_use]
    pub fn expand(&self, r: impl Into<Self>) -> Self {
        self.expand_impl(r.into())
    }

    fn expand_impl(&self, r: Self) -> Self {
        Self {
            left: self.left.min(r.left),
            top: self.top.min(r.top),
            right: self.right.max(r.right),
            bottom: self.bottom.max(r.bottom),
        }
    }

    /// Transforms the bounding box.
    pub fn transform(&self, ts: tiny_skia_path::Transform) -> Option<Self> {
        self.to_rect()?.transform(ts).map(Self::from)
    }

    /// Converts a bounding box into [`Rect`].
    pub fn to_rect(&self) -> Option<Rect> {
        if !self.is_default() {
            Rect::from_ltrb(self.left, self.top, self.right, self.bottom)
        } else {
            None
        }
    }

    /// Converts a bounding box into [`NonZeroRect`].
    pub fn to_non_zero_rect(&self) -> Option<NonZeroRect> {
        if !self.is_default() {
            NonZeroRect::from_ltrb(self.left, self.top, self.right, self.bottom)
        } else {
            None
        }
    }
}

/// Some useful utilities.
pub mod utils {
    use super::*;
    use crate::Align;

    /// Converts `viewBox` to `Transform`.
    pub fn view_box_to_transform(
        view_box: NonZeroRect,
        aspect: AspectRatio,
        img_size: Size,
    ) -> Transform {
        let vr = view_box;

        let sx = img_size.width() / vr.width();
        let sy = img_size.height() / vr.height();

        let (sx, sy) = if aspect.align == Align::None {
            (sx, sy)
        } else {
            let s = if aspect.slice {
                if sx < sy {
                    sy
                } else {
                    sx
                }
            } else {
                if sx > sy {
                    sy
                } else {
                    sx
                }
            };

            (s, s)
        };

        let x = -vr.x() * sx;
        let y = -vr.y() * sy;
        let w = img_size.width() - vr.width() * sx;
        let h = img_size.height() - vr.height() * sy;

        let (tx, ty) = aligned_pos(aspect.align, x, y, w, h);
        Transform::from_row(sx, 0.0, 0.0, sy, tx, ty)
    }

    /// Returns object aligned position.
    pub fn aligned_pos(align: Align, x: f32, y: f32, w: f32, h: f32) -> (f32, f32) {
        match align {
            Align::None => (x, y),
            Align::XMinYMin => (x, y),
            Align::XMidYMin => (x + w / 2.0, y),
            Align::XMaxYMin => (x + w, y),
            Align::XMinYMid => (x, y + h / 2.0),
            Align::XMidYMid => (x + w / 2.0, y + h / 2.0),
            Align::XMaxYMid => (x + w, y + h / 2.0),
            Align::XMinYMax => (x, y + h),
            Align::XMidYMax => (x + w / 2.0, y + h),
            Align::XMaxYMax => (x + w, y + h),
        }
    }
}
