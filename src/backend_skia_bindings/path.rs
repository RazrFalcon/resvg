// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::skia;
use crate::prelude::*;
use crate::backend_utils::*;
use super::style;

pub fn draw(
    tree: &usvg::Tree,
    path: &usvg::Path,
    opt: &Options,
    bbox: Option<Rect>,
    canvas: &mut skia::Canvas,
    blend_mode: skia::BlendMode
) -> Option<Rect> {
    // `usvg` guaranties that path without a bbox will not use
    // a paint server with ObjectBoundingBox,
    // so we can pass whatever rect we want, because it will not be used anyway.
    let style_bbox = bbox.unwrap_or_else(|| Rect::new(0.0, 0.0, 1.0, 1.0).unwrap());

    let mut skia_path = convert_path(&path.segments);
    if let Some(ref fill) = path.fill {
        if fill.rule == usvg::FillRule::EvenOdd {
            skia_path.set_fill_type(skia::PathFillType::EvenOdd);
        }
    };

    let antialias = use_shape_antialiasing(path.rendering_mode);

    let global_ts = usvg::Transform::from_native(&canvas.total_matrix());

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
    segments: &[usvg::PathSegment],
) -> skia::Path {
    let mut s_path = skia::Path::new();
    for seg in segments {
        match *seg {
            usvg::PathSegment::MoveTo { x, y } => {
                s_path.move_to(skia::Point::new(x as f32, y as f32));
            }
            usvg::PathSegment::LineTo { x, y } => {
                s_path.line_to(skia::Point::new(x as f32, y as f32));
            }
            usvg::PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                let p1 = skia::Point::new(x1 as f32, y1 as f32);
                let p2 = skia::Point::new(x2 as f32, y2 as f32);
                let p3 = skia::Point::new(x as f32, y as f32);
                s_path.cubic_to(p1, p2, p3);
            }
            usvg::PathSegment::ClosePath => {
                s_path.close();
            }
        }
    }

    s_path
}
