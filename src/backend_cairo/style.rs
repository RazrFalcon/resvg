// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use usvg::try_opt;

use crate::{prelude::*, backend_utils::ConvTransform};
use super::{ReCairoContextExt, FlatRender, CairoFlatRender};


pub fn fill(
    tree: &usvg::Tree,
    fill: &Option<usvg::Fill>,
    opt: &Options,
    bbox: Rect,
    cr: &cairo::Context,
) {
    match *fill {
        Some(ref fill) => {
            match fill.paint {
                usvg::Paint::Color(c) => {
                    cr.set_source_color(c, fill.opacity);
                }
                usvg::Paint::Link(ref id) => {
                    if let Some(node) = tree.defs_by_id(id) {
                        match *node.borrow() {
                            usvg::NodeKind::LinearGradient(ref lg) => {
                                prepare_linear(lg, fill.opacity, bbox, cr);
                            }
                            usvg::NodeKind::RadialGradient(ref rg) => {
                                prepare_radial(rg, fill.opacity, bbox, cr);
                            }
                            usvg::NodeKind::Pattern(ref pattern) => {
                                prepare_pattern(&node, pattern, opt, fill.opacity, bbox, cr);
                            }
                            _ => {}
                        }
                    }
                }
            }

            match fill.rule {
                usvg::FillRule::NonZero => cr.set_fill_rule(cairo::FillRule::Winding),
                usvg::FillRule::EvenOdd => cr.set_fill_rule(cairo::FillRule::EvenOdd),
            }
        }
        None => {
            cr.reset_source_rgba();
            cr.set_fill_rule(cairo::FillRule::Winding);
        }
    }
}

pub fn stroke(
    tree: &usvg::Tree,
    stroke: &Option<usvg::Stroke>,
    opt: &Options,
    bbox: Rect,
    cr: &cairo::Context,
) {
    match *stroke {
        Some(ref stroke) => {
            match stroke.paint {
                usvg::Paint::Color(c) => {
                    cr.set_source_color(c, stroke.opacity);
                }
                usvg::Paint::Link(ref id) => {
                    if let Some(node) = tree.defs_by_id(id) {
                        match *node.borrow() {
                            usvg::NodeKind::LinearGradient(ref lg) => {
                                prepare_linear(lg, stroke.opacity, bbox, cr);
                            }
                            usvg::NodeKind::RadialGradient(ref rg) => {
                                prepare_radial(rg, stroke.opacity, bbox, cr);
                            }
                            usvg::NodeKind::Pattern(ref pattern) => {
                                prepare_pattern(&node, pattern, opt, stroke.opacity, bbox, cr);
                            }
                            _ => {}
                        }
                    }
                }
            }

            let linecap = match stroke.linecap {
                usvg::LineCap::Butt => cairo::LineCap::Butt,
                usvg::LineCap::Round => cairo::LineCap::Round,
                usvg::LineCap::Square => cairo::LineCap::Square,
            };
            cr.set_line_cap(linecap);

            let linejoin = match stroke.linejoin {
                usvg::LineJoin::Miter => cairo::LineJoin::Miter,
                usvg::LineJoin::Round => cairo::LineJoin::Round,
                usvg::LineJoin::Bevel => cairo::LineJoin::Bevel,
            };
            cr.set_line_join(linejoin);

            match stroke.dasharray {
                Some(ref list) => cr.set_dash(list, stroke.dashoffset as f64),
                None => cr.set_dash(&[], 0.0),
            }

            cr.set_miter_limit(stroke.miterlimit.value());
            cr.set_line_width(stroke.width.value());
        }
        None => {
            // reset stroke properties
            cr.reset_source_rgba();
            cr.set_line_cap(cairo::LineCap::Butt);
            cr.set_line_join(cairo::LineJoin::Miter);
            cr.set_miter_limit(4.0);
            cr.set_line_width(1.0);
            cr.set_dash(&[], 0.0);
        }
    }
}

fn prepare_linear(
    g: &usvg::LinearGradient,
    opacity: usvg::Opacity,
    bbox: Rect,
    cr: &cairo::Context,
) {
    let grad = cairo::LinearGradient::new(g.x1, g.y1, g.x2, g.y2);
    prepare_base_gradient(&g.base, &grad, opacity, bbox);
    cr.set_source(&grad);
}

fn prepare_radial(
    g: &usvg::RadialGradient,
    opacity: usvg::Opacity,
    bbox: Rect,
    cr: &cairo::Context,
) {
    let grad = cairo::RadialGradient::new(g.fx, g.fy, 0.0, g.cx, g.cy, g.r.value());
    prepare_base_gradient(&g.base, &grad, opacity, bbox);
    cr.set_source(&grad);
}

fn prepare_base_gradient(
    g: &usvg::BaseGradient,
    grad: &cairo::Gradient,
    opacity: usvg::Opacity,
    bbox: Rect,
) {
    let spread_method = match g.spread_method {
        usvg::SpreadMethod::Pad => cairo::Extend::Pad,
        usvg::SpreadMethod::Reflect => cairo::Extend::Reflect,
        usvg::SpreadMethod::Repeat => cairo::Extend::Repeat,
    };
    grad.set_extend(spread_method);

    let mut matrix = g.transform.to_native();

    if g.units == usvg::Units::ObjectBoundingBox {
        let m = usvg::Transform::from_bbox(bbox).to_native();
        matrix = cairo::Matrix::multiply(&matrix, &m);
    }

    matrix.invert();
    grad.set_matrix(matrix);

    for stop in &g.stops {
        grad.add_color_stop_rgba(
            stop.offset.value(),
            stop.color.red as f64 / 255.0,
            stop.color.green as f64 / 255.0,
            stop.color.blue as f64 / 255.0,
            stop.opacity.value() * opacity.value(),
        );
    }
}

fn prepare_pattern(
    node: &usvg::Node,
    pattern: &usvg::Pattern,
    opt: &Options,
    opacity: usvg::Opacity,
    bbox: Rect,
    cr: &cairo::Context,
) {
    let r = if pattern.units == usvg::Units::ObjectBoundingBox {
        pattern.rect.bbox_transform(bbox)
    } else {
        pattern.rect
    };

    let global_ts = usvg::Transform::from_native(&cr.get_matrix());
    let (sx, sy) = global_ts.get_scale();

    let img_size = try_opt!(Size::new(r.width() * sx, r.height() * sy)).to_screen_size();
    let surface = try_create_surface!(img_size, ());

    let sub_cr = cairo::Context::new(&surface);
    sub_cr.transform(cairo::Matrix::new(sx, 0.0, 0.0, sy, 0.0, 0.0));

    if let Some(vbox) = pattern.view_box {
        let ts = utils::view_box_to_transform(vbox.rect, vbox.aspect, r.size());
        sub_cr.transform(ts.to_native());
    } else if pattern.content_units == usvg::Units::ObjectBoundingBox {
        // 'Note that this attribute has no effect if attribute `viewBox` is specified.'

        // We don't use Transform::from_bbox(bbox) because `x` and `y` should be
        // ignored for some reasons...
        sub_cr.scale(bbox.width(), bbox.height());
    }

    let ref tree = node.tree();
    let mut render = CairoFlatRender::new(tree, opt, img_size, &sub_cr);
    render.render_group(node);

    let mut ts = usvg::Transform::default();
    ts.append(&pattern.transform);
    ts.translate(r.x(), r.y());
    ts.scale(1.0 / sx, 1.0 / sy);


    let surface = if !opacity.is_default() {
        // If `opacity` isn't `1` then we have to make image semitransparent.
        // The only way to do this is by making a new image and rendering
        // the pattern on it with transparency.

        let surface2 = try_create_surface!(img_size, ());
        let sub_cr2 = cairo::Context::new(&surface2);
        sub_cr2.set_source_surface(&surface, 0.0, 0.0);
        sub_cr2.paint_with_alpha(opacity.value());

        surface2
    } else {
        surface
    };


    let patt = cairo::SurfacePattern::create(&surface);
    patt.set_extend(cairo::Extend::Repeat);
    patt.set_filter(cairo::Filter::Best);

    let mut m: cairo::Matrix = ts.to_native();
    m.invert();
    patt.set_matrix(m);

    cr.set_source(&patt);
}
