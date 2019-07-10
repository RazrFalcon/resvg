// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::skia;
use usvg::try_opt;

use crate::{prelude::*, backend_utils::*};
use super::{path, SkiaLayers};


pub fn clip(
    node: &usvg::Node,
    cp: &usvg::ClipPath,
    opt: &Options,
    bbox: Rect,
    layers: &mut SkiaLayers,
    canvas: &mut skia::Canvas,
) {
    let clip_surface = try_opt!(layers.get());
    let mut clip_surface = clip_surface.borrow_mut();
    clip_surface.get_canvas().clear_rgba(0, 0, 0, 255);

    let mut clip_canvas = clip_surface.get_canvas();
    clip_canvas.set_matrix(&canvas.get_total_matrix());
    clip_canvas.concat(&cp.transform.to_native());

    if cp.units == usvg::Units::ObjectBoundingBox {
        clip_canvas.concat(&usvg::Transform::from_bbox(bbox).to_native());
    }

    let ts = clip_canvas.get_total_matrix();
    for node in node.children() {
        clip_canvas.concat(&node.transform().to_native());

        match *node.borrow() {
            usvg::NodeKind::Path(ref path_node) => {
                path::draw(&node.tree(), path_node, opt, &mut clip_canvas, skia::BlendMode::Clear);
            }
            usvg::NodeKind::Group(ref g) => {
                clip_group(&node, g, opt, bbox, layers, &mut clip_canvas);
            }
            _ => {}
        }

        clip_canvas.set_matrix(&ts);
    }

    if let Some(ref id) = cp.clip_path {
        if let Some(ref clip_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::ClipPath(ref cp) = *clip_node.borrow() {
                clip(clip_node, cp, opt, bbox, layers, canvas);
            }
        }
    }

    canvas.set_matrix(&skia::Matrix::default());
    canvas.draw_surface(&clip_surface, 0.0, 0.0, 255, skia::BlendMode::DestinationOut);
}

fn clip_group(
    node: &usvg::Node,
    g: &usvg::Group,
    opt: &Options,
    bbox: Rect,
    layers: &mut SkiaLayers,
    canvas: &mut skia::Canvas,
) {
    if let Some(ref id) = g.clip_path {
        if let Some(ref clip_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::ClipPath(ref cp) = *clip_node.borrow() {
                // If a `clipPath` child also has a `clip-path`
                // then we should render this child on a new canvas,
                // clip it, and only then draw it to the `clipPath`.

                let clip_surface = try_opt!(layers.get());
                let mut clip_surface = clip_surface.borrow_mut();

                let mut clip_canvas = clip_surface.get_canvas();
                clip_canvas.set_matrix(&canvas.get_total_matrix());
                draw_group_child(&node, opt, &mut clip_canvas);

                clip(clip_node, cp, opt, bbox, layers, &mut clip_canvas);
                //clip_p.end();

                canvas.set_matrix(&skia::Matrix::default());
                //p.set_composition_mode(qt::CompositionMode::Xor);
                canvas.draw_surface(&clip_surface, 0.0, 0.0, 255, skia::BlendMode::Xor);
            }
        }
    }
}

fn draw_group_child(
    node: &usvg::Node,
    opt: &Options,
    canvas: &mut skia::Canvas,
) {
    if let Some(child) = node.first_child() {
        canvas.concat(&child.transform().to_native());

        match *child.borrow() {
            usvg::NodeKind::Path(ref path_node) => {
                path::draw(&child.tree(), path_node, opt, canvas, skia::BlendMode::SourceOver);
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
    layers: &mut SkiaLayers,
    sub_canvas: &mut skia::Canvas,
) {
    let mask_surface = try_opt!(layers.get());
    let mut mask_surface = mask_surface.borrow_mut();

    {
        let mut mask_canvas = mask_surface.get_canvas();
        mask_canvas.set_matrix(&sub_canvas.get_total_matrix());

        let r = if mask.units == usvg::Units::ObjectBoundingBox {
            mask.rect.bbox_transform(bbox)
        } else {
            mask.rect
        };

        mask_canvas.clip_rect(r.x(), r.y(), r.width(), r.height());

        if mask.content_units == usvg::Units::ObjectBoundingBox {
            mask_canvas.concat(&usvg::Transform::from_bbox(bbox).to_native());
        }

        super::render_group(node, opt, layers, &mut mask_canvas);
    }

    image_to_mask(&mut mask_surface.data_mut(), layers.image_size());

    if let Some(ref id) = mask.mask {
        if let Some(ref mask_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::Mask(ref mask) = *mask_node.borrow() {
                self::mask(mask_node, mask, opt, bbox, layers, sub_canvas);
            }
        }
    }

    sub_canvas.set_matrix(&skia::Matrix::default());
    sub_canvas.draw_surface(&mask_surface, 0.0, 0.0, 255, skia::BlendMode::DestinationIn);
}
