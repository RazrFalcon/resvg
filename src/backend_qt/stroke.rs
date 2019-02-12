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
    stroke: &Option<usvg::Stroke>,
    opt: &Options,
    bbox: Rect,
    p: &mut qt::Painter,
) {
    match *stroke {
        Some(ref stroke) => {
            let mut pen = qt::Pen::new();
            let opacity = stroke.opacity;

            match stroke.paint {
                usvg::Paint::Color(c) => {
                    let a = f64_bound(0.0, opacity.value() * 255.0, 255.0) as u8;
                    pen.set_color(c.red, c.green, c.blue, a);
                }
                usvg::Paint::Link(ref id) => {
                    let mut brush = qt::Brush::new();

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

                    pen.set_brush(brush);
                }
            }

            let linecap = match stroke.linecap {
                usvg::LineCap::Butt => qt::LineCap::Flat,
                usvg::LineCap::Round => qt::LineCap::Round,
                usvg::LineCap::Square => qt::LineCap::Square,
            };
            pen.set_line_cap(linecap);

            let linejoin = match stroke.linejoin {
                usvg::LineJoin::Miter => qt::LineJoin::Miter,
                usvg::LineJoin::Round => qt::LineJoin::Round,
                usvg::LineJoin::Bevel => qt::LineJoin::Bevel,
            };
            pen.set_line_join(linejoin);

            pen.set_miter_limit(stroke.miterlimit.value());
            pen.set_width(stroke.width.value());

            if let Some(ref list) = stroke.dasharray {
                pen.set_dash_offset(stroke.dashoffset as f64);
                pen.set_dash_array(list);
            }

            p.set_pen(pen);
        }
        None => {
            p.reset_pen();
        }
    }
}
