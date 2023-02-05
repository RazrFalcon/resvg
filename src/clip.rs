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

    draw_children(
        tree,
        &cp.root,
        bbox,
        tiny_skia::BlendMode::Clear,
        &mut clip_canvas,
    );

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

fn draw_children(
    tree: &usvg::Tree,
    node: &usvg::Node,
    bbox: usvg::PathBbox,
    mode: tiny_skia::BlendMode,
    canvas: &mut Canvas,
) {
    let ts = canvas.transform;
    for child in node.children() {
        canvas.apply_transform(child.transform().to_native());

        match *child.borrow() {
            usvg::NodeKind::Path(ref path_node) => {
                crate::path::draw(tree, path_node, mode, canvas);
            }
            usvg::NodeKind::Group(ref g) => {
                if let Some(ref cp) = g.clip_path {
                    // If a `clipPath` child also has a `clip-path`
                    // then we should render this child on a new canvas,
                    // clip it, and only then draw it to the `clipPath`.
                    clip_group(tree, &child, cp, bbox, canvas);
                } else {
                    draw_children(tree, &child, bbox, mode, canvas);
                }
            }
            _ => {}
        }

        canvas.transform = ts;
    }
}

fn clip_group(
    tree: &usvg::Tree,
    node: &usvg::Node,
    cp: &usvg::ClipPath,
    bbox: usvg::PathBbox,
    canvas: &mut Canvas,
) -> Option<()> {
    let mut clip_pixmap = tiny_skia::Pixmap::new(canvas.pixmap.width(), canvas.pixmap.height())?;
    let mut clip_canvas = Canvas::from(clip_pixmap.as_mut());
    clip_canvas.transform = canvas.transform;

    draw_children(
        tree,
        node,
        bbox,
        tiny_skia::BlendMode::SourceOver,
        &mut clip_canvas,
    );
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

    Some(())
}
