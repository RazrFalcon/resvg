// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use qt;
use usvg::tree::prelude::*;

// self
use traits::{
    ConvTransform,
};


pub fn prepare_linear(
    node: &tree::Node,
    g: &tree::LinearGradient,
    opacity: tree::Opacity,
    brush: &mut qt::Brush,
) {
    let mut grad = qt::LinearGradient::new(g.x1, g.y1, g.x2, g.y2);
    prepare_base(node, &g.d, &mut grad, opacity);

    brush.set_linear_gradient(grad);
    brush.set_transform(g.d.transform.to_native());
}

pub fn prepare_radial(
    node: &tree::Node,
    g: &tree::RadialGradient,
    opacity: tree::Opacity,
    brush: &mut qt::Brush,
) {
    let mut grad = qt::RadialGradient::new(g.cx, g.cy, g.fx, g.fy, g.r);
    prepare_base(node, &g.d, &mut grad, opacity);

    brush.set_radial_gradient(grad);
    brush.set_transform(g.d.transform.to_native());
}

fn prepare_base(
    node: &tree::Node,
    g: &tree::BaseGradient,
    grad: &mut qt::Gradient,
    opacity: tree::Opacity,
) {
    let spread_method = match g.spread_method {
        tree::SpreadMethod::Pad => qt::Spread::PadSpread,
        tree::SpreadMethod::Reflect => qt::Spread::ReflectSpread,
        tree::SpreadMethod::Repeat => qt::Spread::RepeatSpread,
    };
    grad.set_spread(spread_method);

    if g.units == tree::Units::ObjectBoundingBox {
        grad.set_units(qt::CoordinateMode::ObjectBoundingMode)
    }

    for node in node.children() {
        if let tree::NodeKind::Stop(stop) = *node.kind() {
            grad.set_color_at(
                *stop.offset,
                stop.color.red,
                stop.color.green,
                stop.color.blue,
                ((*stop.opacity * *opacity) * 255.0) as u8,
            );
        }
    }
}
