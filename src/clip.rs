// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::render::prelude::*;

pub fn clip(
    node: &usvg::Node,
    cp: &usvg::ClipPath,
    bbox: Rect,
    layers: &mut Layers,
    canvas: &mut tiny_skia::Canvas,
) {
    let clip_canvas = try_opt!(layers.get());
    let mut clip_canvas = clip_canvas.borrow_mut();

    clip_canvas.pixmap.fill(tiny_skia::Color::BLACK);

    clip_canvas.set_transform(canvas.get_transform());
    clip_canvas.apply_transform(&cp.transform.to_native());

    if cp.units == usvg::Units::ObjectBoundingBox {
        clip_canvas.apply_transform(&usvg::Transform::from_bbox(bbox).to_native());
    }

    let ts = clip_canvas.get_transform();
    for node in node.children() {
        clip_canvas.apply_transform(&node.transform().to_native());

        match *node.borrow() {
            usvg::NodeKind::Path(ref path_node) => {
                crate::path::draw(
                    &node.tree(),
                    path_node,
                    tiny_skia::BlendMode::Clear,
                    &mut clip_canvas,
                );
            }
            usvg::NodeKind::Group(ref g) => {
                clip_group(&node, g, bbox, layers, &mut clip_canvas);
            }
            _ => {}
        }

        clip_canvas.set_transform(ts);
    }

    if let Some(ref id) = cp.clip_path {
        if let Some(ref clip_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::ClipPath(ref cp) = *clip_node.borrow() {
                clip(clip_node, cp, bbox, layers, canvas);
            }
        }
    }

    let mut paint = tiny_skia::PixmapPaint::default();
    paint.blend_mode = tiny_skia::BlendMode::DestinationOut;
    canvas.reset_transform();
    canvas.draw_pixmap(0, 0, &clip_canvas.pixmap, &paint);
}

fn clip_group(
    node: &usvg::Node,
    g: &usvg::Group,
    bbox: Rect,
    layers: &mut Layers,
    canvas: &mut tiny_skia::Canvas,
) {
    if let Some(ref id) = g.clip_path {
        if let Some(ref clip_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::ClipPath(ref cp) = *clip_node.borrow() {
                // If a `clipPath` child also has a `clip-path`
                // then we should render this child on a new canvas,
                // clip it, and only then draw it to the `clipPath`.

                let clip_canvas = try_opt!(layers.get());
                let mut clip_canvas = clip_canvas.borrow_mut();

                clip_canvas.set_transform(canvas.get_transform());

                draw_group_child(&node, &mut clip_canvas);
                clip(clip_node, cp, bbox, layers, &mut clip_canvas);

                let mut paint = tiny_skia::PixmapPaint::default();
                paint.blend_mode = tiny_skia::BlendMode::Xor;
                canvas.reset_transform();
                canvas.draw_pixmap(0, 0, &clip_canvas.pixmap, &paint);
            }
        }
    }
}

fn draw_group_child(node: &usvg::Node, canvas: &mut tiny_skia::Canvas) {
    if let Some(child) = node.first_child() {
        canvas.apply_transform(&child.transform().to_native());

        match *child.borrow() {
            usvg::NodeKind::Path(ref path_node) => {
                crate::path::draw(&child.tree(), path_node, tiny_skia::BlendMode::SourceOver, canvas);
            }
            _ => {}
        }
    }
}
