// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use cairo::{
    self,
    MatrixTrait,
};
use usvg;

// self
use geom::*;
use traits::{
    ConvTransform,
    TransformFromBBox,
};


pub fn prepare_linear(
    node: &usvg::Node,
    g: &usvg::LinearGradient,
    opacity: usvg::Opacity,
    bbox: Rect,
    cr: &cairo::Context,
) {
    let grad = cairo::LinearGradient::new(g.x1, g.y1, g.x2, g.y2);
    prepare_base(node, &g.d, &grad, opacity, bbox);
    cr.set_source(&grad);
}

pub fn prepare_radial(
    node: &usvg::Node,
    g: &usvg::RadialGradient,
    opacity: usvg::Opacity,
    bbox: Rect,
    cr: &cairo::Context
) {
    let grad = cairo::RadialGradient::new(g.fx, g.fy, 0.0, g.cx, g.cy, g.r);
    prepare_base(node, &g.d, &grad, opacity, bbox);
    cr.set_source(&grad);
}

fn prepare_base(
    node: &usvg::Node,
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
        let m = cairo::Matrix::from_bbox(bbox);
        matrix = cairo::Matrix::multiply(&matrix, &m);
    }

    matrix.invert();
    grad.set_matrix(matrix);

    for node in node.children() {
        if let usvg::NodeKind::Stop(stop) = *node.borrow() {
            grad.add_color_stop_rgba(
                *stop.offset,
                stop.color.red as f64 / 255.0,
                stop.color.green as f64 / 255.0,
                stop.color.blue as f64 / 255.0,
                (*stop.opacity) * (*opacity),
            );
        }
    }
}
