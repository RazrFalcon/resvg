// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::prelude::*;
use super::style;


pub fn draw(
    tree: &usvg::Tree,
    path: &usvg::Path,
    opt: &Options,
    draw_opt: &raqote::DrawOptions,
    dt: &mut raqote::DrawTarget,
) -> Option<Rect> {
    let mut pb = raqote::PathBuilder::new();
    for seg in &path.segments {
        match *seg {
            usvg::PathSegment::MoveTo { x, y } => {
                pb.move_to(x as f32, y as f32);
            }
            usvg::PathSegment::LineTo { x, y } => {
                pb.line_to(x as f32, y as f32);
            }
            usvg::PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                pb.cubic_to(x1 as f32, y1 as f32, x2 as f32, y2 as f32, x as f32, y as f32);
            }
            usvg::PathSegment::ClosePath => {
                pb.close();
            }
        }
    }
    let mut segments = pb.finish();

    let bbox = utils::path_bbox(&path.segments, None, None);

    // `usvg` guaranties that path without a bbox will not use
    // a paint server with ObjectBoundingBox,
    // so we can pass whatever rect we want, because it will not be used anyway.
    let style_bbox = bbox.unwrap_or_else(|| Rect::new(0.0, 0.0, 1.0, 1.0).unwrap());

    if path.visibility != usvg::Visibility::Visible {
        return bbox;
    }

//    if !backend_utils::use_shape_antialiasing(path.rendering_mode) {
//        cr.set_antialias(cairo::Antialias::None);
//    }

    if let Some(ref fill) = path.fill {
        match fill.rule {
            usvg::FillRule::NonZero => segments.winding = raqote::Winding::NonZero,
            usvg::FillRule::EvenOdd => segments.winding = raqote::Winding::EvenOdd,
        }
    }

    style::fill(tree, &segments, &path.fill, opt, style_bbox, draw_opt, dt);
    style::stroke(tree, &segments, &path.stroke, opt, style_bbox, draw_opt, dt);

//    // Revert anti-aliasing.
//    cr.set_antialias(cairo::Antialias::Default);

    bbox
}
