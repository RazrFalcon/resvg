// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::qt;
use usvg::{try_opt, try_opt_warn};

use crate::{prelude::*, backend_utils::*};


pub fn draw_raster(
    data: &usvg::ImageData,
    view_box: usvg::ViewBox,
    rendering_mode: usvg::ImageRendering,
    opt: &Options,
    p: &mut qt::Painter,
) {
    let img = match data {
        usvg::ImageData::Path(ref path) => {
            let path = image::get_abs_path(path, opt);
            try_opt_warn!(
                qt::Image::from_file(&path),
                "Failed to load an external image: {:?}.", path
            )
        }
        usvg::ImageData::Raw(ref data) => {
            try_opt_warn!(
                qt::Image::from_data(data),
                "Failed to load an embedded image."
            )
        }
    };

    let img_size = try_opt!(ScreenSize::new(img.width(), img.height()));

    if rendering_mode == usvg::ImageRendering::OptimizeSpeed {
        p.set_smooth_pixmap_transform(false);
    }

    if view_box.aspect.slice {
        let r = view_box.rect;
        p.set_clip_rect(r.x(), r.y(), r.width(), r.height());
    }

    let r = image::image_rect(&view_box, img_size);
    p.draw_image_rect(r.x(), r.y(), r.width(), r.height(), &img);

    // Revert.
    p.set_smooth_pixmap_transform(true);
    p.reset_clip_path();
}

pub fn draw_svg(
    data: &usvg::ImageData,
    view_box: usvg::ViewBox,
    opt: &Options,
    p: &mut qt::Painter,
) {
    let (tree, sub_opt) = try_opt!(image::load_sub_svg(data, opt));

    let img_size = tree.svg_node().size.to_screen_size();
    let (ts, clip) = image::prepare_sub_svg_geom(view_box, img_size);

    if let Some(clip) = clip {
        p.set_clip_rect(clip.x(), clip.y(), clip.width(), clip.height());
    }

    p.apply_transform(&ts.to_native());
    super::render_to_canvas(&tree, &sub_opt, img_size, p);
    p.reset_clip_path();
}
