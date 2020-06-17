// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use usvg::{FuzzyZero, Rect};
use crate::{style, Options};


pub fn draw(
    tree: &usvg::Tree,
    path: &usvg::Path,
    opt: &Options,
    cr: &cairo::Context,
) -> Option<Rect> {
    let bbox = path.data.bbox();
    if path.visibility != usvg::Visibility::Visible {
        return bbox;
    }

    let mut is_square_cap = false;
    if let Some(ref stroke) = path.stroke {
        is_square_cap = stroke.linecap == usvg::LineCap::Square;
    }

    draw_path(&path.data, is_square_cap, cr);

    // `usvg` guaranties that path without a bbox will not use
    // a paint server with ObjectBoundingBox,
    // so we can pass whatever rect we want, because it will not be used anyway.
    let style_bbox = bbox.unwrap_or_else(|| Rect::new(0.0, 0.0, 1.0, 1.0).unwrap());

    if !path.rendering_mode.use_shape_antialiasing() {
        cr.set_antialias(cairo::Antialias::None);
    }

    style::fill(tree, &path.fill, opt, style_bbox, cr);
    if path.stroke.is_some() {
        cr.fill_preserve();

        style::stroke(tree, &path.stroke, opt, style_bbox, cr);
        cr.stroke();
    } else {
        cr.fill();
    }

    // Revert anti-aliasing.
    cr.set_antialias(cairo::Antialias::Default);

    bbox
}

fn draw_path(
    path: &usvg::PathData,
    is_square_cap: bool,
    cr: &cairo::Context,
) {
    // Reset path, in case something was left from the previous paint pass.
    cr.new_path();

    for subpath in path.subpaths() {
        draw_subpath(subpath, is_square_cap, cr);
    }
}

fn draw_subpath(
    path: usvg::SubPathData,
    is_square_cap: bool,
    cr: &cairo::Context,
) {
    assert_ne!(path.len(), 0);

    // This is a workaround for a cairo bug(?).
    //
    // Buy the SVG spec, a zero length subpath with a square cap should be
    // rendered as a square/rect, but it's not (at least on 1.14.12/1.15.12).
    // And this is probably a bug, since round caps are rendered correctly.
    let is_zero_path = is_square_cap && path.length().is_fuzzy_zero();

    if !is_zero_path {
        for seg in path.iter() {
            match *seg {
                usvg::PathSegment::MoveTo { x, y } => {
                    cr.new_sub_path();
                    cr.move_to(x, y);
                }
                usvg::PathSegment::LineTo { x, y } => {
                    cr.line_to(x, y);
                }
                usvg::PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                    cr.curve_to(x1, y1, x2, y2, x, y);
                }
                usvg::PathSegment::ClosePath => {
                    cr.close_path();
                }
            }
        }
    } else {
        if let usvg::PathSegment::MoveTo { x, y } = path[0] {
            // Draw zero length path.
            let shift = 0.002; // Purely empirical.
            cr.new_sub_path();
            cr.move_to(x, y);
            cr.line_to(x + shift, y);
        }
    }
}
