// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use qt;
use usvg;

// self
use super::prelude::*;
use backend_utils::mask;


pub fn apply(
    node: &usvg::Node,
    mask: &usvg::Mask,
    opt: &Options,
    bbox: Rect,
    layers: &mut QtLayers,
    sub_p: &mut qt::Painter,
) {
    let mask_img = try_opt!(layers.get(), ());
    let mut mask_img = mask_img.borrow_mut();

    {
        let mut mask_p = qt::Painter::new(&mut mask_img);
        mask_p.set_transform(&sub_p.get_transform());

        let r = if mask.units == usvg::Units::ObjectBoundingBox {
            mask.rect.transform(usvg::Transform::from_bbox(bbox))
        } else {
            mask.rect
        };

        mask_p.set_clip_rect(r.x, r.y, r.width, r.height);

        if mask.content_units == usvg::Units::ObjectBoundingBox {
            mask_p.apply_transform(&qt::Transform::from_bbox(bbox));
        }

        super::render_group(node, opt, layers, &mut mask_p);
    }

    mask::image_to_mask(&mut mask_img.data_mut(), layers.image_size());

    if let Some(ref id) = mask.mask {
        if let Some(ref mask_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::Mask(ref mask) = *mask_node.borrow() {
                apply(mask_node, mask, opt, bbox, layers, sub_p);
            }
        }
    }

    sub_p.set_transform(&qt::Transform::default());
    sub_p.set_composition_mode(qt::CompositionMode::DestinationIn);
    sub_p.draw_image(0.0, 0.0, &mask_img);
}
