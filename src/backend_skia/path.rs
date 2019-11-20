// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::skia;

use crate::prelude::*;
use crate::ConvTransform;
use super::style;


pub fn draw(
    tree: &usvg::Tree,
    path: &usvg::Path,
    opt: &Options,
    blend_mode: skia::BlendMode,
    canvas: &mut skia::Canvas,
) -> Option<Rect> {
    let bbox = path.data.bbox();
    if path.visibility != usvg::Visibility::Visible {
        return bbox;
    }

    // `usvg` guaranties that path without a bbox will not use
    // a paint server with ObjectBoundingBox,
    // so we can pass whatever rect we want, because it will not be used anyway.
    let style_bbox = bbox.unwrap_or_else(|| Rect::new(0.0, 0.0, 1.0, 1.0).unwrap());

    let mut skia_path = convert_path(&path.data);
    if let Some(ref fill) = path.fill {
        if fill.rule == usvg::FillRule::EvenOdd {
            skia_path.set_fill_type(skia::FillType::EvenOdd);
        }
    };

    let antialias = crate::use_shape_antialiasing(path.rendering_mode);

    let global_ts = usvg::Transform::from_native(&canvas.get_matrix());

    if path.fill.is_some() {
        let mut fill = style::fill(tree, &path.fill, opt, style_bbox, global_ts);
        fill.set_anti_alias(antialias);
        fill.set_blend_mode(blend_mode);
        canvas.draw_path(&skia_path, &fill);
    }

    if path.stroke.is_some() {
        let mut stroke = style::stroke(tree, &path.stroke, opt, style_bbox, global_ts);
        stroke.set_anti_alias(antialias);
        stroke.set_blend_mode(blend_mode);
        canvas.draw_path(&skia_path, &stroke);
    }

    bbox
}

fn convert_path(
    path: &usvg::PathData,
) -> skia::Path {
    let mut s_path = skia::Path::new();
    for seg in path.iter() {
        match *seg {
            usvg::PathSegment::MoveTo { x, y } => {
                s_path.move_to(x, y);
            }
            usvg::PathSegment::LineTo { x, y } => {
                s_path.line_to(x, y);
            }
            usvg::PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                s_path.cubic_to(x1, y1, x2, y2, x, y);
            }
            usvg::PathSegment::ClosePath => {
                s_path.close();
            }
        }
    }

    s_path
}
