// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::tree::{Group, Node, OptionLog, Tree};

pub struct Context {
    pub max_bbox: tiny_skia::IntRect,
}

impl Tree {
    /// Renders the tree onto the pixmap.
    ///
    /// `transform` will be used as a root transform.
    /// Can be used to position SVG inside the `pixmap`.
    ///
    /// The produced content is in the sRGB color space.
    pub fn render(&self, transform: tiny_skia::Transform, pixmap: &mut tiny_skia::PixmapMut) {
        let target_size = tiny_skia::IntSize::from_wh(pixmap.width(), pixmap.height()).unwrap();
        let max_bbox = tiny_skia::IntRect::from_xywh(
            -(target_size.width() as i32) * 2,
            -(target_size.height() as i32) * 2,
            target_size.width() * 4,
            target_size.height() * 4,
        )
        .unwrap();

        let ts =
            usvg::utils::view_box_to_transform(self.view_box.rect, self.view_box.aspect, self.size);

        let root_transform = transform.pre_concat(ts);

        let ctx = Context { max_bbox };
        render_nodes(&self.children, &ctx, root_transform, pixmap);
    }
}

pub fn render_nodes(
    children: &[Node],
    ctx: &Context,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) {
    for node in children {
        render_node(node, ctx, transform, pixmap);
    }
}

fn render_node(
    node: &Node,
    ctx: &Context,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) {
    match node {
        Node::Group(ref group) => {
            render_group(group, ctx, transform, pixmap);
        }
        Node::FillPath(ref path) => {
            crate::path::render_fill_path(
                path,
                tiny_skia::BlendMode::SourceOver,
                ctx,
                transform,
                pixmap,
            );
        }
        Node::StrokePath(ref path) => {
            crate::path::render_stroke_path(
                path,
                tiny_skia::BlendMode::SourceOver,
                ctx,
                transform,
                pixmap,
            );
        }
        Node::Image(ref image) => {
            crate::image::render_image(image, transform, pixmap);
        }
    }
}

fn render_group(
    group: &Group,
    ctx: &Context,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) -> Option<()> {
    let transform = transform.pre_concat(group.transform);

    if group.is_transform_only() {
        render_nodes(&group.children, ctx, transform, pixmap);
        return Some(());
    }

    let bbox = group.bbox.transform(transform)?;

    let mut ibbox = if group.filters.is_empty() {
        // Convert group bbox into an integer one, expanding each side outwards by 2px
        // to make sure that anti-aliased pixels would not be clipped.
        tiny_skia::IntRect::from_xywh(
            bbox.x().floor() as i32 - 2,
            bbox.y().floor() as i32 - 2,
            bbox.width().ceil() as u32 + 4,
            bbox.height().ceil() as u32 + 4,
        )?
    } else {
        // The bounding box for groups with filters is special and should not be expanded by 2px,
        // because it's already acting as a clipping region.
        let bbox = bbox.to_non_zero_rect()?.to_int_rect();
        // Make sure our filter region is not bigger than 4x the canvas size.
        // This is required mainly to prevent huge filter regions that would tank the performance.
        // It should not affect the final result in any way.
        crate::geom::fit_to_rect(bbox, ctx.max_bbox)?
    };

    // Make sure our layer is not bigger than 4x the canvas size.
    // This is required to prevent huge layers.
    if group.filters.is_empty() {
        ibbox = crate::geom::fit_to_rect(ibbox, ctx.max_bbox)?;
    }

    let shift_ts = {
        // Original shift.
        let mut dx = bbox.x();
        let mut dy = bbox.y();

        // Account for subpixel positioned layers.
        dx -= bbox.x() - ibbox.x() as f32;
        dy -= bbox.y() - ibbox.y() as f32;

        tiny_skia::Transform::from_translate(-dx, -dy)
    };

    let transform = shift_ts.pre_concat(transform);

    let mut sub_pixmap = tiny_skia::Pixmap::new(ibbox.width(), ibbox.height())
        .log_none(|| log::warn!("Failed to allocate a group layer for: {:?}.", ibbox))?;

    render_nodes(&group.children, ctx, transform, &mut sub_pixmap.as_mut());

    if !group.filters.is_empty() {
        for filter in &group.filters {
            crate::filter::apply(filter, transform, &mut sub_pixmap);
        }
    }

    if let Some(ref clip_path) = group.clip_path {
        crate::clip::apply(clip_path, transform, &mut sub_pixmap);
    }

    if let Some(ref mask) = group.mask {
        crate::mask::apply(mask, ctx, transform, &mut sub_pixmap);
    }

    let paint = tiny_skia::PixmapPaint {
        opacity: group.opacity.get(),
        blend_mode: group.blend_mode,
        quality: tiny_skia::FilterQuality::Nearest,
    };

    pixmap.draw_pixmap(
        ibbox.x(),
        ibbox.y(),
        sub_pixmap.as_ref(),
        &paint,
        tiny_skia::Transform::identity(),
        None,
    );

    Some(())
}

pub trait TinySkiaPixmapMutExt {
    fn create_rect_mask(
        &self,
        transform: tiny_skia::Transform,
        rect: tiny_skia::Rect,
    ) -> Option<tiny_skia::Mask>;
}

impl TinySkiaPixmapMutExt for tiny_skia::PixmapMut<'_> {
    fn create_rect_mask(
        &self,
        transform: tiny_skia::Transform,
        rect: tiny_skia::Rect,
    ) -> Option<tiny_skia::Mask> {
        let path = tiny_skia::PathBuilder::from_rect(rect);

        let mut mask = tiny_skia::Mask::new(self.width(), self.height())?;
        mask.fill_path(&path, tiny_skia::FillRule::Winding, true, transform);

        Some(mask)
    }
}
