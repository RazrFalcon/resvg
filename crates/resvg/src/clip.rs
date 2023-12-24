// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::render::Context;

pub fn apply(
    clip: &usvg::ClipPath,
    object_bbox: tiny_skia::Rect,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::Pixmap,
) {
    let mut clip_transform = clip.transform;
    if clip.units == usvg::Units::ObjectBoundingBox {
        let object_bbox = match object_bbox.to_non_zero_rect() {
            Some(v) => v,
            None => {
                log::warn!("Clipping of zero-sized shapes is not allowed.");
                return;
            }
        };

        let ts = usvg::Transform::from_bbox(object_bbox);
        clip_transform = clip_transform.pre_concat(ts);
    }

    let mut clip_pixmap = tiny_skia::Pixmap::new(pixmap.width(), pixmap.height()).unwrap();
    clip_pixmap.fill(tiny_skia::Color::BLACK);

    draw_children(
        &clip.root,
        tiny_skia::BlendMode::Clear,
        object_bbox,
        transform.pre_concat(clip_transform),
        &mut clip_pixmap.as_mut(),
    );

    if let Some(ref clip) = clip.clip_path {
        apply(&clip.borrow(), object_bbox, transform, pixmap);
    }

    let mut mask = tiny_skia::Mask::from_pixmap(clip_pixmap.as_ref(), tiny_skia::MaskType::Alpha);
    mask.invert();
    pixmap.apply_mask(&mask);
}

fn draw_children(
    parent: &usvg::Group,
    mode: tiny_skia::BlendMode,
    object_bbox: tiny_skia::Rect,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) {
    for child in &parent.children {
        match child {
            usvg::Node::Path(ref path) => {
                if path.visibility != usvg::Visibility::Visible {
                    continue;
                }

                // We could use any values here. They will not be used anyway.
                let ctx = Context {
                    max_bbox: tiny_skia::IntRect::from_xywh(0, 0, 1, 1).unwrap(),
                };

                crate::path::fill_path(path, mode, &ctx, object_bbox, transform, pixmap);
            }
            usvg::Node::Text(ref text) => {
                if let (Some(flattened), Some(bbox)) = (&text.flattened, text.bounding_box) {
                    draw_children(flattened, mode, bbox.to_rect(), transform, pixmap);
                }
            }
            usvg::Node::Group(ref group) => {
                let transform = transform.pre_concat(group.transform);

                if let Some(ref clip) = group.clip_path {
                    // If a `clipPath` child also has a `clip-path`
                    // then we should render this child on a new canvas,
                    // clip it, and only then draw it to the `clipPath`.
                    clip_group(group, &clip.borrow(), object_bbox, transform, pixmap);
                } else {
                    draw_children(group, mode, object_bbox, transform, pixmap);
                }
            }
            _ => {}
        }
    }
}

fn clip_group(
    children: &usvg::Group,
    clip: &usvg::ClipPath,
    object_bbox: tiny_skia::Rect,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) -> Option<()> {
    let mut clip_pixmap = tiny_skia::Pixmap::new(pixmap.width(), pixmap.height()).unwrap();

    draw_children(
        children,
        tiny_skia::BlendMode::SourceOver,
        object_bbox,
        transform,
        &mut clip_pixmap.as_mut(),
    );
    apply(clip, object_bbox, transform, &mut clip_pixmap);

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
