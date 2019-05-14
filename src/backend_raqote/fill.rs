// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use raqote;

// self
use super::prelude::*;
use super::{
    gradient,
//    pattern,
};


pub fn apply(
    tree: &usvg::Tree,
    path: &raqote::Path,
    fill: &Option<usvg::Fill>,
    opt: &Options,
    bbox: Rect,
    dt: &mut raqote::DrawTarget,
) {
    if let Some(ref fill) = fill {
        let source = match fill.paint {
            usvg::Paint::Color(c) => {
                let alpha = (fill.opacity.value() * 255.0) as u8;
                raqote::Source::Solid(c.to_solid(alpha))
            }
            usvg::Paint::Link(ref id) => {
                if let Some(node) = tree.defs_by_id(id) {
                    match *node.borrow() {
                        usvg::NodeKind::LinearGradient(ref lg) => {
                            gradient::prepare_linear(lg, fill.opacity, bbox)
                        }
                        usvg::NodeKind::RadialGradient(ref rg) => {
                            gradient::prepare_radial(rg, fill.opacity, bbox)
                        }
                        usvg::NodeKind::Pattern(ref pattern) => {
//                            pattern::apply(&node, pattern, opt, fill.opacity, bbox, cr);
                            return;
                        }
                        _ => {
                            return;
                        }
                    }
                } else {
                    return;
                }
            }
        };

        dt.fill(
            path,
            &source,
            &raqote::DrawOptions::default(),
        );
    }
}
