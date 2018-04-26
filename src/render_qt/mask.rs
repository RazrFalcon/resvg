// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use qt;
use usvg::tree;

// self
use geom::*;
use traits::{
    TransformFromBBox,
};
use super::{
    QtLayers,
};
use {
    utils,
    Options,
};


pub fn apply(
    node: &tree::Node,
    mask: &tree::Mask,
    opt: &Options,
    bbox: Rect,
    layers: &mut QtLayers,
    sub_p: &qt::Painter,
    p: &qt::Painter,
) {
    let mask_img = try_opt!(layers.get(), ());
    let mut mask_img = mask_img.borrow_mut();

    {
        let mask_p = qt::Painter::new(&mask_img);
        mask_p.set_transform(&p.get_transform());

        let r = if mask.units == tree::Units::ObjectBoundingBox {
            mask.rect.transform(tree::Transform::from_bbox(bbox))
        } else {
            mask.rect
        };

        let mut p_path = qt::PainterPath::new();
        p_path.move_to(r.x(), r.y());
        p_path.line_to(r.x() + r.width(), r.y());
        p_path.line_to(r.x() + r.width(), r.y() + r.height());
        p_path.line_to(r.x(), r.y() + r.height());
        p_path.close_path();

        // TODO: use clip_rect
        mask_p.set_clip_path(&p_path);

        if mask.content_units == tree::Units::ObjectBoundingBox {
            mask_p.apply_transform(&qt::Transform::from_bbox(bbox));
        }

        super::render_group(node, opt, layers, &mask_p);
    }

    utils::image_to_mask(&mut mask_img.data_mut(), layers.image_size(), None);

    sub_p.set_transform(&qt::Transform::default());
    sub_p.set_composition_mode(qt::CompositionMode::CompositionMode_DestinationIn);
    sub_p.draw_image(0.0, 0.0, &mask_img);

    layers.release();
}
