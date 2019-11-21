// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::skia;

use crate::{prelude::*, ConvTransform, RenderState};
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

    clip_surface.fill(0, 0, 0, 255);

    clip_surface.set_matrix(&canvas.get_matrix());
    clip_surface.concat(&cp.transform.to_native());

    if cp.units == usvg::Units::ObjectBoundingBox {
        clip_surface.concat(&usvg::Transform::from_bbox(bbox).to_native());
    }

    let ts = clip_surface.get_matrix();
    for node in node.children() {
        clip_surface.concat(&node.transform().to_native());

        match *node.borrow() {
            usvg::NodeKind::Path(ref path_node) => {
                path::draw(&node.tree(), path_node, opt, skia::BlendMode::Clear, &mut clip_surface);
            }
            usvg::NodeKind::Group(ref g) => {
                clip_group(&node, g, opt, bbox, layers, &mut clip_surface);
            }
            _ => {}
        }

        clip_surface.set_matrix(&ts);
    }

    if let Some(ref id) = cp.clip_path {
        if let Some(ref clip_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::ClipPath(ref cp) = *clip_node.borrow() {
                clip(clip_node, cp, opt, bbox, layers, canvas);
            }
        }
    }

    canvas.reset_matrix();
    canvas.draw_surface(
        &clip_surface, 0.0, 0.0, 255, skia::BlendMode::DestinationOut, skia::FilterQuality::Low,
    );
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

                clip_surface.set_matrix(&canvas.get_matrix());

                draw_group_child(&node, opt, &mut clip_surface);
                clip(clip_node, cp, opt, bbox, layers, &mut clip_surface);

                canvas.reset_matrix();
                canvas.draw_surface(
                    &clip_surface, 0.0, 0.0, 255, skia::BlendMode::Xor, skia::FilterQuality::Low,
                );
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
                path::draw(&child.tree(), path_node, opt, skia::BlendMode::SourceOver, canvas);
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

        super::render_group(node, opt, &mut RenderState::Ok, layers, &mut mask_surface);

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

        crate::image_to_mask(data.as_bgra_mut(), layers.image_size());

        // BGRA -> RGBA.
        if !skia::Surface::is_bgra() {
            data.as_bgra_mut().iter_mut().for_each(|p| swap(&mut p.r, &mut p.b));
        }
    }

    if let Some(ref id) = mask.mask {
        if let Some(ref mask_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::Mask(ref mask) = *mask_node.borrow() {
                self::mask(mask_node, mask, opt, bbox, layers, canvas);
            }
        }
    }

    canvas.reset_matrix();
    canvas.draw_surface(
        &mask_surface, 0.0, 0.0, 255, skia::BlendMode::DestinationIn, skia::FilterQuality::Low,
    );
}
