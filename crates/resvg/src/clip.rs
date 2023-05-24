// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::rc::Rc;

use crate::render::Context;
use crate::tree::{Node, OptionLog};

pub struct ClipPath {
    pub transform: tiny_skia::Transform,
    pub clip_path: Option<Box<Self>>,
    pub children: Vec<Node>,
}

pub fn convert(
    upath: Option<Rc<usvg::ClipPath>>,
    object_bbox: tiny_skia::Rect,
) -> Option<ClipPath> {
    let upath = upath?;

    let mut transform = upath.transform;

    if upath.units == usvg::Units::ObjectBoundingBox {
        let object_bbox = object_bbox
            .to_non_zero_rect()
            .log_none(|| log::warn!("Clipping of zero-sized shapes is not allowed."))?;

        let ts = usvg::Transform::from_bbox(object_bbox);
        transform = transform.pre_concat(ts);
    }

    let (children, _) = crate::tree::convert_node(upath.root.clone());
    Some(ClipPath {
        transform,
        clip_path: convert(upath.clip_path.clone(), object_bbox).map(Box::new),
        children,
    })
}

pub fn apply(clip: &ClipPath, transform: tiny_skia::Transform, pixmap: &mut tiny_skia::Pixmap) {
    let mut clip_pixmap = tiny_skia::Pixmap::new(pixmap.width(), pixmap.height()).unwrap();
    clip_pixmap.fill(tiny_skia::Color::BLACK);

    draw_children(
        &clip.children,
        tiny_skia::BlendMode::Clear,
        transform.pre_concat(clip.transform),
        &mut clip_pixmap.as_mut(),
    );

    if let Some(ref clip) = clip.clip_path {
        apply(clip, transform, pixmap);
    }

    let mut mask = tiny_skia::Mask::from_pixmap(clip_pixmap.as_ref(), tiny_skia::MaskType::Alpha);
    mask.invert();
    pixmap.apply_mask(&mask);
}

fn draw_children(
    children: &[Node],
    mode: tiny_skia::BlendMode,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) {
    for child in children {
        match child {
            Node::FillPath(ref path) => {
                // We could use any values here. They will not be used anyway.
                let ctx = Context {
                    max_bbox: tiny_skia::IntRect::from_xywh(0, 0, 1, 1).unwrap(),
                };

                crate::path::render_fill_path(path, mode, &ctx, transform, pixmap);
            }
            Node::Group(ref group) => {
                let transform = transform.pre_concat(group.transform);

                if let Some(ref clip) = group.clip_path {
                    // If a `clipPath` child also has a `clip-path`
                    // then we should render this child on a new canvas,
                    // clip it, and only then draw it to the `clipPath`.
                    clip_group(&group.children, clip, transform, pixmap);
                } else {
                    draw_children(&group.children, mode, transform, pixmap);
                }
            }
            _ => {}
        }
    }
}

fn clip_group(
    children: &[Node],
    clip: &ClipPath,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) -> Option<()> {
    let mut clip_pixmap = tiny_skia::Pixmap::new(pixmap.width(), pixmap.height()).unwrap();

    draw_children(
        children,
        tiny_skia::BlendMode::SourceOver,
        transform,
        &mut clip_pixmap.as_mut(),
    );
    apply(clip, transform, &mut clip_pixmap);

    let mut paint = tiny_skia::PixmapPaint::default();
    paint.blend_mode = tiny_skia::BlendMode::Xor;
    pixmap.draw_pixmap(
        0,
        0,
        clip_pixmap.as_ref(),
        &paint,
        tiny_skia::Transform::identity(),
        None,
    );

    Some(())
}
