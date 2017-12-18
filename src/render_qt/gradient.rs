// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use qt;

use dom::{
    self,
    GradientUnits,
    SpreadMethod,
};

use super::ext::TransformToMatrix;


pub fn prepare_linear(
    g: &dom::LinearGradient,
    opacity: f64,
    brush: &mut qt::Brush,
) {
    let mut grad = qt::LinearGradient::new(g.x1, g.y1, g.x2, g.y2);
    prepare_base(&g.d, &mut grad, opacity);

    brush.set_linear_gradient(grad);
    brush.set_transform(g.d.transform.to_qtransform());
}

pub fn prepare_radial(
    g: &dom::RadialGradient,
    opacity: f64,
    brush: &mut qt::Brush,
) {
    let mut grad = qt::RadialGradient::new(g.cx, g.cy, g.fx, g.fy, g.r);
    prepare_base(&g.d, &mut grad, opacity);

    brush.set_radial_gradient(grad);
    brush.set_transform(g.d.transform.to_qtransform());
}

fn prepare_base(g: &dom::BaseGradient, grad: &mut qt::Gradient, opacity: f64) {
    let spread_method = match g.spread_method {
        SpreadMethod::Pad => qt::Spread::PadSpread,
        SpreadMethod::Reflect => qt::Spread::ReflectSpread,
        SpreadMethod::Repeat => qt::Spread::RepeatSpread,
    };
    grad.set_spread(spread_method);

    if g.units == GradientUnits::ObjectBoundingBox {
        grad.set_units(qt::CoordinateMode::ObjectBoundingMode)
    }

    for stop in &g.stops {
        grad.set_color_at(
            stop.offset,
            stop.color.red,
            stop.color.green,
            stop.color.blue,
            ((stop.opacity * opacity) * 255.0) as u8,
        );
    }
}
