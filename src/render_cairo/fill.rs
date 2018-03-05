// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use cairo;

// self
use tree::{
    self,
    TreeExt,
};
use math;
use super::{
    gradient,
    pattern,
};
use super::ext::*;
use {
    Options,
};


pub fn apply(
    rtree: &tree::RenderTree,
    fill: &Option<tree::Fill>,
    opt: &Options,
    bbox: math::Rect,
    cr: &cairo::Context,
) {
    match *fill {
        Some(ref fill) => {
            match fill.paint {
                tree::Paint::Color(c) => {
                    cr.set_source_color(&c, fill.opacity);
                }
                tree::Paint::Link(id) => {
                    if let Some(node) = rtree.defs_at(id) {
                        match *node.value() {
                            tree::NodeKind::LinearGradient(ref lg) => {
                                gradient::prepare_linear(node, lg, fill.opacity, bbox, cr);
                            }
                            tree::NodeKind::RadialGradient(ref rg) => {
                                gradient::prepare_radial(node, rg, fill.opacity, bbox, cr);
                            }
                            tree::NodeKind::Pattern(ref pattern) => {
                                pattern::apply(node, pattern, opt, fill.opacity, bbox, cr);
                            }
                            _ => {}
                        }
                    }
                }
            }

            match fill.rule {
                tree::FillRule::NonZero => cr.set_fill_rule(cairo::FillRule::Winding),
                tree::FillRule::EvenOdd => cr.set_fill_rule(cairo::FillRule::EvenOdd),
            }
        }
        None => {
            // reset fill properties
            cr.reset_source_rgba();
            cr.set_fill_rule(cairo::FillRule::Winding);
        }
    }
}
