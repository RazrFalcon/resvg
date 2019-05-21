// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use raqote;

// self
use super::prelude::*;


pub fn fill(
    tree: &usvg::Tree,
    path: &raqote::Path,
    fill: &Option<usvg::Fill>,
    opt: &Options,
    bbox: Rect,
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
                                prepare_pattern(&node, pattern, opt, ts, bbox, fill.opacity),
                                ()
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
            &raqote::DrawOptions::default(),
        );
    }
}

pub fn stroke(
    tree: &usvg::Tree,
    path: &raqote::Path,
    stroke: &Option<usvg::Stroke>,
    opt: &Options,
    bbox: Rect,
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
                                prepare_pattern(&node, pattern, opt, ts, bbox, stroke.opacity),
                                ()
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
            &raqote::DrawOptions::default(),
        );
    }
}

fn prepare_linear<'a>(
    g: &usvg::LinearGradient,
    opacity: usvg::Opacity,
    bbox: Rect,
) -> raqote::Source<'a> {
    let mut ts = if g.units == usvg::Units::ObjectBoundingBox {
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
        transform.post_mul(&ts.to_native());
    }

    grad
}

fn prepare_radial<'a>(
    g: &usvg::RadialGradient,
    opacity: usvg::Opacity,
    bbox: Rect,
) -> raqote::Source<'a> {
    raqote::Source::RadialGradient(
        raqote::Gradient { stops: conv_stops(g, opacity) },
        conv_spread(g.base.spread_method),
        g.base.transform.to_native(),
    )
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
            color: stop.color.to_u32((alpha * 255.0) as u8),
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

    let mut layers = super::create_layers(img_size, opt);
    super::render_group(pattern_node, opt, &mut layers, &mut dt);

//    let img = if !opacity.is_default() {
//        // If `opacity` isn't `1` then we have to make image semitransparent.
//        // The only way to do this is by making a new image and rendering
//        // the pattern on it with transparency.
//
//        let mut img2 = try_create_image!(img_size, ());
//        img2.fill(0, 0, 0, 0);
//
//        let mut p2 = qt::Painter::new(&mut img2);
//        p2.set_opacity(opacity.value());
//        p2.draw_image(0.0, 0.0, &img);
//        p2.end();
//
//        img2
//    } else {
//        img
//    };

    let mut ts = usvg::Transform::default();
    ts.append(&pattern.transform);
    ts.translate(r.x(), r.y());
    ts.scale(1.0 / sx, 1.0 / sy);

    Some((dt, ts))
}

fn create_pattern_image(
    dt: &raqote::DrawTarget,
    ts: usvg::Transform,
) -> raqote::Source {
    let img = raqote::Image {
        width: dt.width(),
        height: dt.height(),
        data: dt.get_data(),
    };

    raqote::Source::Image(
        img,
        raqote::ExtendMode::Repeat,
        ts.to_native(),
    )
}
