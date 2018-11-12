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
                    if let Some(node) = tree.defs_by_id(id) {
                        match *node.borrow() {
                            usvg::NodeKind::LinearGradient(ref lg) => {
                                gradient::prepare_linear(lg, stroke.opacity, bbox, cr);
                            }
                            usvg::NodeKind::RadialGradient(ref rg) => {
                                gradient::prepare_radial(rg, stroke.opacity, bbox, cr);
                            }
                            usvg::NodeKind::Pattern(ref pattern) => {
                                pattern::apply(&node, pattern, opt, stroke.opacity, bbox, cr);
                            }
                            _ => {}
                        }
                    }
                }
            }

            let linecap = match stroke.linecap {
                usvg::LineCap::Butt => cairo::LineCap::Butt,
                usvg::LineCap::Round => cairo::LineCap::Round,
                usvg::LineCap::Square => cairo::LineCap::Square,
            };
            cr.set_line_cap(linecap);

            let linejoin = match stroke.linejoin {
                usvg::LineJoin::Miter => cairo::LineJoin::Miter,
                usvg::LineJoin::Round => cairo::LineJoin::Round,
                usvg::LineJoin::Bevel => cairo::LineJoin::Bevel,
            };
            cr.set_line_join(linejoin);

            match stroke.dasharray {
                Some(ref list) => cr.set_dash(list, stroke.dashoffset),
                None => cr.set_dash(&[], 0.0),
            }

            cr.set_miter_limit(stroke.miterlimit);
            cr.set_line_width(stroke.width.value());
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
