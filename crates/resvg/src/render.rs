// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::OptionLog;

pub struct Context {
    pub max_bbox: tiny_skia::IntRect,
}

pub fn render_nodes(
    parent: &usvg::Group,
    ctx: &Context,
    transform: tiny_skia::Transform,
    text_bbox: Option<tiny_skia::NonZeroRect>,
    pixmap: &mut tiny_skia::PixmapMut,
) {
    for node in &parent.children {
        render_node(node, ctx, transform, text_bbox, pixmap);
    }
}

pub fn render_node(
    node: &usvg::Node,
    ctx: &Context,
    transform: tiny_skia::Transform,
    text_bbox: Option<tiny_skia::NonZeroRect>,
    pixmap: &mut tiny_skia::PixmapMut,
) {
    match node {
        usvg::Node::Group(ref group) => {
            render_group(group, ctx, transform, text_bbox, pixmap);
        }
        usvg::Node::Path(ref path) => {
            crate::path::render(
                path,
                tiny_skia::BlendMode::SourceOver,
                ctx,
                text_bbox,
                transform,
                pixmap,
            );
        }
        usvg::Node::Image(ref image) => {
            crate::image::render(image, transform, pixmap);
        }
        usvg::Node::Text(ref text) => {
            if let (Some(bbox), Some(ref flattened)) = (text.bounding_box, &text.flattened) {
                render_group(flattened, ctx, transform, Some(bbox), pixmap);
            } else {
                log::warn!("Text nodes should be flattened before rendering.");
            }
        }
    }
}

fn render_group(
    group: &usvg::Group,
    ctx: &Context,
    transform: tiny_skia::Transform,
    text_bbox: Option<tiny_skia::NonZeroRect>,
    pixmap: &mut tiny_skia::PixmapMut,
) -> Option<()> {
    let transform = transform.pre_concat(group.transform);

    if !group.should_isolate() {
        render_nodes(group, ctx, transform, text_bbox, pixmap);
        return Some(());
    }

    let bbox = group.layer_bounding_box?.transform(transform)?;

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
        let bbox = bbox.to_int_rect();
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

    render_nodes(group, ctx, transform, text_bbox, &mut sub_pixmap.as_mut());

    if !group.filters.is_empty() {
        for filter in &group.filters {
            crate::filter::apply(group, &filter.borrow(), transform, &mut sub_pixmap);
        }
    }

    if let (Some(ref clip_path), Some(object_bbox)) = (&group.clip_path, group.bounding_box) {
        crate::clip::apply(&clip_path.borrow(), object_bbox, transform, &mut sub_pixmap);
    }

    if let (Some(ref mask), Some(object_bbox)) = (&group.mask, group.bounding_box) {
        crate::mask::apply(&mask.borrow(), ctx, object_bbox, transform, &mut sub_pixmap);
    }

    let paint = tiny_skia::PixmapPaint {
        opacity: group.opacity.get(),
        blend_mode: convert_blend_mode(group.blend_mode),
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

pub fn convert_blend_mode(mode: usvg::BlendMode) -> tiny_skia::BlendMode {
    match mode {
        usvg::BlendMode::Normal => tiny_skia::BlendMode::SourceOver,
        usvg::BlendMode::Multiply => tiny_skia::BlendMode::Multiply,
        usvg::BlendMode::Screen => tiny_skia::BlendMode::Screen,
        usvg::BlendMode::Overlay => tiny_skia::BlendMode::Overlay,
        usvg::BlendMode::Darken => tiny_skia::BlendMode::Darken,
        usvg::BlendMode::Lighten => tiny_skia::BlendMode::Lighten,
        usvg::BlendMode::ColorDodge => tiny_skia::BlendMode::ColorDodge,
        usvg::BlendMode::ColorBurn => tiny_skia::BlendMode::ColorBurn,
        usvg::BlendMode::HardLight => tiny_skia::BlendMode::HardLight,
        usvg::BlendMode::SoftLight => tiny_skia::BlendMode::SoftLight,
        usvg::BlendMode::Difference => tiny_skia::BlendMode::Difference,
        usvg::BlendMode::Exclusion => tiny_skia::BlendMode::Exclusion,
        usvg::BlendMode::Hue => tiny_skia::BlendMode::Hue,
        usvg::BlendMode::Saturation => tiny_skia::BlendMode::Saturation,
        usvg::BlendMode::Color => tiny_skia::BlendMode::Color,
        usvg::BlendMode::Luminosity => tiny_skia::BlendMode::Luminosity,
    }
}
