// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Some useful utilities.

use crate::{Align, AspectRatio, Rect, ScreenSize, Size, Transform, ViewBox};

// TODO: https://github.com/rust-lang/rust/issues/44095
/// Bounds `f64` number.
#[inline]
pub(crate) fn f64_bound(min: f64, val: f64, max: f64) -> f64 {
    debug_assert!(min.is_finite());
    debug_assert!(val.is_finite());
    debug_assert!(max.is_finite());

    if val > max {
        max
    } else if val < min {
        min
    } else {
        val
    }
}

/// Converts `viewBox` to `Transform`.
pub fn view_box_to_transform(
    view_box: Rect,
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
            if sx < sy { sy } else { sx }
        } else {
            if sx > sy { sy } else { sx }
        };

        (s, s)
    };

    let x = -vr.x() * sx;
    let y = -vr.y() * sy;
    let w = img_size.width() - vr.width() * sx;
    let h = img_size.height() - vr.height() * sy;

    let (tx, ty) = aligned_pos(aspect.align, x, y, w, h);
    Transform::new(sx, 0.0, 0.0, sy, tx, ty)
}

/// Converts `viewBox` to `Transform` with an optional clip rectangle.
///
/// Unlike `view_box_to_transform`, returns an optional clip rectangle
/// that should be applied before rendering the image.
pub fn view_box_to_transform_with_clip(
    view_box: &ViewBox,
    img_size: ScreenSize,
) -> (Transform, Option<Rect>) {
    let r = view_box.rect;

    let new_size = img_size.fit_view_box(view_box);

    let (tx, ty, clip) = if view_box.aspect.slice {
        let (dx, dy) = aligned_pos(
            view_box.aspect.align,
            0.0, 0.0, new_size.width() as f64 - r.width(), new_size.height() as f64 - r.height(),
        );

        (r.x() - dx, r.y() - dy, Some(r))
    } else {
        let (dx, dy) = aligned_pos(
            view_box.aspect.align,
            r.x(), r.y(), r.width() - new_size.width() as f64, r.height() - new_size.height() as f64,
        );

        (dx, dy, None)
    };

    let sx = new_size.width() as f64 / img_size.width() as f64;
    let sy = new_size.height() as f64 / img_size.height() as f64;
    let ts = Transform::new(sx, 0.0, 0.0, sy, tx, ty);

    (ts, clip)
}

/// Returns object aligned position.
pub fn aligned_pos(
    align: Align,
    x: f64, y: f64, w: f64, h: f64,
) -> (f64, f64) {
    match align {
        Align::None     => (x,           y          ),
        Align::XMinYMin => (x,           y          ),
        Align::XMidYMin => (x + w / 2.0, y          ),
        Align::XMaxYMin => (x + w,       y          ),
        Align::XMinYMid => (x,           y + h / 2.0),
        Align::XMidYMid => (x + w / 2.0, y + h / 2.0),
        Align::XMaxYMid => (x + w,       y + h / 2.0),
        Align::XMinYMax => (x,           y + h      ),
        Align::XMidYMax => (x + w / 2.0, y + h      ),
        Align::XMaxYMax => (x + w,       y + h      ),
    }
}

pub(crate) fn file_extension(path: &std::path::Path) -> Option<&str> {
    path.extension().and_then(|e| e.to_str())
}
