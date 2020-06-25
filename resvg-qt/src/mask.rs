// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::render::prelude::*;

pub fn mask(
    node: &usvg::Node,
    mask: &usvg::Mask,
    bbox: Rect,
    layers: &mut Layers,
    p: &mut qt::Painter,
) {
    let mask_img = try_opt!(layers.get());
    let mut mask_img = mask_img.borrow_mut();

    {
        let mut mask_p = qt::Painter::new(&mut mask_img);
        mask_p.set_transform(&p.get_transform());

        let r = if mask.units == usvg::Units::ObjectBoundingBox {
            mask.rect.bbox_transform(bbox)
        } else {
            mask.rect
        };

        mask_p.set_clip_rect(r.x(), r.y(), r.width(), r.height());

        if mask.content_units == usvg::Units::ObjectBoundingBox {
            mask_p.apply_transform(&usvg::Transform::from_bbox(bbox).to_native());
        }

        crate::render::render_group(node, &mut RenderState::Ok, layers, &mut mask_p);
    }

    use rgb::FromSlice;
    image_to_mask(mask_img.data_mut().as_bgra_mut(), layers.image_size());

    if let Some(ref id) = mask.mask {
        if let Some(ref mask_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::Mask(ref mask) = *mask_node.borrow() {
                self::mask(mask_node, mask, bbox, layers, p);
            }
        }
    }

    p.set_transform(&qt::Transform::default());
    p.set_composition_mode(qt::CompositionMode::DestinationIn);
    p.draw_image(0.0, 0.0, &mask_img);
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
