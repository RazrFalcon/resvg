// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::render::prelude::*;

pub fn mask(
    node: &usvg::Node,
    mask: &usvg::Mask,
    bbox: Rect,
    layers: &mut Layers,
    canvas: &mut skia::Canvas,
) {
    let mask_surface = try_opt!(layers.get());
    let mut mask_surface = mask_surface.borrow_mut();

    {
        mask_surface.set_matrix(&canvas.get_matrix());

        let r = if mask.units == usvg::Units::ObjectBoundingBox {
            mask.rect.bbox_transform(bbox)
        } else {
            mask.rect
        };

        mask_surface.save();
        mask_surface.set_clip_rect(r.x(), r.y(), r.width(), r.height());

        if mask.content_units == usvg::Units::ObjectBoundingBox {
            mask_surface.concat(&usvg::Transform::from_bbox(bbox).to_native());
        }

        crate::render::render_group(node, &mut RenderState::Ok, layers, &mut mask_surface);

        mask_surface.restore();
    }

    {
        use rgb::FromSlice;
        use std::mem::swap;

        let mut data = mask_surface.data_mut();

        // RGBA -> BGRA.
        if !skia::Surface::is_bgra() {
            data.as_bgra_mut().iter_mut().for_each(|p| swap(&mut p.r, &mut p.b));
        }

        image_to_mask(data.as_bgra_mut(), layers.image_size());

        // BGRA -> RGBA.
        if !skia::Surface::is_bgra() {
            data.as_bgra_mut().iter_mut().for_each(|p| swap(&mut p.r, &mut p.b));
        }
    }

    if let Some(ref id) = mask.mask {
        if let Some(ref mask_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::Mask(ref mask) = *mask_node.borrow() {
                self::mask(mask_node, mask, bbox, layers, canvas);
            }
        }
    }

    canvas.reset_matrix();
    canvas.draw_surface(
        &mask_surface, 0.0, 0.0, 255, skia::BlendMode::DestinationIn, skia::FilterQuality::Low,
    );
}

/// Converts an image into an alpha mask.
fn image_to_mask(
    data: &mut [rgb::alt::BGRA8],
    img_size: ScreenSize,
) {
    let width = img_size.width();
    let height = img_size.height();

    let coeff_r = 0.2125 / 255.0;
    let coeff_g = 0.7154 / 255.0;
    let coeff_b = 0.0721 / 255.0;

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            let ref mut pixel = data[idx];

            let r = pixel.r as f64;
            let g = pixel.g as f64;
            let b = pixel.b as f64;

            let luma = r * coeff_r + g * coeff_g + b * coeff_b;

            pixel.r = 0;
            pixel.g = 0;
            pixel.b = 0;
            pixel.a = usvg::utils::f64_bound(0.0, luma * 255.0, 255.0) as u8;
        }
    }
}
