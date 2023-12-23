// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::render::Context;

pub fn apply(
    mask: &usvg::Mask,
    ctx: &Context,
    object_bbox: tiny_skia::Rect,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::Pixmap,
) {
    let mut content_transform = tiny_skia::Transform::default();
    if mask.content_units == usvg::Units::ObjectBoundingBox {
        let object_bbox = match object_bbox.to_non_zero_rect() {
            Some(v) => v,
            None => {
                log::warn!("Masking of zero-sized shapes is not allowed.");
                return;
            }
        };

        let ts = usvg::Transform::from_bbox(object_bbox);
        content_transform = ts;
    }

    if mask.units == usvg::Units::ObjectBoundingBox && object_bbox.to_non_zero_rect().is_none() {
        // `objectBoundingBox` units and zero-sized bbox? Clear the canvas and return.
        // Technically a UB, but this is what Chrome and Firefox do.
        pixmap.fill(tiny_skia::Color::TRANSPARENT);
        return;
    }

    let region = if mask.units == usvg::Units::ObjectBoundingBox {
        if let Some(bbox) = object_bbox.to_non_zero_rect() {
            mask.rect.bbox_transform(bbox)
        } else {
            // The actual values does not matter. Will not be used anyway.
            tiny_skia::NonZeroRect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap()
        }
    } else {
        mask.rect
    };

    let mut mask_pixmap = tiny_skia::Pixmap::new(pixmap.width(), pixmap.height()).unwrap();

    {
        // TODO: only when needed
        // Mask has to be clipped by mask.region
        let mut alpha_mask = tiny_skia::Mask::new(pixmap.width(), pixmap.height()).unwrap();
        alpha_mask.fill_path(
            &tiny_skia::PathBuilder::from_rect(region.to_rect()),
            tiny_skia::FillRule::Winding,
            true,
            transform,
        );

        let content_transform = transform.pre_concat(content_transform);
        crate::render::render_nodes(
            &mask.root,
            ctx,
            content_transform,
            None,
            &mut mask_pixmap.as_mut(),
        );

        mask_pixmap.apply_mask(&alpha_mask);
    }

    if let Some(ref mask) = mask.mask {
        self::apply(&mask.borrow(), ctx, object_bbox, transform, pixmap);
    }

    let mask_type = match mask.kind {
        usvg::MaskType::Luminance => tiny_skia::MaskType::Luminance,
        usvg::MaskType::Alpha => tiny_skia::MaskType::Alpha,
    };

    let mask = tiny_skia::Mask::from_pixmap(mask_pixmap.as_ref(), mask_type);
    pixmap.apply_mask(&mask);
}
