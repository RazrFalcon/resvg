// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use qt;

// self
use super::prelude::*;
use super::{
    gradient,
    pattern,
};


pub fn apply(
    tree: &usvg::Tree,
    fill: &Option<usvg::Fill>,
    opt: &Options,
    bbox: Rect,
    p: &mut qt::Painter,
) {
    match *fill {
        Some(ref fill) => {
            let mut brush = qt::Brush::new();
            let opacity = fill.opacity;

            match fill.paint {
                usvg::Paint::Color(c) => {
                    let a = f64_bound(0.0, *opacity * 255.0, 255.0) as u8;
                    brush.set_color(c.red, c.green, c.blue, a);
                }
                usvg::Paint::Link(ref id) => {
                    if let Some(node) = tree.defs_by_id(id) {
                        match *node.borrow() {
                            usvg::NodeKind::LinearGradient(ref lg) => {
                                gradient::prepare_linear(lg, opacity, bbox, &mut brush);
                            }
                            usvg::NodeKind::RadialGradient(ref rg) => {
                                gradient::prepare_radial(rg, opacity, bbox, &mut brush);
                            }
                            usvg::NodeKind::Pattern(ref pattern) => {
                                let ts = p.get_transform();
                                pattern::apply(&node, pattern, opt, ts, bbox, opacity, &mut brush);
                            }
                            _ => {}
                        }
                    }
                }
            }

            p.set_brush(brush);
        }
        None => {
            p.reset_brush();
        }
    }
}
