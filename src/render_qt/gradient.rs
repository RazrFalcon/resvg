// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use qt;

use tree::{
    self,
    Units,
    SpreadMethod,
};

use traits::{
    ConvTransform,
};


pub fn prepare_linear(
    node: tree::DefsNodeRef,
    g: &tree::LinearGradient,
    opacity: f64,
    brush: &mut qt::Brush,
) {
    let mut grad = qt::LinearGradient::new(g.x1, g.y1, g.x2, g.y2);
    prepare_base(node, &g.d, &mut grad, opacity);

    brush.set_linear_gradient(grad);
    brush.set_transform(g.d.transform.to_native());
}

pub fn prepare_radial(
    node: tree::DefsNodeRef,
    g: &tree::RadialGradient,
    opacity: f64,
    brush: &mut qt::Brush,
) {
    let mut grad = qt::RadialGradient::new(g.cx, g.cy, g.fx, g.fy, g.r);
    prepare_base(node, &g.d, &mut grad, opacity);

    brush.set_radial_gradient(grad);
    brush.set_transform(g.d.transform.to_native());
}

fn prepare_base(
    node: tree::DefsNodeRef,
    g: &tree::BaseGradient,
    grad: &mut qt::Gradient,
    opacity: f64,
) {
    let spread_method = match g.spread_method {
        SpreadMethod::Pad => qt::Spread::PadSpread,
        SpreadMethod::Reflect => qt::Spread::ReflectSpread,
        SpreadMethod::Repeat => qt::Spread::RepeatSpread,
    };
    grad.set_spread(spread_method);

    if g.units == Units::ObjectBoundingBox {
        grad.set_units(qt::CoordinateMode::ObjectBoundingMode)
    }

    for stop in node.stops() {
        grad.set_color_at(
            stop.offset,
            stop.color.red,
            stop.color.green,
            stop.color.blue,
            ((stop.opacity * opacity) * 255.0) as u8,
        );
    }
}
