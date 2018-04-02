// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use qt;
use usvg::tree::prelude::*;

// self
use geom::*;
use super::{
    gradient,
    pattern,
};
use {
    Options,
};


pub fn apply(
    tree: &tree::Tree,
    fill: &Option<tree::Fill>,
    opt: &Options,
    bbox: Rect,
    p: &qt::Painter,
) {
    match *fill {
        Some(ref fill) => {
            let mut brush = qt::Brush::new();
            let opacity = fill.opacity;

            match fill.paint {
                tree::Paint::Color(c) => {
                    // a-fill-opacity-001.svg
                    let a = f64_bound(0.0, *opacity * 255.0, 255.0) as u8;
                    brush.set_color(c.red, c.green, c.blue, a);
                }
                tree::Paint::Link(ref id) => {
                    // a-fill-opacity-003.svg
                    // a-fill-opacity-004.svg
                    if let Some(node) = tree.defs_by_id(id) {
                        match *node.kind() {
                            tree::NodeKind::LinearGradient(ref lg) => {
                                gradient::prepare_linear(&node, lg, opacity, &mut brush);
                            }
                            tree::NodeKind::RadialGradient(ref rg) => {
                                gradient::prepare_radial(&node, rg, opacity, &mut brush);
                            }
                            tree::NodeKind::Pattern(ref pattern) => {
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
