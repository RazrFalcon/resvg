// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{prelude::*, ConvTransform, RenderState};
use super::{path, CairoLayers, ReCairoContextExt};


pub fn clip(
    node: &usvg::Node,
    cp: &usvg::ClipPath,
    opt: &Options,
    bbox: Rect,
    layers: &mut CairoLayers,
    cr: &cairo::Context,
) {
    let clip_surface = try_opt!(layers.get());
    let clip_surface = clip_surface.borrow_mut();

    let clip_cr = cairo::Context::new(&*clip_surface);
    clip_cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
    clip_cr.paint();
    clip_cr.set_matrix(cr.get_matrix());
    clip_cr.transform(cp.transform.to_native());

    if cp.units == usvg::Units::ObjectBoundingBox {
        clip_cr.transform(usvg::Transform::from_bbox(bbox).to_native());
    }

    clip_cr.set_operator(cairo::Operator::Clear);

    let matrix = clip_cr.get_matrix();
    for node in node.children() {
        clip_cr.transform(node.transform().to_native());

        match *node.borrow() {
            usvg::NodeKind::Path(ref p) => {
                path::draw(&node.tree(), p, opt, &clip_cr);
            }
            usvg::NodeKind::Group(ref g) => {
                clip_group(&node, g, opt, bbox, layers, &clip_cr);
            }
            _ => {}
        }

        clip_cr.set_matrix(matrix);
    }

    if let Some(ref id) = cp.clip_path {
        if let Some(ref clip_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::ClipPath(ref cp) = *clip_node.borrow() {
                clip(clip_node, cp, opt, bbox, layers, cr);
            }
        }
    }

    cr.set_matrix(cairo::Matrix::identity());
    cr.set_source_surface(&*clip_surface, 0.0, 0.0);
    cr.set_operator(cairo::Operator::DestOut);
    cr.paint();

    // Reset operator.
    cr.set_operator(cairo::Operator::Over);

    // Reset source to unborrow the `clip_surface` from the `Context`.
    cr.reset_source_rgba();
}

fn clip_group(
    node: &usvg::Node,
    g: &usvg::Group,
    opt: &Options,
    bbox: Rect,
    layers: &mut CairoLayers,
    cr: &cairo::Context,
) {
    if let Some(ref id) = g.clip_path {
        if let Some(ref clip_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::ClipPath(ref cp) = *clip_node.borrow() {
                // If a `clipPath` child also has a `clip-path`
                // then we should render this child on a new canvas,
                // clip it, and only then draw it to the `clipPath`.

                let clip_surface = try_opt!(layers.get());
                let clip_surface = clip_surface.borrow_mut();

                let clip_cr = cairo::Context::new(&*clip_surface);
                clip_cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
                clip_cr.paint();
                clip_cr.set_matrix(cr.get_matrix());

                draw_group_child(&node, opt, &clip_cr);

                clip(clip_node, cp, opt, bbox, layers, &clip_cr);

                cr.set_matrix(cairo::Matrix::identity());
                cr.set_operator(cairo::Operator::Xor);
                cr.set_source_surface(&*clip_surface, 0.0, 0.0);
                cr.set_operator(cairo::Operator::DestOut);
                cr.paint();
            }
        }
    }
}

fn draw_group_child(
    node: &usvg::Node,
    opt: &Options,
    cr: &cairo::Context,
) {
    if let Some(child) = node.first_child() {
        cr.transform(child.transform().to_native());

        match *child.borrow() {
            usvg::NodeKind::Path(ref path_node) => {
                path::draw(&child.tree(), path_node, opt, cr);
            }
            _ => {}
        }
    }
}

pub fn mask(
    node: &usvg::Node,
    mask: &usvg::Mask,
    opt: &Options,
    bbox: Rect,
    layers: &mut CairoLayers,
    cr: &cairo::Context,
) {
    let mask_surface = try_opt!(layers.get());
    let mut mask_surface = mask_surface.borrow_mut();

    {
        let mask_cr = cairo::Context::new(&*mask_surface);
        mask_cr.set_matrix(cr.get_matrix());

        let r = if mask.units == usvg::Units::ObjectBoundingBox {
            mask.rect.bbox_transform(bbox)
        } else {
            mask.rect
        };

        mask_cr.rectangle(r.x(), r.y(), r.width(), r.height());
        mask_cr.clip();

        if mask.content_units == usvg::Units::ObjectBoundingBox {
            mask_cr.transform(usvg::Transform::from_bbox(bbox).to_native());
        }

        super::render_group(node, opt, &mut RenderState::Ok, layers, &mask_cr);
    }

    {
        let mut data = try_opt_warn!(
            mask_surface.get_data().ok(),
            "Failed to borrow a surface for mask '{}'.", mask.id
        );

        use rgb::FromSlice;
        crate::image_to_mask(data.as_bgra_mut(), layers.image_size());
    }

    if let Some(ref id) = mask.mask {
        if let Some(ref mask_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::Mask(ref mask) = *mask_node.borrow() {
                self::mask(mask_node, mask, opt, bbox, layers, cr);
            }
        }
    }

    cr.set_matrix(cairo::Matrix::identity());
    cr.set_source_surface(&*mask_surface, 0.0, 0.0);
    cr.set_operator(cairo::Operator::DestIn);
    cr.paint();

    // Reset operator.
    cr.set_operator(cairo::Operator::Over);

    // Reset source to unborrow the `mask_surface` from the `Context`.
    cr.reset_source_rgba();
}
