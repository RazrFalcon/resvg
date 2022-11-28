// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use usvg::NodeExt;

use crate::{render::Canvas, ConvTransform, OptionLog};

pub fn clip(
    tree: &usvg::Tree,
    cp: &usvg::ClipPath,
    bbox: usvg::PathBbox,
    canvas: &mut Canvas,
) -> Option<()> {
    let mut clip_pixmap = tiny_skia::Pixmap::new(canvas.pixmap.width(), canvas.pixmap.height())?;
    clip_pixmap.fill(tiny_skia::Color::BLACK);

    let mut clip_canvas = Canvas::from(clip_pixmap.as_mut());
    clip_canvas.transform = canvas.transform;
    clip_canvas.apply_transform(cp.transform.to_native());

    if cp.units == usvg::Units::ObjectBoundingBox {
        let bbox = bbox
            .to_rect()
            .log_none(|| log::warn!("Clipping of zero-sized shapes is not allowed."))?;

        clip_canvas.apply_transform(usvg::Transform::from_bbox(bbox).to_native());
    }

    let ts = clip_canvas.transform;
    for node in cp.root.children() {
        clip_canvas.apply_transform(node.transform().to_native());

        match *node.borrow() {
            usvg::NodeKind::Path(ref path_node) => {
                crate::path::draw(
                    tree,
                    path_node,
                    tiny_skia::BlendMode::Clear,
                    &mut clip_canvas,
                );
            }
            usvg::NodeKind::Group(ref g) => {
                clip_group(tree, &node, g, bbox, &mut clip_canvas);
            }
            _ => {}
        }

        clip_canvas.transform = ts;
    }

    if let Some(ref cp) = cp.clip_path {
        clip(tree, cp, bbox, canvas);
    }

    let mut paint = tiny_skia::PixmapPaint::default();
    paint.blend_mode = tiny_skia::BlendMode::DestinationOut;
    canvas.pixmap.draw_pixmap(
        0,
        0,
        clip_pixmap.as_ref(),
        &paint,
        tiny_skia::Transform::identity(),
        None,
    );

    Some(())
}

fn clip_group(
    tree: &usvg::Tree,
    node: &usvg::Node,
    g: &usvg::Group,
    bbox: usvg::PathBbox,
    canvas: &mut Canvas,
) -> Option<()> {
    if let Some(ref cp) = g.clip_path {
        // If a `clipPath` child also has a `clip-path`
        // then we should render this child on a new canvas,
        // clip it, and only then draw it to the `clipPath`.

        let mut clip_pixmap =
            tiny_skia::Pixmap::new(canvas.pixmap.width(), canvas.pixmap.height())?;
        let mut clip_canvas = Canvas::from(clip_pixmap.as_mut());
        clip_canvas.transform = canvas.transform;

        draw_group_child(tree, node, &mut clip_canvas);
        clip(tree, cp, bbox, &mut clip_canvas);

        let mut paint = tiny_skia::PixmapPaint::default();
        paint.blend_mode = tiny_skia::BlendMode::Xor;
        canvas.pixmap.draw_pixmap(
            0,
            0,
            clip_pixmap.as_ref(),
            &paint,
            tiny_skia::Transform::identity(),
            None,
        );
    }

    Some(())
}

fn draw_group_child(tree: &usvg::Tree, node: &usvg::Node, canvas: &mut Canvas) {
    if let Some(child) = node.first_child() {
        canvas.apply_transform(child.transform().to_native());

        if let usvg::NodeKind::Path(ref path_node) = *child.borrow() {
            crate::path::draw(tree, path_node, tiny_skia::BlendMode::SourceOver, canvas);
        }
    }
}
