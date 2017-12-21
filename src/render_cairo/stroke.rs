// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use cairo;

use dom;
use math;

use super::{
    gradient,
    ReCairoContextExt
};


pub fn apply(
    doc: &dom::Document,
    stroke: &Option<dom::Stroke>,
    cr: &cairo::Context,
    bbox: &math::Rect,
) {
    match *stroke {
        Some(ref stroke) => {
            match stroke.paint {
                dom::Paint::Color(c) => {
                    cr.set_source_color(&c, stroke.opacity);
                }
                dom::Paint::Link(id) => {
                    let ref_elem = doc.get_defs(id);

                    match ref_elem.kind {
                        dom::RefElementKind::LinearGradient(ref lg) =>
                            gradient::prepare_linear(lg, stroke.opacity, bbox, cr),
                        dom::RefElementKind::RadialGradient(ref rg) =>
                            gradient::prepare_radial(rg, stroke.opacity, bbox, cr),
                    }
                }
            }

            let linecap = match stroke.linecap {
                dom::LineCap::Butt => cairo::LineCap::Butt,
                dom::LineCap::Round => cairo::LineCap::Round,
                dom::LineCap::Square => cairo::LineCap::Square,
            };
            cr.set_line_cap(linecap);

            let linejoin = match stroke.linejoin {
                dom::LineJoin::Miter => cairo::LineJoin::Miter,
                dom::LineJoin::Round => cairo::LineJoin::Round,
                dom::LineJoin::Bevel => cairo::LineJoin::Bevel,
            };
            cr.set_line_join(linejoin);

            match stroke.dasharray {
                Some(ref list) => cr.set_dash(list, stroke.dashoffset),
                None => cr.set_dash(&[], 0.0),
            }

            cr.set_miter_limit(stroke.miterlimit);
            cr.set_line_width(stroke.width);
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
