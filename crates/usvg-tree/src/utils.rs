// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Some useful utilities.

use crate::{Align, AspectRatio, Rect, Size, Transform};

/// Converts `viewBox` to `Transform`.
pub fn view_box_to_transform(view_box: Rect, aspect: AspectRatio, img_size: Size) -> Transform {
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
    Transform::new(sx, 0.0, 0.0, sy, tx, ty)
}

/// Returns object aligned position.
pub fn aligned_pos(align: Align, x: f64, y: f64, w: f64, h: f64) -> (f64, f64) {
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
