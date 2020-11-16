// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::render::prelude::*;

pub fn fill(
    tree: &usvg::Tree,
    fill: &usvg::Fill,
    bbox: Rect,
    path: &tiny_skia::Path,
    anti_alias: bool,
    blend_mode: tiny_skia::BlendMode,
    canvas: &mut tiny_skia::Canvas,
) {
    let pattern_pixmap;

    let mut paint = tiny_skia::Paint::default();

    let opacity = fill.opacity;
    match fill.paint {
        usvg::Paint::Color(c) => {
            paint.set_color_rgba8(c.red, c.green, c.blue, opacity.to_u8());
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
                        let global_ts = usvg::Transform::from_native(canvas.get_transform());
                        let (patt_pix, patt_ts)
                            = try_opt!(prepare_pattern_pixmap(&node, pattern, &global_ts, bbox));

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

    canvas.fill_path(path, &paint, rule);
}

pub fn stroke(
    tree: &usvg::Tree,
    stroke: &Option<usvg::Stroke>,
    bbox: Rect,
    path: &tiny_skia::Path,
    anti_alias: bool,
    blend_mode: tiny_skia::BlendMode,
    canvas: &mut tiny_skia::Canvas,
) {
    let pattern_pixmap;

    let mut paint = tiny_skia::Paint::default();
    let mut props = tiny_skia::Stroke::default();

    if let Some(ref stroke) = stroke {
        let opacity = stroke.opacity;
        match stroke.paint {
            usvg::Paint::Color(c) => {
                paint.set_color_rgba8(c.red, c.green, c.blue, opacity.to_u8());
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
                            let global_ts = usvg::Transform::from_native(canvas.get_transform());
                            let (patt_pix, patt_ts)
                                = try_opt!(prepare_pattern_pixmap(&node, pattern, &global_ts, bbox));

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

    canvas.stroke_path(&path, &paint, &props);
}

fn prepare_linear(
    g: &usvg::LinearGradient,
    opacity: usvg::Opacity,
    bbox: Rect,
    paint: &mut tiny_skia::Paint,
) {
    let mode = match g.spread_method {
        usvg::SpreadMethod::Pad => tiny_skia::SpreadMode::Pad,
        usvg::SpreadMethod::Reflect => tiny_skia::SpreadMode::Reflect,
        usvg::SpreadMethod::Repeat => tiny_skia::SpreadMode::Repeat,
    };

    let transform = {
        if g.units == usvg::Units::ObjectBoundingBox {
            let mut ts = usvg::Transform::from_bbox(bbox);
            ts.append(&g.transform);
            ts.to_native()
        } else {
            g.transform.to_native()
        }
    };

    let mut points = Vec::with_capacity(g.stops.len());
    for stop in &g.stops {
        let a = stop.opacity * opacity;
        let color = tiny_skia::Color::from_rgba8(stop.color.red, stop.color.green, stop.color.blue, a.to_u8());
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
}

fn prepare_radial(
    g: &usvg::RadialGradient,
    opacity: usvg::Opacity,
    bbox: Rect,
    paint: &mut tiny_skia::Paint,
) {
    let mode = match g.spread_method {
        usvg::SpreadMethod::Pad => tiny_skia::SpreadMode::Pad,
        usvg::SpreadMethod::Reflect => tiny_skia::SpreadMode::Reflect,
        usvg::SpreadMethod::Repeat => tiny_skia::SpreadMode::Repeat,
    };

    let transform = {
        if g.units == usvg::Units::ObjectBoundingBox {
            let mut ts = usvg::Transform::from_bbox(bbox);
            ts.append(&g.transform);
            ts.to_native()
        } else {
            g.transform.to_native()
        }
    };

    let mut points = Vec::with_capacity(g.stops.len());
    for stop in &g.stops {
        let a = stop.opacity * opacity;
        let color = tiny_skia::Color::from_rgba8(stop.color.red, stop.color.green, stop.color.blue, a.to_u8());
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
}

fn prepare_pattern_pixmap(
    pattern_node: &usvg::Node,
    pattern: &usvg::Pattern,
    global_ts: &usvg::Transform,
    bbox: Rect,
) -> Option<(tiny_skia::Pixmap, usvg::Transform)> {
    let r = if pattern.units == usvg::Units::ObjectBoundingBox {
        pattern.rect.bbox_transform(bbox)
    } else {
        pattern.rect
    };

    let (sx, sy) = global_ts.get_scale();

    let img_size = Size::new(r.width() * sx as f64, r.height() * sy as f64)?.to_screen_size();
    let mut canvas = tiny_skia::Canvas::new(img_size.width(), img_size.height())?;

    canvas.scale(sx as f32, sy as f32);
    if let Some(vbox) = pattern.view_box {
        let ts = usvg::utils::view_box_to_transform(vbox.rect, vbox.aspect, r.size());
        canvas.apply_transform(&ts.to_native());
    } else if pattern.content_units == usvg::Units::ObjectBoundingBox {
        // 'Note that this attribute has no effect if attribute `viewBox` is specified.'

        // We don't use Transform::from_bbox(bbox) because `x` and `y` should be
        // ignored for some reasons...
        canvas.scale(bbox.width() as f32, bbox.height() as f32);
    }

    let mut layers = Layers::new(img_size);
    crate::render::render_group(pattern_node, &mut RenderState::Ok, &mut layers, &mut canvas);

    let mut ts = usvg::Transform::default();
    ts.append(&pattern.transform);
    ts.translate(r.x(), r.y());
    ts.scale(1.0 / sx as f64, 1.0 / sy as f64);

    Some((canvas.pixmap, ts))
}

fn prepare_pattern(
    pixmap: &tiny_skia::Pixmap,
    ts: usvg::Transform,
    opacity: usvg::Opacity,
) -> tiny_skia::Shader {
    tiny_skia::Pattern::new(
        pixmap,
        tiny_skia::SpreadMode::Repeat,
        tiny_skia::FilterQuality::Bicubic,
        opacity.value() as f32,
        ts.to_native(),
    )
}
