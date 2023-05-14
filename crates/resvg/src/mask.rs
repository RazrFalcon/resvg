// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::rc::Rc;

use crate::render::{Context, TinySkiaPixmapMutExt};
use crate::tree::{ConvTransform, Node, OptionLog, UsvgRectExt};

pub struct Mask {
    pub mask_all: bool,
    pub rect: tiny_skia::Rect,
    pub kind: usvg::MaskType,
    pub mask: Option<Box<Self>>,
    pub children: Vec<Node>,
}

pub fn convert(
    umask: Option<Rc<usvg::Mask>>,
    object_bbox: usvg::PathBbox,
    mut transform: tiny_skia::Transform,
) -> Option<Mask> {
    let umask = umask?;

    if umask.content_units == usvg::Units::ObjectBoundingBox {
        let object_bbox = object_bbox
            .to_rect()
            .log_none(|| log::warn!("Masking of zero-sized shapes is not allowed."))?;

        let ts = usvg::Transform::from_bbox(object_bbox);
        transform = transform.pre_concat(ts.to_native());
    }

    let mut mask_all = false;
    if umask.units == usvg::Units::ObjectBoundingBox && object_bbox.to_rect().is_none() {
        // `objectBoundingBox` units and zero-sized bbox? Clear the canvas and return.
        // Technically a UB, but this is what Chrome and Firefox do.
        mask_all = true;
    }

    let rect = if umask.units == usvg::Units::ObjectBoundingBox {
        if let Some(bbox) = object_bbox.to_rect() {
            umask.rect.bbox_transform(bbox)
        } else {
            // The actual values does not matter. Will not be used anyway.
            usvg::Rect::new(0.0, 0.0, 1.0, 1.0).unwrap()
        }
    } else {
        umask.rect
    }
    .to_skia_rect()?;

    let (children, _) = crate::tree::convert_node(umask.root.clone(), transform);
    Some(Mask {
        mask_all,
        rect,
        kind: umask.kind,
        mask: convert(umask.mask.clone(), object_bbox, transform).map(Box::new),
        children,
    })
}

pub fn apply(
    mask: &Mask,
    ctx: &Context,
    parent_offset: (i32, i32),
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::Pixmap,
) {
    if mask.mask_all {
        pixmap.fill(tiny_skia::Color::TRANSPARENT);
        return;
    }

    let mut mask_pixmap = tiny_skia::Pixmap::new(pixmap.width(), pixmap.height()).unwrap();

    {
        // Mask has to be clipped to the mask.rect
        let alpha_mask = mask_pixmap.as_mut().create_rect_mask(transform, mask.rect);

        crate::render::render_nodes(
            &mask.children,
            ctx,
            parent_offset,
            transform,
            &mut mask_pixmap.as_mut(),
        );

        if let Some(alpha_mask) = alpha_mask {
            mask_pixmap.apply_mask(&alpha_mask);
        }
    }

    if let Some(ref mask) = mask.mask {
        self::apply(mask, ctx, parent_offset, transform, pixmap);
    }

    let mask_type = match mask.kind {
        usvg::MaskType::Luminance => tiny_skia::MaskType::Luminance,
        usvg::MaskType::Alpha => tiny_skia::MaskType::Alpha,
    };

    let mask = tiny_skia::Mask::from_pixmap(mask_pixmap.as_ref(), mask_type);
    pixmap.apply_mask(&mask);
}
