// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use raqote;

// self
use super::prelude::*;


pub fn prepare_linear<'a>(
    g: &usvg::LinearGradient,
    opacity: usvg::Opacity,
    bbox: Rect,
) -> raqote::Source<'a> {
    raqote::Source::LinearGradient(
        raqote::Gradient { stops: conv_stops(g, opacity) },
        g.base.transform.to_native(),
    )
}

pub fn prepare_radial<'a>(
    g: &usvg::RadialGradient,
    opacity: usvg::Opacity,
    bbox: Rect,
) -> raqote::Source<'a> {
    raqote::Source::RadialGradient(
        raqote::Gradient { stops: conv_stops(g, opacity) },
        g.base.transform.to_native(),
    )
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
            color: stop.color.to_u32(255),
        });
    }

    stops
}
