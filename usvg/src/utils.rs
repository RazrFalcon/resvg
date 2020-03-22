// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Some useful utilities.

use crate::{geom::*, tree};

/// Converts `viewBox` to `Transform`.
pub fn view_box_to_transform(
    view_box: Rect,
    aspect: tree::AspectRatio,
    img_size: Size,
) -> tree::Transform {
    let vr = view_box;

    let sx = img_size.width() / vr.width();
    let sy = img_size.height() / vr.height();

    let (sx, sy) = if aspect.align == tree::Align::None {
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
    tree::Transform::new(sx, 0.0, 0.0, sy, tx, ty)
}

/// Returns object aligned position.
pub fn aligned_pos(align: tree::Align, x: f64, y: f64, w: f64, h: f64) -> (f64, f64) {
    match align {
        tree::Align::None => (x, y),
        tree::Align::XMinYMin => (x, y),
        tree::Align::XMidYMin => (x + w / 2.0, y),
        tree::Align::XMaxYMin => (x + w, y),
        tree::Align::XMinYMid => (x, y + h / 2.0),
        tree::Align::XMidYMid => (x + w / 2.0, y + h / 2.0),
        tree::Align::XMaxYMid => (x + w, y + h / 2.0),
        tree::Align::XMinYMax => (x, y + h),
        tree::Align::XMidYMax => (x + w / 2.0, y + h),
        tree::Align::XMaxYMax => (x + w, y + h),
    }
}

pub(crate) fn file_extension(path: &std::path::Path) -> Option<&str> {
    if let Some(ext) = path.extension() {
        ext.to_str()
    } else {
        None
    }
}
