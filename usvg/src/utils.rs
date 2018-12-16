// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Some useful utilities.

// external
use svgdom::{
    Align,
    AspectRatio,
    Transform,
};

// self
use geom::*;


/// Converts `viewBox` to `Transform`.
pub fn view_box_to_transform(
    view_box: Rect,
    aspect: AspectRatio,
    img_size: Size,
) -> Transform {
    let vr = view_box;

    let sx = img_size.width / vr.width;
    let sy = img_size.height / vr.height;

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

    let x = -vr.x * sx;
    let y = -vr.y * sy;
    let w = img_size.width - vr.width * sx;
    let h = img_size.height - vr.height * sy;

    let pos = aligned_pos(aspect.align, x, y, w, h);
    Transform::new(sx, 0.0, 0.0, sy, pos.x, pos.y)
}

/// Returns object aligned position.
pub fn aligned_pos(align: Align, x: f64, y: f64, w: f64, h: f64) -> Point {
    match align {
        Align::None     => Point::new(x,           y          ),
        Align::XMinYMin => Point::new(x,           y          ),
        Align::XMidYMin => Point::new(x + w / 2.0, y          ),
        Align::XMaxYMin => Point::new(x + w,       y          ),
        Align::XMinYMid => Point::new(x,           y + h / 2.0),
        Align::XMidYMid => Point::new(x + w / 2.0, y + h / 2.0),
        Align::XMaxYMid => Point::new(x + w,       y + h / 2.0),
        Align::XMinYMax => Point::new(x,           y + h      ),
        Align::XMidYMax => Point::new(x + w / 2.0, y + h      ),
        Align::XMaxYMax => Point::new(x + w,       y + h      ),
    }
}
