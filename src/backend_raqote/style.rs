// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{prelude::*, ConvTransform, RenderState};
use super::{ColorExt, RaqoteDrawTargetExt};


pub fn fill(
    tree: &usvg::Tree,
    path: &raqote::Path,
    fill: &Option<usvg::Fill>,
    opt: &Options,
    bbox: Rect,
    draw_opt: &raqote::DrawOptions,
    dt: &mut raqote::DrawTarget,
) {
    if let Some(ref fill) = fill {
        let patt_dt;
        let source = match fill.paint {
            usvg::Paint::Color(c) => {
                let alpha = (fill.opacity.value() * 255.0) as u8;
                raqote::Source::Solid(c.to_solid(alpha))
            }
            usvg::Paint::Link(ref id) => {
                if let Some(node) = tree.defs_by_id(id) {
                    match *node.borrow() {
                        usvg::NodeKind::LinearGradient(ref lg) => {
                            prepare_linear(lg, fill.opacity, bbox)
                        }
                        usvg::NodeKind::RadialGradient(ref rg) => {
                            prepare_radial(rg, fill.opacity, bbox)
                        }
                        usvg::NodeKind::Pattern(ref pattern) => {
                            let ts = *dt.get_transform();
                            let (sub_dt, patt_ts) = try_opt!(
                                prepare_pattern(&node, pattern, opt, ts, bbox, fill.opacity)
                            );
                            patt_dt = sub_dt;
                            create_pattern_image(&patt_dt, patt_ts)
                        }
                        _ => {
                            return;
                        }
                    }
                } else {
                    return;
                }
            }
        };

        dt.fill(
            path,
            &source,
            draw_opt,
        );
    }
}

pub fn stroke(
    tree: &usvg::Tree,
    path: &raqote::Path,
    stroke: &Option<usvg::Stroke>,
    opt: &Options,
    bbox: Rect,
    draw_opt: &raqote::DrawOptions,
    dt: &mut raqote::DrawTarget,
) {
    if let Some(ref stroke) = stroke {
        let cap = match stroke.linecap {
            usvg::LineCap::Butt => raqote::LineCap::Butt,
            usvg::LineCap::Round => raqote::LineCap::Round,
            usvg::LineCap::Square => raqote::LineCap::Square,
        };

        let join = match stroke.linejoin {
            usvg::LineJoin::Miter => raqote::LineJoin::Miter,
            usvg::LineJoin::Round => raqote::LineJoin::Round,
            usvg::LineJoin::Bevel => raqote::LineJoin::Bevel,
        };

        let mut dash_array = Vec::new();
        if let Some(ref list) = stroke.dasharray {
            dash_array = list.iter().map(|n| *n as f32).collect();
        }

        let style = raqote::StrokeStyle {
            cap,
            join,
            width: stroke.width.value() as f32,
            miter_limit: stroke.miterlimit.value() as f32,
            dash_array,
            dash_offset: stroke.dashoffset,
        };

        let patt_dt;
        let source = match stroke.paint {
            usvg::Paint::Color(c) => {
                let alpha = (stroke.opacity.value() * 255.0) as u8;
                raqote::Source::Solid(c.to_solid(alpha))
            }
            usvg::Paint::Link(ref id) => {
                if let Some(node) = tree.defs_by_id(id) {
                    match *node.borrow() {
                        usvg::NodeKind::LinearGradient(ref lg) => {
                            prepare_linear(lg, stroke.opacity, bbox)
                        }
                        usvg::NodeKind::RadialGradient(ref rg) => {
                            prepare_radial(rg, stroke.opacity, bbox)
                        }
                        usvg::NodeKind::Pattern(ref pattern) => {
                            let ts = *dt.get_transform();
                            let (sub_dt, patt_ts) = try_opt!(
                                prepare_pattern(&node, pattern, opt, ts, bbox, stroke.opacity)
                            );
                            patt_dt = sub_dt;
                            create_pattern_image(&patt_dt, patt_ts)
                        }
                        _ => {
                            return;
                        }
                    }
                } else {
                    return;
                }
            }
        };

        dt.stroke(
            &path,
            &source,
            &style,
            draw_opt,
        );
    }
}

fn prepare_linear<'a>(
    g: &usvg::LinearGradient,
    opacity: usvg::Opacity,
    bbox: Rect,
) -> raqote::Source<'a> {
    let ts = if g.units == usvg::Units::ObjectBoundingBox {
        let mut ts = usvg::Transform::from_bbox(bbox);
        ts.append(&g.transform);
        ts
    } else {
        g.transform
    };

    let mut grad = raqote::Source::new_linear_gradient(
        raqote::Gradient { stops: conv_stops(g, opacity) },
        raqote::Point::new(g.x1 as f32, g.y1 as f32),
        raqote::Point::new(g.x2 as f32, g.y2 as f32),
        conv_spread(g.base.spread_method),
    );

    if let raqote::Source::LinearGradient(_, _, ref mut transform) = grad {
        let ts: raqote::Transform = ts.to_native();
        if let Some(ts) = ts.inverse() {
            *transform = transform.pre_transform(&ts);
        }
    }

    grad
}

fn prepare_radial<'a>(
    g: &usvg::RadialGradient,
    opacity: usvg::Opacity,
    bbox: Rect,
) -> raqote::Source<'a> {
    let ts = if g.units == usvg::Units::ObjectBoundingBox {
        let mut ts = usvg::Transform::from_bbox(bbox);
        ts.append(&g.transform);
        ts
    } else {
        g.transform
    };

    let mut grad = if g.fx == g.cx && g.fy == g.cy {
        raqote::Source::new_radial_gradient(
            raqote::Gradient { stops: conv_stops(g, opacity) },
            raqote::Point::new(g.cx as f32, g.cy as f32),
            g.r.value() as f32,
            conv_spread(g.base.spread_method),
        )
    } else {
        raqote::Source::new_two_circle_radial_gradient(
            raqote::Gradient { stops: conv_stops(g, opacity) },
            raqote::Point::new(g.fx as f32, g.fy as f32),
            0.0,
            raqote::Point::new(g.cx as f32, g.cy as f32),
            g.r.value() as f32,
            conv_spread(g.base.spread_method),
        )
    };

    match grad {
          raqote::Source::RadialGradient(_, _, ref mut transform)
        | raqote::Source::TwoCircleRadialGradient(_, _, _, _, _, _, ref mut transform) => {
            let ts: raqote::Transform = ts.to_native();
            if let Some(ts) = ts.inverse() {
                *transform = transform.pre_transform(&ts);
            }
        }
        _ => {}
    }

    grad
}

fn conv_spread(v: usvg::SpreadMethod) -> raqote::Spread {
    match v {
        usvg::SpreadMethod::Pad => raqote::Spread::Pad,
        usvg::SpreadMethod::Reflect => raqote::Spread::Reflect,
        usvg::SpreadMethod::Repeat => raqote::Spread::Repeat,
    }
}

fn conv_stops(
    g: &usvg::BaseGradient,
    opacity: usvg::Opacity,
) -> Vec<raqote::GradientStop> {
    let mut stops = Vec::new();

    for stop in &g.stops {
        let alpha = stop.opacity.value() * opacity.value();
        stops.push(raqote::GradientStop {
            position: stop.offset.value() as f32,
            color: stop.color.to_color((alpha * 255.0) as u8),
        });
    }

    stops
}

fn prepare_pattern<'a>(
    pattern_node: &usvg::Node,
    pattern: &usvg::Pattern,
    opt: &Options,
    global_ts: raqote::Transform,
    bbox: Rect,
    opacity: usvg::Opacity,
) -> Option<(raqote::DrawTarget, usvg::Transform)> {
    let r = if pattern.units == usvg::Units::ObjectBoundingBox {
        pattern.rect.bbox_transform(bbox)
    } else {
        pattern.rect
    };

    let global_ts = usvg::Transform::from_native(&global_ts);
    let (sx, sy) = global_ts.get_scale();

    let img_size = Size::new(r.width() * sx, r.height() * sy)?.to_screen_size();
    let mut dt = raqote::DrawTarget::new(img_size.width() as i32, img_size.height() as i32);

    dt.transform(&raqote::Transform::create_scale(sx as f32, sy as f32));
    if let Some(vbox) = pattern.view_box {
        let ts = utils::view_box_to_transform(vbox.rect, vbox.aspect, r.size());
        dt.transform(&ts.to_native());
    } else if pattern.content_units == usvg::Units::ObjectBoundingBox {
        // 'Note that this attribute has no effect if attribute `viewBox` is specified.'

        // We don't use Transform::from_bbox(bbox) because `x` and `y` should be
        // ignored for some reasons...
        dt.transform(&raqote::Transform::create_scale(bbox.width() as f32, bbox.height() as f32));
    }

    let mut layers = super::create_layers(img_size);
    super::render_group(pattern_node, opt, &mut RenderState::Ok, &mut layers, &mut dt);

    let img = if !opacity.is_default() {
        // If `opacity` isn't `1` then we have to make image semitransparent.
        // The only way to do this is by making a new image and rendering
        // the pattern on it with transparency.

        let mut img2 = raqote::DrawTarget::new(img_size.width() as i32, img_size.height() as i32);
        img2.draw_image_at(0.0, 0.0, &dt.as_image(), &raqote::DrawOptions {
            blend_mode: raqote::BlendMode::Src,
            alpha: opacity.value() as f32,
            ..raqote::DrawOptions::default()
        });

        img2
    } else {
        dt
    };

    let mut ts = usvg::Transform::default();
    ts.append(&pattern.transform);
    ts.translate(r.x(), r.y());
    ts.scale(1.0 / sx, 1.0 / sy);

    Some((img, ts))
}

fn create_pattern_image(
    dt: &raqote::DrawTarget,
    ts: usvg::Transform,
) -> raqote::Source {
    let ts: raqote::Transform = ts.to_native();
    raqote::Source::Image(
        dt.as_image(),
        raqote::ExtendMode::Repeat,
        raqote::FilterMode::Bilinear,
        ts.inverse().unwrap(),
    )
}
