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
    stroke: &Option<usvg::Stroke>,
    opt: &Options,
    bbox: Rect,
    dt: &mut raqote::DrawTarget,
) {
    if let Some(ref stroke) = stroke {
        let cap = match stroke.linecap {
            usvg::LineCap::Butt => raqote::LineCap::Butt,
            usvg::LineCap::Round => raqote::LineCap::Round,
            usvg::LineCap::Square => raqote::LineCap::Square,
        };

        let join = match stroke.linejoin {
            usvg::LineJoin::Miter => raqote::LineJoin::Miter,
            usvg::LineJoin::Round => raqote::LineJoin::Round,
            usvg::LineJoin::Bevel => raqote::LineJoin::Bevel,
        };

        let mut dash_array = Vec::new();
        if let Some(ref list) = stroke.dasharray {
            dash_array = list.iter().map(|n| *n as f32).collect();
        }

        let style = raqote::StrokeStyle {
            cap,
            join,
            width: stroke.width.value() as f32,
            miter_limit: stroke.miterlimit.value() as f32,
            dash_array,
            dash_offset: stroke.dashoffset,
        };

        let source = match stroke.paint {
            usvg::Paint::Color(c) => {
                let alpha = (stroke.opacity.value() * 255.0) as u8;
                raqote::Source::Solid(c.to_solid(alpha))
            }
            usvg::Paint::Link(ref id) => {
                if let Some(node) = tree.defs_by_id(id) {
                    match *node.borrow() {
                        usvg::NodeKind::LinearGradient(ref lg) => {
                            gradient::prepare_linear(lg, stroke.opacity, bbox)
                        }
                        usvg::NodeKind::RadialGradient(ref rg) => {
                            gradient::prepare_radial(rg, stroke.opacity, bbox)
                        }
                        usvg::NodeKind::Pattern(ref pattern) => {
//                            pattern::apply(&node, pattern, opt, stroke.opacity, bbox, cr);
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

        dt.stroke(
            &path,
            &source,
            &style,
            &raqote::DrawOptions::default(),
        );
    }
}
