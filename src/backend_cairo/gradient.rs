// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use cairo::{
    self,
    MatrixTrait,
};

// self
use super::prelude::*;


pub fn prepare_linear(
    g: &usvg::LinearGradient,
    opacity: usvg::Opacity,
    bbox: Rect,
    cr: &cairo::Context,
) {
    let grad = cairo::LinearGradient::new(g.x1, g.y1, g.x2, g.y2);
    prepare_base(&g.base, &grad, opacity, bbox, &g.id);
    cr.set_source(&cairo::Pattern::LinearGradient(grad));
}

pub fn prepare_radial(
    g: &usvg::RadialGradient,
    opacity: usvg::Opacity,
    bbox: Rect,
    cr: &cairo::Context
) {
    let grad = cairo::RadialGradient::new(g.fx, g.fy, 0.0, g.cx, g.cy, g.r.value());
    prepare_base(&g.base, &grad, opacity, bbox, &g.id);
    cr.set_source(&cairo::Pattern::RadialGradient(grad));
}

fn prepare_base<G>(
    g: &usvg::BaseGradient,
    grad: &G,
    opacity: usvg::Opacity,
    bbox: Rect,
    id: &str,
) where G: cairo::Gradient {
    let spread_method = match g.spread_method {
        usvg::SpreadMethod::Pad => cairo::Extend::Pad,
        usvg::SpreadMethod::Reflect => cairo::Extend::Reflect,
        usvg::SpreadMethod::Repeat => cairo::Extend::Repeat,
    };
    grad.set_extend(spread_method);

    let mut matrix = g.transform.to_native();

    if g.units == usvg::Units::ObjectBoundingBox {
        let m = try_opt_warn!(cairo::Matrix::from_bbox(bbox), (),
                              "Gradient '{}' cannot be used on a zero-sized object.", id);
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
