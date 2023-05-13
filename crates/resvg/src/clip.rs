// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::rc::Rc;

use crate::render::{Canvas, Context};
use crate::tree::{ConvTransform, Node, OptionLog};

pub struct ClipPath {
    pub clip_path: Option<Box<Self>>,
    pub children: Vec<Node>,
}

pub fn convert(
    upath: Option<Rc<usvg::ClipPath>>,
    object_bbox: usvg::PathBbox,
    mut transform: tiny_skia::Transform,
) -> Option<ClipPath> {
    let upath = upath?;

    transform = transform.pre_concat(upath.transform.to_native());

    if upath.units == usvg::Units::ObjectBoundingBox {
        let object_bbox = object_bbox
            .to_rect()
            .log_none(|| log::warn!("Clipping of zero-sized shapes is not allowed."))?;

        let ts = usvg::Transform::from_bbox(object_bbox);
        transform = transform.pre_concat(ts.to_native());
    }

    let (children, _) = crate::tree::convert_node(upath.root.clone(), transform);
    Some(ClipPath {
        clip_path: convert(upath.clip_path.clone(), object_bbox, transform).map(Box::new),
        children,
    })
}

pub fn apply(clip: &ClipPath, transform: tiny_skia::Transform, pixmap: &mut tiny_skia::Pixmap) {
    let mut canvas = Canvas::from(pixmap.as_mut());
    canvas.transform = transform;
    apply_inner(clip, &mut canvas);
}

fn apply_inner(clip: &ClipPath, canvas: &mut Canvas) {
    let mut clip_pixmap = canvas.new_pixmap();
    clip_pixmap.fill(tiny_skia::Color::BLACK);

    let mut clip_canvas = Canvas::from(clip_pixmap.as_mut());
    clip_canvas.transform = canvas.transform;

    draw_children(
        &clip.children,
        tiny_skia::BlendMode::Clear,
        &mut clip_canvas,
    );

    if let Some(ref clip) = clip.clip_path {
        apply_inner(clip, canvas);
    }

    let mut mask = tiny_skia::Mask::from_pixmap(clip_pixmap.as_ref(), tiny_skia::MaskType::Alpha);
    mask.invert();
    canvas.pixmap.apply_mask(&mask);
}

fn draw_children(children: &[Node], mode: tiny_skia::BlendMode, canvas: &mut Canvas) {
    for child in children {
        match child {
            Node::FillPath(ref path) => {
                // We could use any values here. They will not be used anyway.
                let ctx = Context {
                    root_transform: usvg::Transform::default(),
                    target_size: usvg::ScreenSize::new(1, 1).unwrap(),
                    max_filter_region: usvg::ScreenRect::new(0, 0, 1, 1).unwrap(),
                };

                crate::path::render_fill_path(path, mode, &ctx, canvas);
            }
            Node::Group(ref group) => {
                if let Some(ref clip) = group.clip_path {
                    // If a `clipPath` child also has a `clip-path`
                    // then we should render this child on a new canvas,
                    // clip it, and only then draw it to the `clipPath`.
                    clip_group(&group.children, clip, canvas);
                } else {
                    draw_children(&group.children, mode, canvas);
                }
            }
            _ => {}
        }
    }
}

fn clip_group(children: &[Node], clip: &ClipPath, canvas: &mut Canvas) -> Option<()> {
    let mut clip_pixmap = canvas.new_pixmap();
    let mut clip_canvas = Canvas::from(clip_pixmap.as_mut());
    clip_canvas.transform = canvas.transform;

    draw_children(children, tiny_skia::BlendMode::SourceOver, &mut clip_canvas);
    apply_inner(clip, &mut clip_canvas);

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
