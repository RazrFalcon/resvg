// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{render::Canvas, ConvTransform};

pub fn mask(
    tree: &usvg::Tree,
    mask: &usvg::Mask,
    bbox: usvg::PathBbox,
    canvas: &mut Canvas,
) -> Option<()> {
    let bbox = if mask.units == usvg::Units::ObjectBoundingBox
        || mask.content_units == usvg::Units::ObjectBoundingBox
    {
        if let Some(bbox) = bbox.to_rect() {
            bbox
        } else {
            // `objectBoundingBox` units and zero-sized bbox? Clear the canvas and return.
            // Technically a UB, but this is what Chrome and Firefox do.
            canvas.pixmap.fill(tiny_skia::Color::TRANSPARENT);
            return None;
        }
    } else {
        usvg::Rect::new_bbox() // actual value doesn't matter, unreachable
    };

    let mut mask_pixmap = tiny_skia::Pixmap::new(canvas.pixmap.width(), canvas.pixmap.height())?;
    {
        let mut mask_canvas = Canvas::from(mask_pixmap.as_mut());
        mask_canvas.transform = canvas.transform;

        let r = if mask.units == usvg::Units::ObjectBoundingBox {
            mask.rect.bbox_transform(bbox)
        } else {
            mask.rect
        };

        let rr = tiny_skia::Rect::from_xywh(
            r.x() as f32,
            r.y() as f32,
            r.width() as f32,
            r.height() as f32,
        );
        if let Some(rr) = rr {
            mask_canvas.set_clip_rect(rr);
        }

        if mask.content_units == usvg::Units::ObjectBoundingBox {
            mask_canvas.apply_transform(usvg::Transform::from_bbox(bbox).to_native());
        }

        crate::render::render_group(tree, &mask.root, &mut mask_canvas);
    }

    if let Some(ref mask) = mask.mask {
        self::mask(tree, mask, bbox.to_path_bbox(), canvas);
    }

    let mask_type = match mask.kind {
        usvg::MaskType::Luminance => tiny_skia::MaskType::Luminance,
        usvg::MaskType::Alpha => tiny_skia::MaskType::Alpha,
    };

    let mask = tiny_skia::Mask::from_pixmap(mask_pixmap.as_ref(), mask_type);
    canvas.pixmap.apply_mask(&mask);

    Some(())
}
