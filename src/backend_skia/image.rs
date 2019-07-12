// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::skia;
use usvg::try_opt;

use crate::prelude::*;
use crate::backend_utils::{self, ConvTransform};


pub fn draw_raster(
    format: usvg::ImageFormat,
    data: &usvg::ImageData,
    view_box: usvg::ViewBox,
    rendering_mode: usvg::ImageRendering,
    opt: &Options,
    canvas: &mut skia::Canvas,
) {
    let img = try_opt!(backend_utils::image::load_raster(format, data, opt));

    let image = {
        let mut image = try_create_surface!(img.size, ());
        backend_utils::image::image_to_surface(&img, &mut image.data_mut());
        image
    };

//    if rendering_mode == usvg::ImageRendering::OptimizeSpeed {
//        canvas.set_smooth_pixmap_transform(false);
//    }

    if view_box.aspect.slice {
        let r = view_box.rect;
        canvas.clip_rect(r.x(), r.y(), r.width(), r.height());
    }

    let r = backend_utils::image::image_rect(&view_box, img.size);
    canvas.draw_surface(&last.img, r.x(), r.y(), 255, skia::BlendMode::SourceOver);

    // Revert.
//    p.set_smooth_pixmap_transform(true);
//    p.reset_clip_path();
}

pub fn draw_svg(
    data: &usvg::ImageData,
    view_box: usvg::ViewBox,
    opt: &Options,
    canvas: &mut skia::Canvas,
) {
    let (tree, sub_opt) = try_opt!(backend_utils::image::load_sub_svg(data, opt));

    let img_size = tree.svg_node().size.to_screen_size();
    let (ts, clip) = backend_utils::image::prepare_sub_svg_geom(view_box, img_size);

    if let Some(clip) = clip {
        canvas.clip_rect(clip.x(), clip.y(), clip.width(), clip.height());
    }

    canvas.concat(&ts.to_native());
    super::render_to_canvas(&tree, &sub_opt, img_size, canvas);
}
