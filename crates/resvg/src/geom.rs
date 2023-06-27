// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/// Fits the current rect into the specified bounds.
pub fn fit_to_rect(
    r: tiny_skia::IntRect,
    bounds: tiny_skia::IntRect,
) -> Option<tiny_skia::IntRect> {
    let mut left = r.left();
    if left < bounds.left() {
        left = bounds.left();
    }

    let mut top = r.top();
    if top < bounds.top() {
        top = bounds.top();
    }

    let mut right = r.right();
    if right > bounds.right() {
        right = bounds.right();
    }

    let mut bottom = r.bottom();
    if bottom > bounds.bottom() {
        bottom = bounds.bottom();
    }

    tiny_skia::IntRect::from_ltrb(left, top, right, bottom)
}

/// Converts `viewBox` to `Transform` with an optional clip rectangle.
///
/// Unlike `view_box_to_transform`, returns an optional clip rectangle
/// that should be applied before rendering the image.
pub fn view_box_to_transform_with_clip(
    view_box: &usvg::ViewBox,
    img_size: tiny_skia::IntSize,
) -> (usvg::Transform, Option<tiny_skia::NonZeroRect>) {
    let r = view_box.rect;

    let new_size = fit_view_box(img_size.to_size(), view_box);

    let (tx, ty, clip) = if view_box.aspect.slice {
        let (dx, dy) = usvg::utils::aligned_pos(
            view_box.aspect.align,
            0.0,
            0.0,
            new_size.width() - r.width(),
            new_size.height() - r.height(),
        );

        (r.x() - dx, r.y() - dy, Some(r))
    } else {
        let (dx, dy) = usvg::utils::aligned_pos(
            view_box.aspect.align,
            r.x(),
            r.y(),
            r.width() - new_size.width(),
            r.height() - new_size.height(),
        );

        (dx, dy, None)
    };

    let sx = new_size.width() / img_size.width() as f32;
    let sy = new_size.height() / img_size.height() as f32;
    let ts = usvg::Transform::from_row(sx, 0.0, 0.0, sy, tx, ty);

    (ts, clip)
}

/// Fits size into a viewbox.
pub fn fit_view_box(size: usvg::Size, vb: &usvg::ViewBox) -> usvg::Size {
    let s = vb.rect.size();

    if vb.aspect.align == usvg::Align::None {
        s
    } else if vb.aspect.slice {
        size.expand_to(s)
    } else {
        size.scale_to(s)
    }
}
