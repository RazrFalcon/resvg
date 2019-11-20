// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::qt;

use crate::{prelude::*, ConvTransform};


pub fn fill(
    tree: &usvg::Tree,
    fill: &Option<usvg::Fill>,
    opt: &Options,
    bbox: Rect,
    p: &mut qt::Painter,
) {
    match *fill {
        Some(ref fill) => {
            let mut brush = qt::Brush::new();
            let opacity = fill.opacity;

            match fill.paint {
                usvg::Paint::Color(c) => {
                    let a = f64_bound(0.0, opacity.value() * 255.0, 255.0) as u8;
                    brush.set_color(c.red, c.green, c.blue, a);
                }
                usvg::Paint::Link(ref id) => {
                    if let Some(node) = tree.defs_by_id(id) {
                        match *node.borrow() {
                            usvg::NodeKind::LinearGradient(ref lg) => {
                                prepare_linear(lg, opacity, bbox, &mut brush);
                            }
                            usvg::NodeKind::RadialGradient(ref rg) => {
                                prepare_radial(rg, opacity, bbox, &mut brush);
                            }
                            usvg::NodeKind::Pattern(ref pattern) => {
                                let ts = p.get_transform();
                                prepare_pattern(&node, pattern, opt, ts, bbox, opacity, &mut brush);
                            }
                            _ => {}
                        }
                    }
                }
            }

            p.set_brush(brush);
        }
        None => {
            p.reset_brush();
        }
    }
}

pub fn stroke(
    tree: &usvg::Tree,
    stroke: &Option<usvg::Stroke>,
    opt: &Options,
    bbox: Rect,
    p: &mut qt::Painter,
) {
    match *stroke {
        Some(ref stroke) => {
            let mut pen = qt::Pen::new();
            let opacity = stroke.opacity;

            match stroke.paint {
                usvg::Paint::Color(c) => {
                    let a = f64_bound(0.0, opacity.value() * 255.0, 255.0) as u8;
                    pen.set_color(c.red, c.green, c.blue, a);
                }
                usvg::Paint::Link(ref id) => {
                    let mut brush = qt::Brush::new();

                    if let Some(node) = tree.defs_by_id(id) {
                        match *node.borrow() {
                            usvg::NodeKind::LinearGradient(ref lg) => {
                                prepare_linear(lg, opacity, bbox, &mut brush);
                            }
                            usvg::NodeKind::RadialGradient(ref rg) => {
                                prepare_radial(rg, opacity, bbox, &mut brush);
                            }
                            usvg::NodeKind::Pattern(ref pattern) => {
                                let ts = p.get_transform();
                                prepare_pattern(&node, pattern, opt, ts, bbox, opacity, &mut brush);
                            }
                            _ => {}
                        }
                    }

                    pen.set_brush(brush);
                }
            }

            let linecap = match stroke.linecap {
                usvg::LineCap::Butt => qt::LineCap::Flat,
                usvg::LineCap::Round => qt::LineCap::Round,
                usvg::LineCap::Square => qt::LineCap::Square,
            };
            pen.set_line_cap(linecap);

            let linejoin = match stroke.linejoin {
                usvg::LineJoin::Miter => qt::LineJoin::Miter,
                usvg::LineJoin::Round => qt::LineJoin::Round,
                usvg::LineJoin::Bevel => qt::LineJoin::Bevel,
            };
            pen.set_line_join(linejoin);

            pen.set_miter_limit(stroke.miterlimit.value());
            pen.set_width(stroke.width.value());

            if let Some(ref list) = stroke.dasharray {
                pen.set_dash_offset(stroke.dashoffset as f64);
                pen.set_dash_array(list);
            }

            p.set_pen(pen);
        }
        None => {
            p.reset_pen();
        }
    }
}

fn prepare_linear(
    g: &usvg::LinearGradient,
    opacity: usvg::Opacity,
    bbox: Rect,
    brush: &mut qt::Brush,
) {
    let mut grad = qt::LinearGradient::new(g.x1, g.y1, g.x2, g.y2);
    prepare_base_gradient(&g.base, opacity, &mut grad);

    brush.set_linear_gradient(grad);
    transform_gradient(&g.base, bbox, brush);
}

fn prepare_radial(
    g: &usvg::RadialGradient,
    opacity: usvg::Opacity,
    bbox: Rect,
    brush: &mut qt::Brush,
) {
    let mut grad = qt::RadialGradient::new(g.cx, g.cy, g.fx, g.fy, g.r.value());
    prepare_base_gradient(&g.base, opacity, &mut grad);

    brush.set_radial_gradient(grad);
    transform_gradient(&g.base, bbox, brush);
}

fn prepare_base_gradient(
    g: &usvg::BaseGradient,
    opacity: usvg::Opacity,
    grad: &mut dyn qt::Gradient,
) {
    let spread_method = match g.spread_method {
        usvg::SpreadMethod::Pad => qt::Spread::Pad,
        usvg::SpreadMethod::Reflect => qt::Spread::Reflect,
        usvg::SpreadMethod::Repeat => qt::Spread::Repeat,
    };
    grad.set_spread(spread_method);

    for stop in &g.stops {
        grad.set_color_at(
            stop.offset.value(),
            stop.color.red,
            stop.color.green,
            stop.color.blue,
            (stop.opacity.value() * opacity.value() * 255.0) as u8,
        );
    }
}

fn transform_gradient(
    g: &usvg::BaseGradient,
    bbox: Rect,
    brush: &mut qt::Brush,
) {
    // We don't use `QGradient::setCoordinateMode` because it works incorrectly.
    //
    // See QTBUG-67995

    if g.units == usvg::Units::ObjectBoundingBox {
        let mut ts = usvg::Transform::from_bbox(bbox);
        ts.append(&g.transform);
        brush.set_transform(ts.to_native());
    } else {
        brush.set_transform(g.transform.to_native());
    }
}

fn prepare_pattern(
    pattern_node: &usvg::Node,
    pattern: &usvg::Pattern,
    opt: &Options,
    global_ts: qt::Transform,
    bbox: Rect,
    opacity: usvg::Opacity,
    brush: &mut qt::Brush,
) {
    let r = if pattern.units == usvg::Units::ObjectBoundingBox {
        pattern.rect.bbox_transform(bbox)
    } else {
        pattern.rect
    };

    let global_ts = usvg::Transform::from_native(&global_ts);
    let (sx, sy) = global_ts.get_scale();

    let img_size = try_opt!(Size::new(r.width() * sx, r.height() * sy)).to_screen_size();
    let mut img = try_create_image!(img_size, ());
    img.fill(0, 0, 0, 0);

    let mut p = qt::Painter::new(&mut img);

    p.scale(sx, sy);
    if let Some(vbox) = pattern.view_box {
        let ts = utils::view_box_to_transform(vbox.rect, vbox.aspect, r.size());
        p.apply_transform(&ts.to_native());
    } else if pattern.content_units == usvg::Units::ObjectBoundingBox {
        // 'Note that this attribute has no effect if attribute `viewBox` is specified.'

        // We don't use Transform::from_bbox(bbox) because `x` and `y` should be
        // ignored for some reasons...
        p.scale(bbox.width(), bbox.height());
    }

    let mut layers = super::create_layers(img_size);
    super::render_group(pattern_node, opt, &mut crate::RenderState::Ok, &mut layers, &mut p);
    p.end();

    let img = if !opacity.is_default() {
        // If `opacity` isn't `1` then we have to make image semitransparent.
        // The only way to do this is by making a new image and rendering
        // the pattern on it with transparency.

        let mut img2 = try_create_image!(img_size, ());
        img2.fill(0, 0, 0, 0);

        let mut p2 = qt::Painter::new(&mut img2);
        p2.set_opacity(opacity.value());
        p2.draw_image(0.0, 0.0, &img);
        p2.end();

        img2
    } else {
        img
    };

    brush.set_pattern(img);

    let mut ts = usvg::Transform::default();
    ts.append(&pattern.transform);
    ts.translate(r.x(), r.y());
    ts.scale(1.0 / sx, 1.0 / sy);
    brush.set_transform(ts.to_native());
}
