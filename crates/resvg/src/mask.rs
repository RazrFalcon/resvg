// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::rc::Rc;

use crate::render::Context;
use crate::tree::{Node, OptionLog};

pub struct Mask {
    pub mask_all: bool,
    pub region: tiny_skia::Rect,
    pub content_transform: tiny_skia::Transform,
    pub kind: usvg::MaskType,
    pub mask: Option<Box<Self>>,
    pub children: Vec<Node>,
}

pub fn convert(umask: Option<Rc<usvg::Mask>>, object_bbox: tiny_skia::Rect) -> Option<Mask> {
    let umask = umask?;

    let mut content_transform = tiny_skia::Transform::default();
    if umask.content_units == usvg::Units::ObjectBoundingBox {
        let object_bbox = object_bbox
            .to_non_zero_rect()
            .log_none(|| log::warn!("Masking of zero-sized shapes is not allowed."))?;

        let ts = usvg::Transform::from_bbox(object_bbox);
        content_transform = ts;
    }

    let mut mask_all = false;
    if umask.units == usvg::Units::ObjectBoundingBox && object_bbox.to_non_zero_rect().is_none() {
        // `objectBoundingBox` units and zero-sized bbox? Clear the canvas and return.
        // Technically a UB, but this is what Chrome and Firefox do.
        mask_all = true;
    }

    let region = if umask.units == usvg::Units::ObjectBoundingBox {
        if let Some(bbox) = object_bbox.to_non_zero_rect() {
            umask.rect.bbox_transform(bbox)
        } else {
            // The actual values does not matter. Will not be used anyway.
            tiny_skia::NonZeroRect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap()
        }
    } else {
        umask.rect
    };

    let (children, _) = crate::tree::convert_node(umask.root.clone());
    Some(Mask {
        mask_all,
        region: region.to_rect(),
        content_transform,
        kind: umask.kind,
        mask: convert(umask.mask.clone(), object_bbox).map(Box::new),
        children,
    })
}

pub fn apply(
    mask: &Mask,
    ctx: &Context,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::Pixmap,
) {
    if mask.mask_all {
        pixmap.fill(tiny_skia::Color::TRANSPARENT);
        return;
    }

    let mut mask_pixmap = tiny_skia::Pixmap::new(pixmap.width(), pixmap.height()).unwrap();

    {
        // TODO: only when needed
        // Mask has to be clipped by mask.region
        let mut alpha_mask = tiny_skia::Mask::new(pixmap.width(), pixmap.height()).unwrap();
        alpha_mask.fill_path(
            &tiny_skia::PathBuilder::from_rect(mask.region),
            tiny_skia::FillRule::Winding,
            true,
            transform,
        );

        let content_transform = transform.pre_concat(mask.content_transform);
        crate::render::render_nodes(
            &mask.children,
            ctx,
            content_transform,
            &mut mask_pixmap.as_mut(),
        );

        mask_pixmap.apply_mask(&alpha_mask);
    }

    if let Some(ref mask) = mask.mask {
        self::apply(mask, ctx, transform, pixmap);
    }

    let mask_type = match mask.kind {
        usvg::MaskType::Luminance => tiny_skia::MaskType::Luminance,
        usvg::MaskType::Alpha => tiny_skia::MaskType::Alpha,
    };

    let mask = tiny_skia::Mask::from_pixmap(mask_pixmap.as_ref(), mask_type);
    pixmap.apply_mask(&mask);
}
