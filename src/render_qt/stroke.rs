// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use qt;

// self
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
    rtree: &tree::RenderTree,
    stroke: &Option<tree::Stroke>,
    p: &qt::Painter,
    bbox: Rect,
) {
    match *stroke {
        Some(ref stroke) => {
            let mut pen = qt::Pen::new();

            match stroke.paint {
                tree::Paint::Color(c) => {
                    let a = math::f64_bound(0.0, stroke.opacity * 255.0, 255.0) as u8;
                    pen.set_color(c.red, c.green, c.blue, a);
                }
                tree::Paint::Link(id) => {
                    let node = rtree.defs_at(id);
                    let mut brush = qt::Brush::new();

                    match node.kind() {
                        tree::DefsNodeKindRef::LinearGradient(ref lg) =>
                            gradient::prepare_linear(node, lg, stroke.opacity, &mut brush),
                        tree::DefsNodeKindRef::RadialGradient(ref rg) =>
                            gradient::prepare_radial(node, rg, stroke.opacity, &mut brush),
                        tree::DefsNodeKindRef::ClipPath(_) => {}
                        tree::DefsNodeKindRef::Pattern(ref pattern) => {
                            pattern::apply(
                                rtree,
                                p.get_transform(),
                                bbox,
                                node,
                                pattern,
                                &mut brush,
                            );
                        }
                    }

                    pen.set_brush(brush);
                }
            }

            let linecap = match stroke.linecap {
                tree::LineCap::Butt => qt::LineCap::FlatCap,
                tree::LineCap::Round => qt::LineCap::RoundCap,
                tree::LineCap::Square => qt::LineCap::SquareCap,
            };
            pen.set_line_cap(linecap);

            let linejoin = match stroke.linejoin {
                tree::LineJoin::Miter => qt::LineJoin::MiterJoin,
                tree::LineJoin::Round => qt::LineJoin::RoundJoin,
                tree::LineJoin::Bevel => qt::LineJoin::BevelJoin,
            };
            pen.set_line_join(linejoin);

            pen.set_miter_limit(stroke.miterlimit);
            pen.set_width(stroke.width);

            if let Some(ref list ) = stroke.dasharray {
                pen.set_dash_array(list);
                pen.set_dash_offset(stroke.dashoffset);
            }

            p.set_pen(pen);
        }
        None => {
            p.reset_pen();
        }
    }
}
