// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use qt;

use dom;
use math;

use super::{
    gradient,
};


pub fn apply(
    doc: &dom::Document,
    stroke: &Option<dom::Stroke>,
    p: &qt::Painter,
) {
    match *stroke {
        Some(ref stroke) => {
            let mut pen = qt::Pen::new();

            match stroke.paint {
                dom::Paint::Color(c) => {
                    let a = math::f64_bound(0.0, stroke.opacity * 255.0, 255.0) as u8;
                    pen.set_color(c.red, c.green, c.blue, a);
                }
                dom::Paint::Link(id) => {
                    let ref_elem = doc.get_defs(id);
                    let mut brush = qt::Brush::new();

                    match ref_elem.kind {
                        dom::RefElementKind::LinearGradient(ref lg) =>
                            gradient::prepare_linear(lg, stroke.opacity, &mut brush),
                        dom::RefElementKind::RadialGradient(ref rg) =>
                            gradient::prepare_radial(rg, stroke.opacity, &mut brush),
                    }

                    pen.set_brush(brush);
                }
            }

            let linecap = match stroke.linecap {
                dom::LineCap::Butt => qt::LineCap::FlatCap,
                dom::LineCap::Round => qt::LineCap::RoundCap,
                dom::LineCap::Square => qt::LineCap::SquareCap,
            };
            pen.set_line_cap(linecap);

            let linejoin = match stroke.linejoin {
                dom::LineJoin::Miter => qt::LineJoin::MiterJoin,
                dom::LineJoin::Round => qt::LineJoin::RoundJoin,
                dom::LineJoin::Bevel => qt::LineJoin::BevelJoin,
            };
            pen.set_line_join(linejoin);

            pen.set_miter_limit(stroke.miterlimit);
            pen.set_width(stroke.width);

            if let Some(ref list ) = stroke.dasharray {
                pen.set_dash_array(list);
                pen.set_dash_offset(stroke.dashoffset);
            }

            p.set_pen(pen);
        }
        None => {
            p.reset_pen();
        }
    }
}
