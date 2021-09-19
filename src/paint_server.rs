// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use usvg::TransformFromBBox;

use crate::{ConvTransform, OptionLog, render::{Canvas, RenderState}};

pub fn fill(
    tree: &usvg::Tree,
    fill: &usvg::Fill,
    bbox: usvg::PathBbox,
    path: &tiny_skia::Path,
    anti_alias: bool,
    blend_mode: tiny_skia::BlendMode,
    canvas: &mut Canvas,
) -> Option<()> {
    let pattern_pixmap;

    let mut paint = tiny_skia::Paint::default();

    let opacity = fill.opacity;
    match fill.paint {
        usvg::Paint::Color(c) => {
            let alpha = multiply_a8(c.alpha, opacity.to_u8());
            paint.set_color_rgba8(c.red, c.green, c.blue, alpha);
        }
        usvg::Paint::Link(ref id) => {
            if let Some(node) = tree.defs_by_id(id) {
                match *node.borrow() {
                    usvg::NodeKind::LinearGradient(ref lg) => {
                        prepare_linear(lg, opacity, bbox, &mut paint);
                    }
                    usvg::NodeKind::RadialGradient(ref rg) => {
                        prepare_radial(rg, opacity, bbox, &mut paint);
                    }
                    usvg::NodeKind::Pattern(ref pattern) => {
                        let global_ts = usvg::Transform::from_native(canvas.transform);
                        let (patt_pix, patt_ts)
                            = prepare_pattern_pixmap(tree, &node, pattern, &global_ts, bbox)?;

                        pattern_pixmap = patt_pix;
                        paint.shader = prepare_pattern(&pattern_pixmap, patt_ts, opacity);
                    }
                    _ => {}
                }
            }
        }
    }

    paint.anti_alias = anti_alias;
    paint.blend_mode = blend_mode;

    let rule = if fill.rule == usvg::FillRule::NonZero {
        tiny_skia::FillRule::Winding
    } else {
        tiny_skia::FillRule::EvenOdd
    };

    canvas.pixmap.fill_path(path, &paint, rule, canvas.transform, canvas.clip.as_ref());

    Some(())
}

pub fn stroke(
    tree: &usvg::Tree,
    stroke: &Option<usvg::Stroke>,
    bbox: usvg::PathBbox,
    path: &tiny_skia::Path,
    anti_alias: bool,
    blend_mode: tiny_skia::BlendMode,
    canvas: &mut Canvas,
) -> Option<()> {
    let pattern_pixmap;

    let mut paint = tiny_skia::Paint::default();
    let mut props = tiny_skia::Stroke::default();

    if let Some(ref stroke) = stroke {
        let opacity = stroke.opacity;
        match stroke.paint {
            usvg::Paint::Color(c) => {
                let alpha = multiply_a8(c.alpha, opacity.to_u8());
                paint.set_color_rgba8(c.red, c.green, c.blue, alpha);
            }
            usvg::Paint::Link(ref id) => {
                if let Some(node) = tree.defs_by_id(id) {
                    match *node.borrow() {
                        usvg::NodeKind::LinearGradient(ref lg) => {
                            prepare_linear(lg, opacity, bbox, &mut paint);
                        }
                        usvg::NodeKind::RadialGradient(ref rg) => {
                            prepare_radial(rg, opacity, bbox, &mut paint);
                        }
                        usvg::NodeKind::Pattern(ref pattern) => {
                            let global_ts = usvg::Transform::from_native(canvas.transform);
                            let (patt_pix, patt_ts)
                                = prepare_pattern_pixmap(tree, &node, pattern, &global_ts, bbox)?;

                            pattern_pixmap = patt_pix;
                            paint.shader = prepare_pattern(&pattern_pixmap, patt_ts, opacity);
                        }
                        _ => {}
                    }
                }
            }
        }

        let stroke_cap = match stroke.linecap {
            usvg::LineCap::Butt => tiny_skia::LineCap::Butt,
            usvg::LineCap::Round => tiny_skia::LineCap::Round,
            usvg::LineCap::Square => tiny_skia::LineCap::Square,
        };
        props.line_cap = stroke_cap;

        let stroke_join = match stroke.linejoin {
            usvg::LineJoin::Miter => tiny_skia::LineJoin::Miter,
            usvg::LineJoin::Round => tiny_skia::LineJoin::Round,
            usvg::LineJoin::Bevel => tiny_skia::LineJoin::Bevel,
        };
        props.line_join = stroke_join;

        props.miter_limit = stroke.miterlimit.value() as f32;
        props.width = stroke.width.value() as f32;

        if let Some(ref list) = stroke.dasharray {
            let list: Vec<_> = list.iter().map(|n| *n as f32).collect();
            props.dash = tiny_skia::StrokeDash::new(list, stroke.dashoffset);
        }
    }

    paint.anti_alias = anti_alias;
    paint.blend_mode = blend_mode;

    canvas.pixmap.stroke_path(path, &paint, &props, canvas.transform, canvas.clip.as_ref());

    Some(())
}

fn prepare_linear(
    g: &usvg::LinearGradient,
    opacity: usvg::Opacity,
    bbox: usvg::PathBbox,
    paint: &mut tiny_skia::Paint,
) -> Option<()> {
    let mode = match g.spread_method {
        usvg::SpreadMethod::Pad => tiny_skia::SpreadMode::Pad,
        usvg::SpreadMethod::Reflect => tiny_skia::SpreadMode::Reflect,
        usvg::SpreadMethod::Repeat => tiny_skia::SpreadMode::Repeat,
    };

    let transform = {
        if g.units == usvg::Units::ObjectBoundingBox {
            let bbox = bbox.to_rect()
                .log_none(|| log::warn!("Gradient on zero-sized shapes is not allowed."))?;

            let mut ts = usvg::Transform::from_bbox(bbox);
            ts.append(&g.transform);
            ts.to_native()
        } else {
            g.transform.to_native()
        }
    };

    let mut points = Vec::with_capacity(g.stops.len());
    for stop in &g.stops {
        let alpha = stop.opacity * opacity * usvg::Opacity::new(f64::from(stop.color.alpha) / 255.0);
        let color = tiny_skia::Color::from_rgba8(
            stop.color.red, stop.color.green, stop.color.blue, alpha.to_u8());
        points.push(tiny_skia::GradientStop::new(stop.offset.value() as f32, color))
    }

    let gradient = tiny_skia::LinearGradient::new(
        (g.x1 as f32, g.y1 as f32).into(),
        (g.x2 as f32, g.y2 as f32).into(),
        points,
        mode,
        transform,
    );

    if let Some(gradient) = gradient {
        paint.shader = gradient;
    }

    Some(())
}

fn prepare_radial(
    g: &usvg::RadialGradient,
    opacity: usvg::Opacity,
    bbox: usvg::PathBbox,
    paint: &mut tiny_skia::Paint,
) -> Option<()> {
    let mode = match g.spread_method {
        usvg::SpreadMethod::Pad => tiny_skia::SpreadMode::Pad,
        usvg::SpreadMethod::Reflect => tiny_skia::SpreadMode::Reflect,
        usvg::SpreadMethod::Repeat => tiny_skia::SpreadMode::Repeat,
    };

    let transform = {
        if g.units == usvg::Units::ObjectBoundingBox {
            let bbox = bbox.to_rect()
                .log_none(|| log::warn!("Gradient on zero-sized shapes is not allowed."))?;

            let mut ts = usvg::Transform::from_bbox(bbox);
            ts.append(&g.transform);
            ts.to_native()
        } else {
            g.transform.to_native()
        }
    };

    let mut points = Vec::with_capacity(g.stops.len());
    for stop in &g.stops {
        let alpha = stop.opacity * opacity * usvg::Opacity::new(f64::from(stop.color.alpha) / 255.0);
        let color = tiny_skia::Color::from_rgba8(
            stop.color.red, stop.color.green, stop.color.blue, alpha.to_u8());
        points.push(tiny_skia::GradientStop::new(stop.offset.value() as f32, color))
    }

    let gradient = tiny_skia::RadialGradient::new(
        (g.fx as f32, g.fy as f32).into(),
        (g.cx as f32, g.cy as f32).into(),
        g.r.value() as f32,
        points,
        mode,
        transform,
    );

    if let Some(gradient) = gradient {
        paint.shader = gradient;
    }

    Some(())
}

fn prepare_pattern_pixmap(
    tree: &usvg::Tree,
    pattern_node: &usvg::Node,
    pattern: &usvg::Pattern,
    global_ts: &usvg::Transform,
    bbox: usvg::PathBbox,
) -> Option<(tiny_skia::Pixmap, usvg::Transform)> {
    let r = if pattern.units == usvg::Units::ObjectBoundingBox {
        let bbox = bbox.to_rect()
            .log_none(|| log::warn!("Pattern on zero-sized shapes is not allowed."))?;

        pattern.rect.bbox_transform(bbox)
    } else {
        pattern.rect
    };

    let mut ts2 = global_ts.clone();
    ts2.append(&pattern.transform);
    let (sx, sy) = ts2.get_scale();

    let img_size = usvg::Size::new(r.width() * sx, r.height() * sy)?.to_screen_size();
    let mut pixmap = tiny_skia::Pixmap::new(img_size.width(), img_size.height())?;
    let mut canvas = Canvas::from(pixmap.as_mut());

    canvas.scale(sx as f32, sy as f32);
    if let Some(vbox) = pattern.view_box {
        let ts = usvg::utils::view_box_to_transform(vbox.rect, vbox.aspect, r.size());
        canvas.apply_transform(ts.to_native());
    } else if pattern.content_units == usvg::Units::ObjectBoundingBox {
        // 'Note that this attribute has no effect if attribute `viewBox` is specified.'

        // We don't use Transform::from_bbox(bbox) because `x` and `y` should be
        // ignored for some reasons...
        canvas.scale(bbox.width() as f32, bbox.height() as f32);
    }

    crate::render::render_group(tree, pattern_node, &mut RenderState::Ok, &mut canvas);

    let mut ts = usvg::Transform::default();
    ts.append(&pattern.transform);
    ts.translate(r.x(), r.y());
    ts.scale(1.0 / sx, 1.0 / sy);

    Some((pixmap, ts))
}

fn prepare_pattern(
    pixmap: &tiny_skia::Pixmap,
    ts: usvg::Transform,
    opacity: usvg::Opacity,
) -> tiny_skia::Shader {
    tiny_skia::Pattern::new(
        pixmap.as_ref(),
        tiny_skia::SpreadMode::Repeat,
        tiny_skia::FilterQuality::Bicubic,
        opacity.value() as f32,
        ts.to_native(),
    )
}

/// Return a*b/255, rounding any fractional bits.
pub fn multiply_a8(c: u8, a: u8) -> u8 {
    let prod = u32::from(c) * u32::from(a) + 128;
    ((prod + (prod >> 8)) >> 8) as u8
}
