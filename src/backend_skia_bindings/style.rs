// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::skia;
use usvg::try_opt;

use crate::prelude::*;
use crate::backend_utils::*;
use super::SkiaFlatRender;

pub fn fill(
    tree: &usvg::Tree,
    fill: &Option<usvg::Fill>,
    opt: &Options,
    bbox: Rect,
    global_ts: usvg::Transform,
) -> skia::Paint {
    let mut paint = skia::Paint::default();
    paint.set_style(skia::PaintStyle::Fill);

    if let Some(ref fill) = fill {
        let opacity = fill.opacity;
        match fill.paint {
            usvg::Paint::Color(c) => {
                let a = f64_bound(0.0, opacity.value() * 255.0, 255.0) as u8;
                let color = skia::Color::from_argb(a, c.red, c.green, c.blue);
                paint.set_color(color);
            },
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
                            prepare_pattern(&node, pattern, opt, global_ts, bbox, opacity, &mut paint);
                        }
                        _ => {}
                    }
                }
            },
        }
    }

    paint
}

pub fn stroke(
    tree: &usvg::Tree,
    stroke: &Option<usvg::Stroke>,
    opt: &Options,
    bbox: Rect,
    global_ts: usvg::Transform,
) -> skia::Paint {
    let mut paint = skia::Paint::default();
    paint.set_style(skia::PaintStyle::Stroke);

    if let Some(ref stroke) = stroke {
        let opacity = stroke.opacity;
        match stroke.paint {
            usvg::Paint::Color(c) => {
                let a = f64_bound(0.0, opacity.value() * 255.0, 255.0) as u8;
                let color = skia::Color::from_argb(a, c.red, c.green, c.blue);
                paint.set_color(color);
            },
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
                            prepare_pattern(&node, pattern, opt, global_ts, bbox, opacity, &mut paint);
                        }
                        _ => {}
                    }
                }
            },
        }

        let stroke_cap = match stroke.linecap {
            usvg::LineCap::Butt => skia::paint::Cap::Butt,
            usvg::LineCap::Round => skia::paint::Cap::Round,
            usvg::LineCap::Square => skia::paint::Cap::Square,
        };
        paint.set_stroke_cap(stroke_cap);

        let stroke_join = match stroke.linejoin {
            usvg::LineJoin::Miter => skia::paint::Join::Miter,
            usvg::LineJoin::Round => skia::paint::Join::Round,
            usvg::LineJoin::Bevel => skia::paint::Join::Bevel,
        };
        paint.set_stroke_join(stroke_join);

        paint.set_stroke_miter(stroke.miterlimit.value() as f32);
        paint.set_stroke_width(stroke.width.value() as f32);

        if let Some(ref list) = stroke.dasharray {
            let list: Vec<_> = list.iter().map(|n| *n as f32).collect();

            let path_effect = skia::dash_path_effect::new(&list, stroke.dashoffset);
            paint.set_path_effect(path_effect);
        }
    }

    paint
}

fn prepare_linear(
    g: &usvg::LinearGradient,
    opacity: usvg::Opacity,
    bbox: Rect,
    paint: &mut skia::Paint,
) {
    let start_point = skia::Point{x: g.x1 as f32, y: g.y1 as f32};
    let end_point = skia::Point{x: g.x2 as f32, y: g.y2 as f32};
    let base_gradient = prepare_base_gradient(g, opacity, &bbox);

    let shader = skia::Shader::linear_gradient(
        (start_point, end_point),
        base_gradient.colors.as_slice(),
        Some(base_gradient.positions.as_slice()),
        base_gradient.tile_mode,
        None,
        Some(&base_gradient.matrix),
    );
    paint.set_shader(shader);
}

fn prepare_radial(
    g: &usvg::RadialGradient,
    opacity: usvg::Opacity,
    bbox: Rect,
    paint: &mut skia::Paint,
) {
    let start_point = skia::Point{x: g.fx as f32, y: g.fy as f32};
    let end_point = skia::Point{x: g.cx as f32, y: g.cy as f32};
    let base_gradient = prepare_base_gradient(g, opacity, &bbox);

    let shader = skia::Shader::two_point_conical_gradient(
        start_point,
        0.0,
        end_point,
        g.r.value() as f32,
        base_gradient.colors.as_slice(),
        Some(base_gradient.positions.as_slice()),
        base_gradient.tile_mode,
        None,
        Some(&base_gradient.matrix),
    );

    paint.set_shader(shader);
}

fn prepare_base_gradient(
    g: & usvg::BaseGradient,
    opacity: usvg::Opacity,
    bbox: &Rect
) -> Gradient {
    let tile_mode = match g.spread_method {
        usvg::SpreadMethod::Pad => skia::TileMode::Clamp,
        usvg::SpreadMethod::Reflect => skia::TileMode::Mirror,
        usvg::SpreadMethod::Repeat => skia::TileMode::Repeat,
    };
    let matrix = {
            if g.units == usvg::Units::ObjectBoundingBox {
                let mut ts = usvg::Transform::from_bbox(*bbox);
                ts.append(&g.transform);
                ts.to_native()
            } else {
                g.transform.to_native()
            }
    };

    let mut colors: Vec<skia::Color> = Vec::new();
    let mut positions: Vec<f32> = Vec::new();

    for stop in &g.stops {
        let a = (stop.opacity.value() * opacity.value() * 255.0) as u8;
        let color = skia::Color::from_argb(a, stop.color.red, stop.color.green, stop.color.blue);
        colors.push(color);
        positions.push(stop.offset.value() as f32);
    }

    Gradient {
        colors,
        tile_mode,
        positions,
        matrix,
    }
}

fn prepare_pattern(
    pattern_node: &usvg::Node,
    pattern: &usvg::Pattern,
    opt: &Options,
    global_ts: usvg::Transform,
    bbox: Rect,
    opacity: usvg::Opacity,
    paint: &mut skia::Paint,
) {
    let r = if pattern.units == usvg::Units::ObjectBoundingBox {
        pattern.rect.bbox_transform(bbox)
    } else {
        pattern.rect
    };

    let (sx, sy) = global_ts.get_scale();

    let img_size = try_opt!(Size::new(r.width() * sx, r.height() * sy)).to_screen_size();
    let mut surface =  try_create_surface!(img_size, ());
    let mut canvas = surface.canvas();
    canvas.clear(skia::Color::TRANSPARENT);

    canvas.scale((sx as f32, sy as f32));
    if let Some(vbox) = pattern.view_box {
        let ts = utils::view_box_to_transform(vbox.rect, vbox.aspect, r.size());
        canvas.concat(&ts.to_native());
    } else if pattern.content_units == usvg::Units::ObjectBoundingBox {
        // 'Note that this attribute has no effect if attribute `viewBox` is specified.'

        // We don't use Transform::from_bbox(bbox) because `x` and `y` should be
        // ignored for some reasons...
        canvas.scale((bbox.width() as f32, bbox.height() as f32));
    }

    let ref tree = pattern_node.tree();
    let mut render = SkiaFlatRender::new(tree, opt, img_size, &mut canvas);
    render.render_group(pattern_node);
    render.finish();

    let mut ts = usvg::Transform::default();
    ts.append(&pattern.transform);
    ts.translate(r.x(), r.y());
    ts.scale(1.0 / sx, 1.0 / sy);
    let image = surface.image_snapshot();
    let shader = image.to_shader(
        Some((skia::TileMode::Repeat, skia::TileMode::Repeat)),
        Some(&ts.to_native()),
    );

    paint.set_shader(shader);

    if !opacity.is_default() {
        let a = f64_bound(0.0, opacity.value() * 255.0, 255.0) as u8;
        paint.set_alpha(a);
    };
}

struct Gradient {
    colors: Vec<skia::Color>,
    tile_mode: skia::TileMode,
    positions: Vec<f32>,
    matrix: skia::Matrix,
}
