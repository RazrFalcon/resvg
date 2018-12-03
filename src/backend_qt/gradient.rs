// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use qt;
use usvg;

// self
use super::prelude::*;


pub fn prepare_linear(
    g: &usvg::LinearGradient,
    opacity: usvg::Opacity,
    bbox: Rect,
    brush: &mut qt::Brush,
) {
    let mut grad = qt::LinearGradient::new(g.x1, g.y1, g.x2, g.y2);
    prepare_base(&g.base, opacity, &mut grad);

    brush.set_linear_gradient(grad);
    apply_ts(&g.base, bbox, brush);
}

pub fn prepare_radial(
    g: &usvg::RadialGradient,
    opacity: usvg::Opacity,
    bbox: Rect,
    brush: &mut qt::Brush,
) {
    let mut grad = qt::RadialGradient::new(g.cx, g.cy, g.fx, g.fy, g.r);
    prepare_base(&g.base, opacity, &mut grad);

    brush.set_radial_gradient(grad);
    apply_ts(&g.base, bbox, brush);
}

fn prepare_base(
    g: &usvg::BaseGradient,
    opacity: usvg::Opacity,
    grad: &mut qt::Gradient,
) {
    let spread_method = match g.spread_method {
        usvg::SpreadMethod::Pad => qt::Spread::Pad,
        usvg::SpreadMethod::Reflect => qt::Spread::Reflect,
        usvg::SpreadMethod::Repeat => qt::Spread::Repeat,
    };
    grad.set_spread(spread_method);

    for stop in &g.stops {
        grad.set_color_at(
            *stop.offset,
            stop.color.red,
            stop.color.green,
            stop.color.blue,
            ((*stop.opacity * *opacity) * 255.0) as u8,
        );
    }
}

fn apply_ts(
    g: &usvg::BaseGradient,
    bbox: Rect,
    brush: &mut qt::Brush,
) {
    // We doesn't use `QGradient::setCoordinateMode` because it works incorrectly.
    // https://bugreports.qt.io/browse/QTBUG-67995

    if g.units == usvg::Units::ObjectBoundingBox {
        let mut ts = usvg::Transform::from_bbox(bbox);
        ts.append(&g.transform);
        brush.set_transform(ts.to_native());
    } else {
        brush.set_transform(g.transform.to_native());
    }
}
