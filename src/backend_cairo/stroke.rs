// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use cairo;
use usvg;

// self
use super::prelude::*;
use super::{
    gradient,
    pattern,
};


pub fn apply(
    tree: &usvg::Tree,
    stroke: &Option<usvg::Stroke>,
    opt: &Options,
    bbox: Rect,
    cr: &cairo::Context,
) {
    match *stroke {
        Some(ref stroke) => {
            match stroke.paint {
                usvg::Paint::Color(c) => {
                    cr.set_source_color(&c, stroke.opacity);
                }
                usvg::Paint::Link(ref id) => {
                    // a-stroke-002.svg
                    // a-stroke-003.svg
                    // a-stroke-004.svg
                    if let Some(node) = tree.defs_by_id(id) {
                        match *node.borrow() {
                            usvg::NodeKind::LinearGradient(ref lg) => {
                                gradient::prepare_linear(&node, lg, stroke.opacity, bbox, cr);
                            }
                            usvg::NodeKind::RadialGradient(ref rg) => {
                                gradient::prepare_radial(&node, rg, stroke.opacity, bbox, cr);
                            }
                            usvg::NodeKind::Pattern(ref pattern) => {
                                pattern::apply(&node, pattern, opt, stroke.opacity, bbox, cr);
                            }
                            _ => {}
                        }
                    }
                }
            }

            // a-stroke-linecap-001.svg
            // a-stroke-linecap-002.svg
            // a-stroke-linecap-003.svg
            let linecap = match stroke.linecap {
                usvg::LineCap::Butt => cairo::LineCap::Butt,
                usvg::LineCap::Round => cairo::LineCap::Round,
                usvg::LineCap::Square => cairo::LineCap::Square,
            };
            cr.set_line_cap(linecap);

            // a-stroke-linejoin-001.svg
            // a-stroke-linejoin-002.svg
            // a-stroke-linejoin-003.svg
            let linejoin = match stroke.linejoin {
                usvg::LineJoin::Miter => cairo::LineJoin::Miter,
                usvg::LineJoin::Round => cairo::LineJoin::Round,
                usvg::LineJoin::Bevel => cairo::LineJoin::Bevel,
            };
            cr.set_line_join(linejoin);

            // a-stroke-dasharray-001.svg
            // a-stroke-dasharray-002.svg
            // a-stroke-dashoffset-001.svg
            // a-stroke-dashoffset-002.svg
            // a-stroke-dashoffset-006.svg
            match stroke.dasharray {
                Some(ref list) => cr.set_dash(list, stroke.dashoffset),
                None => cr.set_dash(&[], 0.0),
            }

            // a-stroke-miterlimit-002.svg
            cr.set_miter_limit(stroke.miterlimit);
            cr.set_line_width(stroke.width);
        }
        None => {
            // a-stroke-006.svg

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
