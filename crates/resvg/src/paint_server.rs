// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::rc::Rc;

use crate::render::Context;
use crate::tree::{Node, OptionLog};

pub struct Pattern {
    pub rect: tiny_skia::NonZeroRect,
    pub view_box: Option<usvg::ViewBox>,
    pub opacity: usvg::Opacity,
    pub transform: tiny_skia::Transform,
    pub content_transform: tiny_skia::Transform,
    pub children: Vec<Node>,
}

#[derive(Clone)]
pub enum Paint {
    Shader(tiny_skia::Shader<'static>),
    Pattern(Rc<Pattern>),
}

pub fn convert(
    paint: &usvg::Paint,
    opacity: usvg::Opacity,
    object_bbox: Option<tiny_skia::NonZeroRect>,
) -> Option<Paint> {
    match paint {
        usvg::Paint::Color(c) => {
            let c = tiny_skia::Color::from_rgba8(c.red, c.green, c.blue, opacity.to_u8());
            Some(Paint::Shader(tiny_skia::Shader::SolidColor(c)))
        }
        usvg::Paint::LinearGradient(ref lg) => convert_linear_gradient(lg, opacity, object_bbox),
        usvg::Paint::RadialGradient(ref rg) => convert_radial_gradient(rg, opacity, object_bbox),
        usvg::Paint::Pattern(ref patt) => convert_pattern(patt, opacity, object_bbox),
    }
}

fn convert_linear_gradient(
    gradient: &usvg::LinearGradient,
    opacity: usvg::Opacity,
    object_bbox: Option<tiny_skia::NonZeroRect>,
) -> Option<Paint> {
    let (mode, transform, points) = convert_base_gradient(gradient, opacity, object_bbox)?;

    let shader = tiny_skia::LinearGradient::new(
        (gradient.x1, gradient.y1).into(),
        (gradient.x2, gradient.y2).into(),
        points,
        mode,
        transform,
    )?;

    Some(Paint::Shader(shader))
}

fn convert_radial_gradient(
    gradient: &usvg::RadialGradient,
    opacity: usvg::Opacity,
    object_bbox: Option<tiny_skia::NonZeroRect>,
) -> Option<Paint> {
    let (mode, transform, points) = convert_base_gradient(gradient, opacity, object_bbox)?;

    let shader = tiny_skia::RadialGradient::new(
        (gradient.fx, gradient.fy).into(),
        (gradient.cx, gradient.cy).into(),
        gradient.r.get(),
        points,
        mode,
        transform,
    )?;

    Some(Paint::Shader(shader))
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

fn convert_pattern(
    pattern: &usvg::Pattern,
    opacity: usvg::Opacity,
    object_bbox: Option<tiny_skia::NonZeroRect>,
) -> Option<Paint> {
    let content_transform =
        if pattern.content_units == usvg::Units::ObjectBoundingBox && pattern.view_box.is_none() {
            let bbox = object_bbox
                .log_none(|| log::warn!("Pattern on zero-sized shapes is not allowed."))?;

            // No need to shift patterns.
            tiny_skia::Transform::from_scale(bbox.width(), bbox.height())
        } else {
            tiny_skia::Transform::default()
        };

    let (children, _) = crate::tree::convert_node(pattern.root.clone());
    if children.is_empty() {
        return None;
    }

    let rect = if pattern.units == usvg::Units::ObjectBoundingBox {
        let bbox =
            object_bbox.log_none(|| log::warn!("Pattern on zero-sized shapes is not allowed."))?;

        pattern.rect.bbox_transform(bbox)
    } else {
        pattern.rect
    };

    Some(Paint::Pattern(Rc::new(Pattern {
        rect,
        view_box: pattern.view_box,
        opacity,
        transform: pattern.transform,
        content_transform,
        children,
    })))
}

pub fn prepare_pattern_pixmap(
    pattern: &Pattern,
    ctx: &Context,
    transform: tiny_skia::Transform,
) -> Option<(tiny_skia::Pixmap, tiny_skia::Transform)> {
    let (sx, sy) = {
        let ts2 = transform.pre_concat(pattern.transform);
        ts2.get_scale()
    };

    let img_size = tiny_skia::IntSize::from_wh(
        (pattern.rect.width() * sx).round() as u32,
        (pattern.rect.height() * sy).round() as u32,
    )?;
    let mut pixmap = tiny_skia::Pixmap::new(img_size.width(), img_size.height())?;

    let mut transform = tiny_skia::Transform::from_scale(sx, sy);
    if let Some(vbox) = pattern.view_box {
        let ts = usvg::utils::view_box_to_transform(vbox.rect, vbox.aspect, pattern.rect.size());
        transform = transform.pre_concat(ts);
    }

    transform = transform.pre_concat(pattern.content_transform);

    crate::render::render_nodes(&pattern.children, ctx, transform, &mut pixmap.as_mut());

    let mut ts = tiny_skia::Transform::default();
    ts = ts.pre_concat(pattern.transform);
    ts = ts.pre_translate(pattern.rect.x(), pattern.rect.y());
    ts = ts.pre_scale(1.0 / sx, 1.0 / sy);

    Some((pixmap, ts))
}
