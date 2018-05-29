// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use usvg;
use usvg::prelude::*;

// self
use utils;
use geom::*;
use {
    FitTo,
    Options,
};

pub fn load_sub_svg(
    image: &usvg::Image,
    opt: &Options,
) -> Option<(usvg::Tree, Options)> {
    let mut sub_opt = Options {
        usvg: usvg::Options {
            path: None,
            dpi: opt.usvg.dpi,
            keep_named_groups: false,
        },
        fit_to: FitTo::Original,
        background: None,
    };

    let tree = match image.data {
        usvg::ImageData::Path(ref path) => {
            sub_opt.usvg.path = Some(path.into());
            usvg::Tree::from_file(path, &sub_opt.usvg).ok()?
        }
        usvg::ImageData::Raw(ref data) => {
            usvg::Tree::from_data(data, &sub_opt.usvg).ok()?
        }
    };

    Some((tree, sub_opt))
}

pub fn prepare_image_viewbox(img_size: ScreenSize, view_box: &mut usvg::ViewBox) {
    let mut r = view_box.rect;
    // If viewbox w/h is not set - use the one from image.
    if r.width.is_fuzzy_zero() { r.width = img_size.width as f64; }
    if r.height.is_fuzzy_zero() { r.height = img_size.height as f64; }
    view_box.rect = r;
}

pub fn prepare_sub_svg_geom(
    image: &usvg::Image,
    img_size: ScreenSize,
) -> (usvg::Transform, Option<Rect>) {
    let mut view_box = image.view_box;
    prepare_image_viewbox(img_size, &mut view_box);
    let r = view_box.rect;

    let new_size = utils::apply_view_box(&view_box, img_size);

    let (tx, ty, clip) = if view_box.aspect.slice {
        let pos = utils::aligned_pos(
            view_box.aspect.align,
            0.0, 0.0, new_size.width as f64 - r.width, new_size.height as f64 - r.height,
        );

        let r = Rect::new(r.x, r.y, r.width, r.height);
        (r.x - pos.x, r.y - pos.y, Some(r))
    } else {
        let pos = utils::aligned_pos(
            view_box.aspect.align,
            r.x, r.y, r.width - new_size.width as f64, r.height - new_size.height as f64,
        );

        (pos.x, pos.y, None)
    };

    let sx = new_size.width as f64 / img_size.width as f64;
    let sy = new_size.height as f64 / img_size.height as f64;
    let ts = usvg::Transform::new(sx, 0.0, 0.0, sy, tx, ty);

    (ts, clip)
}
