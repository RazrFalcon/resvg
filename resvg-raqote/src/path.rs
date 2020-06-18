// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::render::prelude::*;

pub fn draw(
    tree: &usvg::Tree,
    path: &usvg::Path,
    opt: &Options,
    draw_opt: raqote::DrawOptions,
    dt: &mut raqote::DrawTarget,
) -> Option<Rect> {
    let bbox = path.data.bbox();
    if path.visibility != usvg::Visibility::Visible {
        return bbox;
    }

    let mut is_butt_cap = true;
    if let Some(ref stroke) = path.stroke {
        is_butt_cap = stroke.linecap == usvg::LineCap::Butt;
    }

    let mut new_path = conv_path(&path.data, is_butt_cap);

    // `usvg` guaranties that path without a bbox will not use
    // a paint server with ObjectBoundingBox,
    // so we can pass whatever rect we want, because it will not be used anyway.
    let style_bbox = bbox.unwrap_or_else(|| Rect::new(0.0, 0.0, 1.0, 1.0).unwrap());

    if let Some(ref fill) = path.fill {
        match fill.rule {
            usvg::FillRule::NonZero => new_path.winding = raqote::Winding::NonZero,
            usvg::FillRule::EvenOdd => new_path.winding = raqote::Winding::EvenOdd,
        }
    }

    let mut draw_opt = draw_opt.clone();
    if !path.rendering_mode.use_shape_antialiasing() {
        draw_opt.antialias = raqote::AntialiasMode::None;
    }

    crate::paint_server::fill(tree, &new_path, &path.fill, opt, style_bbox, &draw_opt, dt);
    crate::paint_server::stroke(tree, &new_path, &path.stroke, opt, style_bbox, &draw_opt, dt);

    bbox
}

fn conv_path(
    path: &usvg::PathData,
    is_butt_cap: bool,
) -> raqote::Path {
    let mut pb = raqote::PathBuilder::new();

    for subpath in path.subpaths() {
        conv_subpath(subpath, is_butt_cap, &mut pb);
    }

    pb.finish()
}

fn conv_subpath(
    path: usvg::SubPathData,
    is_butt_cap: bool,
    pb: &mut raqote::PathBuilder,
) {
    assert_ne!(path.len(), 0);

    // Raqote doesn't support line caps on zero-length subpaths,
    // so we have to implement them manually.
    let is_zero_path = !is_butt_cap && path.length().is_fuzzy_zero();

    if !is_zero_path {
        for seg in path.iter() {
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
    } else {
        if let usvg::PathSegment::MoveTo { x, y } = path[0] {
            // Draw zero length path.
            let shift = 0.002; // Purely empirical.
            pb.move_to(x as f32, y as f32);
            pb.line_to(x as f32 + shift, y as f32);
        }
    }
}
