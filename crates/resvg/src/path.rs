// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::render::Context;
use crate::OptionLog;

pub fn render(
    path: &usvg::Path,
    blend_mode: tiny_skia::BlendMode,
    ctx: &Context,
    text_bbox: Option<tiny_skia::NonZeroRect>,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) {
    if path.visibility != usvg::Visibility::Visible {
        return;
    }

    let mut object_bbox = match path.bounding_box {
        Some(v) => v,
        None => {
            log::warn!(
                "Node bounding box should be already calculated. \
                See `usvg::Tree::postprocess`"
            );
            return;
        }
    };

    if let Some(text_bbox) = text_bbox {
        object_bbox = text_bbox.to_rect();
    }

    if path.paint_order == usvg::PaintOrder::FillAndStroke {
        fill_path(path, blend_mode, ctx, object_bbox, transform, pixmap);
        stroke_path(path, blend_mode, ctx, object_bbox, transform, pixmap);
    } else {
        stroke_path(path, blend_mode, ctx, object_bbox, transform, pixmap);
        fill_path(path, blend_mode, ctx, object_bbox, transform, pixmap);
    }
}

pub fn fill_path(
    path: &usvg::Path,
    blend_mode: tiny_skia::BlendMode,
    ctx: &Context,
    object_bbox: tiny_skia::Rect,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) -> Option<()> {
    let fill = path.fill.as_ref()?;

    // Horizontal and vertical lines cannot be filled. Skip.
    if path.data.bounds().width() == 0.0 || path.data.bounds().height() == 0.0 {
        return None;
    }

    let object_bbox = object_bbox.to_non_zero_rect();

    let rule = match fill.rule {
        usvg::FillRule::NonZero => tiny_skia::FillRule::Winding,
        usvg::FillRule::EvenOdd => tiny_skia::FillRule::EvenOdd,
    };

    let pattern_pixmap;
    let mut paint = tiny_skia::Paint::default();
    match fill.paint {
        usvg::Paint::Color(c) => {
            paint.set_color_rgba8(c.red, c.green, c.blue, fill.opacity.to_u8());
        }
        usvg::Paint::LinearGradient(ref lg) => {
            paint.shader = convert_linear_gradient(lg, fill.opacity, object_bbox)?;
        }
        usvg::Paint::RadialGradient(ref rg) => {
            paint.shader = convert_radial_gradient(rg, fill.opacity, object_bbox)?;
        }
        usvg::Paint::Pattern(ref pattern) => {
            let (patt_pix, patt_ts) =
                render_pattern_pixmap(&pattern.borrow(), ctx, transform, object_bbox)?;

            pattern_pixmap = patt_pix;
            paint.shader = tiny_skia::Pattern::new(
                pattern_pixmap.as_ref(),
                tiny_skia::SpreadMode::Repeat,
                tiny_skia::FilterQuality::Bicubic,
                fill.opacity.get(),
                patt_ts,
            )
        }
    }
    paint.anti_alias = path.rendering_mode.use_shape_antialiasing();
    paint.blend_mode = blend_mode;

    pixmap.fill_path(&path.data, &paint, rule, transform, None);

    Some(())
}

fn stroke_path(
    path: &usvg::Path,
    blend_mode: tiny_skia::BlendMode,
    ctx: &Context,
    object_bbox: tiny_skia::Rect,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) -> Option<()> {
    let stroke = path.stroke.as_ref()?;
    let object_bbox = object_bbox.to_non_zero_rect();

    let pattern_pixmap;
    let mut paint = tiny_skia::Paint::default();
    match stroke.paint {
        usvg::Paint::Color(c) => {
            paint.set_color_rgba8(c.red, c.green, c.blue, stroke.opacity.to_u8());
        }
        usvg::Paint::LinearGradient(ref lg) => {
            paint.shader = convert_linear_gradient(lg, stroke.opacity, object_bbox)?;
        }
        usvg::Paint::RadialGradient(ref rg) => {
            paint.shader = convert_radial_gradient(rg, stroke.opacity, object_bbox)?;
        }
        usvg::Paint::Pattern(ref pattern) => {
            let (patt_pix, patt_ts) =
                render_pattern_pixmap(&pattern.borrow(), ctx, transform, object_bbox)?;

            pattern_pixmap = patt_pix;
            paint.shader = tiny_skia::Pattern::new(
                pattern_pixmap.as_ref(),
                tiny_skia::SpreadMode::Repeat,
                tiny_skia::FilterQuality::Bicubic,
                stroke.opacity.get(),
                patt_ts,
            )
        }
    }
    paint.anti_alias = path.rendering_mode.use_shape_antialiasing();
    paint.blend_mode = blend_mode;

    pixmap.stroke_path(&path.data, &paint, &stroke.to_tiny_skia(), transform, None);

    Some(())
}

fn convert_linear_gradient(
    gradient: &usvg::LinearGradient,
    opacity: usvg::Opacity,
    object_bbox: Option<tiny_skia::NonZeroRect>,
) -> Option<tiny_skia::Shader> {
    let (mode, transform, points) = convert_base_gradient(gradient, opacity, object_bbox)?;

    let shader = tiny_skia::LinearGradient::new(
        (gradient.x1, gradient.y1).into(),
        (gradient.x2, gradient.y2).into(),
        points,
        mode,
        transform,
    )?;

    Some(shader)
}

fn convert_radial_gradient(
    gradient: &usvg::RadialGradient,
    opacity: usvg::Opacity,
    object_bbox: Option<tiny_skia::NonZeroRect>,
) -> Option<tiny_skia::Shader> {
    let (mode, transform, points) = convert_base_gradient(gradient, opacity, object_bbox)?;

    let shader = tiny_skia::RadialGradient::new(
        (gradient.fx, gradient.fy).into(),
        (gradient.cx, gradient.cy).into(),
        gradient.r.get(),
        points,
        mode,
        transform,
    )?;

    Some(shader)
}

fn convert_base_gradient(
    gradient: &usvg::BaseGradient,
    opacity: usvg::Opacity,
    object_bbox: Option<tiny_skia::NonZeroRect>,
) -> Option<(
    tiny_skia::SpreadMode,
    tiny_skia::Transform,
    Vec<tiny_skia::GradientStop>,
)> {
    let mode = match gradient.spread_method {
        usvg::SpreadMethod::Pad => tiny_skia::SpreadMode::Pad,
        usvg::SpreadMethod::Reflect => tiny_skia::SpreadMode::Reflect,
        usvg::SpreadMethod::Repeat => tiny_skia::SpreadMode::Repeat,
    };

    let transform = if gradient.units == usvg::Units::ObjectBoundingBox {
        let bbox =
            object_bbox.log_none(|| log::warn!("Gradient on zero-sized shapes is not allowed."))?;
        let ts = tiny_skia::Transform::from_bbox(bbox);
        ts.pre_concat(gradient.transform)
    } else {
        gradient.transform
    };

    let mut points = Vec::with_capacity(gradient.stops.len());
    for stop in &gradient.stops {
        let alpha = stop.opacity * opacity;
        let color = tiny_skia::Color::from_rgba8(
            stop.color.red,
            stop.color.green,
            stop.color.blue,
            alpha.to_u8(),
        );
        points.push(tiny_skia::GradientStop::new(stop.offset.get(), color))
    }

    Some((mode, transform, points))
}

fn render_pattern_pixmap(
    pattern: &usvg::Pattern,
    ctx: &Context,
    transform: tiny_skia::Transform,
    object_bbox: Option<tiny_skia::NonZeroRect>,
) -> Option<(tiny_skia::Pixmap, tiny_skia::Transform)> {
    let content_transform =
        if pattern.content_units == usvg::Units::ObjectBoundingBox && pattern.view_box.is_none() {
            let bbox = object_bbox
                .log_none(|| log::warn!("Pattern on zero-sized shapes is not allowed."))?;

            // No need to shift patterns.
            tiny_skia::Transform::from_scale(bbox.width(), bbox.height())
        } else {
            tiny_skia::Transform::default()
        };

    let rect = if pattern.units == usvg::Units::ObjectBoundingBox {
        let bbox =
            object_bbox.log_none(|| log::warn!("Pattern on zero-sized shapes is not allowed."))?;

        pattern.rect.bbox_transform(bbox)
    } else {
        pattern.rect
    };

    let (sx, sy) = {
        let ts2 = transform.pre_concat(pattern.transform);
        ts2.get_scale()
    };

    let img_size = tiny_skia::IntSize::from_wh(
        (rect.width() * sx).round() as u32,
        (rect.height() * sy).round() as u32,
    )?;
    let mut pixmap = tiny_skia::Pixmap::new(img_size.width(), img_size.height())?;

    let mut transform = tiny_skia::Transform::from_scale(sx, sy);
    if let Some(vbox) = pattern.view_box {
        let ts = usvg::utils::view_box_to_transform(vbox.rect, vbox.aspect, rect.size());
        transform = transform.pre_concat(ts);
    }

    transform = transform.pre_concat(content_transform);

    crate::render::render_nodes(&pattern.root, ctx, transform, None, &mut pixmap.as_mut());

    let mut ts = tiny_skia::Transform::default();
    ts = ts.pre_concat(pattern.transform);
    ts = ts.pre_translate(rect.x(), rect.y());
    ts = ts.pre_scale(1.0 / sx, 1.0 / sy);

    Some((pixmap, ts))
}
