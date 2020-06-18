// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::render::prelude::*;

pub fn mask(
    node: &usvg::Node,
    mask: &usvg::Mask,
    opt: &Options,
    bbox: Rect,
    layers: &mut Layers,
    dt: &mut raqote::DrawTarget,
) {
    let mask_dt = layers.get();
    let mut mask_dt = mask_dt.borrow_mut();

    {
        mask_dt.set_transform(dt.get_transform());

        let r = if mask.units == usvg::Units::ObjectBoundingBox {
            mask.rect.bbox_transform(bbox)
        } else {
            mask.rect
        };

        let mut pb = raqote::PathBuilder::new();
        pb.rect(r.x() as f32, r.y() as f32, r.width() as f32, r.height() as f32);
        mask_dt.push_clip(&pb.finish());

        if mask.content_units == usvg::Units::ObjectBoundingBox {
            mask_dt.transform(&usvg::Transform::from_bbox(bbox).to_native());
        }

        crate::render::render_group(node, opt, &mut RenderState::Ok, layers, &mut mask_dt);
        mask_dt.pop_clip();
    }

    use rgb::FromSlice;
    image_to_mask(mask_dt.get_data_u8_mut().as_bgra_mut(), layers.image_size());

    if let Some(ref id) = mask.mask {
        if let Some(ref mask_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::Mask(ref mask) = *mask_node.borrow() {
                self::mask(mask_node, mask, opt, bbox, layers, dt);
            }
        }
    }

    dt.blend_surface(&mask_dt,
                     raqote::IntRect::new(raqote::IntPoint::new(0, 0),
                                          raqote::IntPoint::new(mask_dt.width(), mask_dt.height())),
                     raqote::IntPoint::new(0, 0),
                     raqote::BlendMode::DstIn);
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
