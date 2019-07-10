// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use crate::skia;

// self
use crate::prelude::*;
use crate::backend_utils::*;
use super::style;


pub fn draw(
    tree: &usvg::Tree,
    path: &usvg::Path,
    opt: &Options,
    canvas: &mut skia::Canvas,
    blend_mode: skia::BlendMode
) -> Option<Rect> {

    // TODO:  need to consider having a stateful paint object for the canvas to hold blend mode
    // and (maybe) performance.

    // TODO:  implement fill rule
    let fill_rule = if let Some(ref fill) = path.fill {
        fill.rule
    } else {
        usvg::FillRule::NonZero
    };

    let bbox = utils::path_bbox(&path.segments, None, None);

    // `usvg` guaranties that path without a bbox will not use
    // a paint server with ObjectBoundingBox,
    // so we can pass whatever rect we want, because it will not be used anyway.
    let style_bbox = bbox.unwrap_or_else(|| Rect::new(0.0, 0.0, 1.0, 1.0).unwrap());

    if path.visibility != usvg::Visibility::Visible {
        return bbox;
    }

    let anti_alias = use_shape_antialiasing(path.rendering_mode);

    if path.fill.is_some() {
        let mut fill = style::fill(tree, &path.fill, opt, style_bbox, canvas);
        fill.set_anti_alias(anti_alias);
        fill.set_blend_mode(blend_mode);
        draw_path(&path.segments, true, canvas, &fill);
    }

    if path.stroke.is_some() {
        let mut stroke = style::stroke(tree, &path.stroke, opt, style_bbox, canvas);
        stroke.set_anti_alias(anti_alias);
        stroke.set_blend_mode(blend_mode);
        draw_path(&path.segments, true, canvas, &stroke);
    }
    
    bbox
}


fn draw_path(
    segments: &[usvg::PathSegment],
    is_square_cap: bool,
    canvas: &mut skia::Canvas,
    paint: &skia::Paint
) {
   
    let mut s_path = skia::Path::new();

    let mut i = 0;
    loop {
        let subpath = get_subpath(i, segments);
        if subpath.is_empty() {
            break;
        }

        draw_subpath(subpath, is_square_cap, &mut s_path);
        i += subpath.len();
    }

    canvas.draw_path(&s_path, &paint);
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

fn draw_subpath(
    segments: &[usvg::PathSegment],
    is_square_cap: bool,
    s_path: &mut skia::Path
) {
    assert_ne!(segments.len(), 0);

    // Buy the SVG spec, a zero length subpath with a square cap should be
    // rendered as a square/rect, but it's not (at least on 1.14.12/1.15.12).
    // And this is probably a bug, since round caps are rendered correctly.
    let is_zero_path = is_square_cap && utils::path_length(segments).is_fuzzy_zero();

    if !is_zero_path {
        for seg in segments {
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
    } else {
        if let usvg::PathSegment::MoveTo { x, y } = segments[0] {
            // Draw zero length path.
            let shift = 0.002; // Purely empirical.
            s_path.move_to(x, y);
            s_path.line_to(x + shift, y);
        }
    }
}
