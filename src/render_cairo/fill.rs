// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use cairo;

use dom;
use math;

use super::{
    gradient,
    pattern,
};

use super::ext::{
    ReCairoContextExt,
};


pub fn apply(
    doc: &dom::Document,
    fill: &Option<dom::Fill>,
    cr: &cairo::Context,
    bbox: &math::Rect,
) {
    match *fill {
        Some(ref fill) => {
            match fill.paint {
                dom::Paint::Color(c) => {
                    cr.set_source_color(&c, fill.opacity);
                }
                dom::Paint::Link(id) => {
                    let ref_elem = doc.get_defs(id);

                    match ref_elem.kind {
                        dom::RefElementKind::LinearGradient(ref lg) =>
                            gradient::prepare_linear(lg, fill.opacity, bbox, cr),
                        dom::RefElementKind::RadialGradient(ref rg) =>
                            gradient::prepare_radial(rg, fill.opacity, bbox, cr),
                        dom::RefElementKind::ClipPath(_) => {}
                        dom::RefElementKind::Pattern(ref pattern) => {
                            pattern::apply(doc, pattern, bbox, cr);
                        }
                    }
                }
            }

            match fill.rule {
                dom::FillRule::NonZero => cr.set_fill_rule(cairo::FillRule::Winding),
                dom::FillRule::EvenOdd => cr.set_fill_rule(cairo::FillRule::EvenOdd),
            }
        }
        None => {
            // reset fill properties
            cr.reset_source_rgba();
            cr.set_fill_rule(cairo::FillRule::Winding);
        }
    }
}
