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
    let mut is_butt_cap = true;
    if let Some(ref stroke) = path.stroke {
        is_butt_cap = stroke.linecap == usvg::LineCap::Butt;
    }

    let mut segments = conv_path(&path.segments, is_butt_cap);

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

fn conv_path(
    segments: &[usvg::PathSegment],
    is_butt_cap: bool,
) -> raqote::Path {
    let mut pb = raqote::PathBuilder::new();

    let mut i = 0;
    loop {
        let subpath = get_subpath(i, segments);
        if subpath.is_empty() {
            break;
        }

        conv_subpath(subpath, is_butt_cap, &mut pb);
        i += subpath.len();
    }

    pb.finish()
}

fn get_subpath(
    start: usize,
    segments: &[usvg::PathSegment],
) -> &[usvg::PathSegment] {
    let mut i = start;
    while i < segments.len() {
        match segments[i] {
            usvg::PathSegment::MoveTo { .. } => {
                if i != start {
                    break;
                }
            }
            usvg::PathSegment::ClosePath => {
                i += 1;
                break;
            }
            _ => {}
        }

        i += 1;
    }

    &segments[start..i]
}

fn conv_subpath(
    segments: &[usvg::PathSegment],
    is_butt_cap: bool,
    pb: &mut raqote::PathBuilder,
) {
    assert_ne!(segments.len(), 0);

    // Raqote doesn't support line caps on zero-length subpaths,
    // so we have to implement them manually.
    let is_zero_path = !is_butt_cap && utils::path_length(segments).is_fuzzy_zero();

    if !is_zero_path {
        for seg in segments {
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
        if let usvg::PathSegment::MoveTo { x, y } = segments[0] {
            // Draw zero length path.
            let shift = 0.002; // Purely empirical.
            pb.move_to(x as f32, y as f32);
            pb.line_to(x as f32 + shift, y as f32);
        }
    }
}
