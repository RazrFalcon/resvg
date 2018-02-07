// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use cairo;

// self
use tree;
use math;
use super::{
    gradient,
    pattern,
};
use super::ext::{
    ReCairoContextExt,
};


pub fn apply(
    rtree: &tree::RenderTree,
    stroke: &Option<tree::Stroke>,
    cr: &cairo::Context,
    bbox: math::Rect,
) {
    match *stroke {
        Some(ref stroke) => {
            match stroke.paint {
                tree::Paint::Color(c) => {
                    cr.set_source_color(&c, stroke.opacity);
                }
                tree::Paint::Link(id) => {
                    let node = rtree.defs_at(id);
                    match *node.value() {
                        tree::NodeKind::LinearGradient(ref lg) => {
                            gradient::prepare_linear(node, lg, stroke.opacity, bbox, cr);
                        }
                        tree::NodeKind::RadialGradient(ref rg) => {
                            gradient::prepare_radial(node, rg, stroke.opacity, bbox, cr);
                        }
                        tree::NodeKind::Pattern(ref pattern) => {
                            pattern::apply(rtree, node, pattern, bbox, cr);
                        }
                        _ => {}
                    }
                }
            }

            let linecap = match stroke.linecap {
                tree::LineCap::Butt => cairo::LineCap::Butt,
                tree::LineCap::Round => cairo::LineCap::Round,
                tree::LineCap::Square => cairo::LineCap::Square,
            };
            cr.set_line_cap(linecap);

            let linejoin = match stroke.linejoin {
                tree::LineJoin::Miter => cairo::LineJoin::Miter,
                tree::LineJoin::Round => cairo::LineJoin::Round,
                tree::LineJoin::Bevel => cairo::LineJoin::Bevel,
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
