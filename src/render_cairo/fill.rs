// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use cairo;

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
    doc: &tree::RenderTree,
    fill: &Option<tree::Fill>,
    cr: &cairo::Context,
    bbox: &math::Rect,
) {
    match *fill {
        Some(ref fill) => {
            match fill.paint {
                tree::Paint::Color(c) => {
                    cr.set_source_color(&c, fill.opacity);
                }
                tree::Paint::Link(id) => {
                    let node = doc.defs_at(id);
                    match node.kind() {
                        tree::DefsNodeKindRef::LinearGradient(ref lg) =>
                            gradient::prepare_linear(node, lg, fill.opacity, bbox, cr),
                        tree::DefsNodeKindRef::RadialGradient(ref rg) =>
                            gradient::prepare_radial(node, rg, fill.opacity, bbox, cr),
                        tree::DefsNodeKindRef::ClipPath(_) => {}
                        tree::DefsNodeKindRef::Pattern(ref pattern) => {
                            pattern::apply(doc, node, pattern, bbox, cr);
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
