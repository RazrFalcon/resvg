// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use qt;

use tree;
use math::{
    self,
    Rect,
};

use super::{
    gradient,
    pattern,
};


pub fn apply(
    doc: &tree::RenderTree,
    fill: &Option<tree::Fill>,
    p: &qt::Painter,
    bbox: &Rect,
) {
    match *fill {
        Some(ref fill) => {
            let mut brush = qt::Brush::new();

            match fill.paint {
                tree::Paint::Color(c) => {
                    let a = math::f64_bound(0.0, fill.opacity * 255.0, 255.0) as u8;
                    brush.set_color(c.red, c.green, c.blue, a);
                }
                tree::Paint::Link(id) => {
                    let node = doc.defs_at(id);
                    match node.kind() {
                        tree::DefsNodeKindRef::LinearGradient(ref lg) => {
                            gradient::prepare_linear(node, lg, fill.opacity, &mut brush);
                        }
                        tree::DefsNodeKindRef::RadialGradient(ref rg) => {
                            gradient::prepare_radial(node, rg, fill.opacity, &mut brush);
                        }
                        tree::DefsNodeKindRef::ClipPath(_) => {}
                        tree::DefsNodeKindRef::Pattern(ref pattern) => {
                            pattern::apply(doc, p.get_transform(), bbox, node, pattern, &mut brush);
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
