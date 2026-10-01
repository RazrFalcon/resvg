// Copyright 2018 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::OptionLog;

pub struct Context {
    pub max_bbox: tiny_skia::IntRect,
}

pub fn render_nodes(
    parent: &usvg::Group,
    ctx: &Context,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) {
    for node in parent.children() {
        render_node(node, ctx, transform, pixmap);
    }
}

pub fn render_node(
    node: &usvg::Node,
    ctx: &Context,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) {
    match node {
        usvg::Node::Group(group) => {
            render_group(group, ctx, transform, pixmap);
        }
        usvg::Node::Path(path) => {
            crate::path::render(
                path,
                tiny_skia::BlendMode::SourceOver,
                ctx,
                transform,
                pixmap,
            );
        }
        usvg::Node::Image(image) => {
            crate::image::render(image, transform, pixmap);
        }
        usvg::Node::Text(text) => {
            render_group(text.flattened(), ctx, transform, pixmap);
        }
    }
}

fn render_group(
    group: &usvg::Group,
    ctx: &Context,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) -> Option<()> {
    let transform = transform.pre_concat(group.transform());

    if !group.should_isolate() {
        render_nodes(group, ctx, transform, pixmap);
        return Some(());
    }

    let rest = if group.filters().is_empty() {
        tiny_skia::Transform::identity()
    } else {
        let sx = transform.sx.hypot(transform.ky);
        let sy = transform.kx.hypot(transform.sy);
        tiny_skia::Transform::from_row(
            transform.sx / sx,
            transform.ky / sx,
            transform.kx / sy,
            transform.sy / sy,
            0.0,
            0.0,
        )
    };
    let inv_rest = rest.invert()?;

    let max_bbox = ctx.max_bbox.to_rect().transform(inv_rest)?.round_out()?;
    let (layer, ibbox) =
        render_layer(group, &Context { max_bbox }, inv_rest.pre_concat(transform))?;

    let paint = tiny_skia::Paint {
        shader: tiny_skia::Pattern::new(
            layer.as_ref(),
            tiny_skia::SpreadMode::Pad,
            tiny_skia::FilterQuality::Bilinear,
            group.opacity().get(),
            tiny_skia::Transform::from_translate(ibbox.x() as f32, ibbox.y() as f32),
        ),
        blend_mode: convert_blend_mode(group.blend_mode()),
        anti_alias: true,
        ..tiny_skia::Paint::default()
    };
    let path = tiny_skia::PathBuilder::from_rect(ibbox.to_rect());
    pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, rest, None);

    Some(())
}

/// Renders an isolated group into a new layer, returning it and its position.
fn render_layer(
    group: &usvg::Group,
    ctx: &Context,
    transform: tiny_skia::Transform,
) -> Option<(tiny_skia::Pixmap, tiny_skia::IntRect)> {
    let bbox = group.layer_bounding_box().transform(transform)?;

    let mut ibbox = if group.filters().is_empty() {
        // Convert group bbox into an integer one, expanding each side outwards by 2px
        // to make sure that anti-aliased pixels would not be clipped.
        tiny_skia::IntRect::from_xywh(
            (bbox.x().floor() as i32).checked_sub(2)?,
            (bbox.y().floor() as i32).checked_sub(2)?,
            (bbox.width().ceil() as u32).checked_add(4)?,
            (bbox.height().ceil() as u32).checked_add(4)?,
        )?
    } else {
        // The bounding box for groups with filters is special and should not be expanded by 2px,
        // because it's already acting as a clipping region.
        let bbox = tiny_skia::IntRect::from_xywh(
            bbox.x().floor() as i32,
            bbox.y().floor() as i32,
            bbox.width().ceil().max(1.0) as u32,
            bbox.height().ceil().max(1.0) as u32,
        )?;
        // Make sure our filter region is not bigger than 4x the canvas size.
        // This is required mainly to prevent huge filter regions that would tank the performance.
        // It should not affect the final result in any way.
        crate::geom::fit_to_rect(bbox, ctx.max_bbox)?
    };

    // A clipped group can only show what falls inside its clip path, so its
    // layer does not need to be larger than the clip path bounds. Clip paths
    // exported by illustration tools are often much smaller than the content
    // they clip, and rendering and clipping both cost per layer pixel.
    if group.filters().is_empty() && group.mask().is_none() {
        if let Some(clip_bbox) = group
            .clip_path()
            .filter(|clip| is_simple_clip(clip))
            .and_then(|clip| clip_bounds(clip, transform))
        {
            ibbox = ibbox.intersect(&clip_bbox)?;
        }
    }

    // Make sure our layer is not bigger than 4x the canvas size.
    // This is required to prevent huge layers.
    if group.filters().is_empty() {
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

    render_nodes(group, ctx, transform, &mut sub_pixmap.as_mut());

    if !group.filters().is_empty() {
        for filter in group.filters() {
            crate::filter::apply(filter, transform, &mut sub_pixmap);
        }
    }

    if let Some(clip_path) = group.clip_path() {
        crate::clip::apply(clip_path, transform, &mut sub_pixmap);
    }

    if let Some(mask) = group.mask() {
        crate::mask::apply(mask, ctx, transform, &mut sub_pixmap);
    }

    Some((sub_pixmap, ibbox))
}

/// A clip path made of paths only (possibly wrapped in plain groups, as
/// `<use>` produces): no nested clip path, mask, filter or text, so that its
/// bounding box is exactly what it lets through.
fn is_simple_clip(clip: &usvg::ClipPath) -> bool {
    clip.clip_path().is_none() && has_only_paths(clip.root())
}

fn has_only_paths(group: &usvg::Group) -> bool {
    group.children().iter().all(|node| match node {
        usvg::Node::Path(_) => true,
        usvg::Node::Group(group) => {
            group.clip_path().is_none()
                && group.mask().is_none()
                && group.filters().is_empty()
                && has_only_paths(group)
        }
        _ => false,
    })
}

/// The canvas area a clip path can let through, expanded by 2px on each side
/// (like group layers) so that anti-aliased edges are kept.
fn clip_bounds(
    clip: &usvg::ClipPath,
    transform: tiny_skia::Transform,
) -> Option<tiny_skia::IntRect> {
    // Rotated or skewed clip paths keep the full layer, as before: shrinking
    // it changes the anti-aliasing of a few edge pixels.
    let transform = transform.pre_concat(clip.transform());
    if transform.has_skew() {
        return None;
    }

    let bbox = clip
        .root()
        .bounding_box()
        .to_non_zero_rect()?
        .transform(transform)?;

    tiny_skia::IntRect::from_xywh(
        (bbox.x().floor() as i32).checked_sub(2)?,
        (bbox.y().floor() as i32).checked_sub(2)?,
        (bbox.width().ceil() as u32).checked_add(4)?,
        (bbox.height().ceil() as u32).checked_add(4)?,
    )
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
        usvg::BlendMode::PlusLighter => tiny_skia::BlendMode::Plus,
    }
}

#[cfg(test)]
mod tests {

    // Derived from https://github.com/servo/servo/issues/42258.
    #[test]
    fn filter_bbox_outside_int_rect() {
        let svg = r#"<svg filter="url(#f)"><filter id="f" x="2em"><feFlood/></filter><path d="M0 0H1e8V1"/></svg>"#;
        let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
        let mut pixmap = tiny_skia::Pixmap::new(1, 1).unwrap();

        // Just make sure we don't panic.
        crate::render(
            &tree,
            tiny_skia::Transform::identity(),
            &mut pixmap.as_mut(),
        );
    }
}
