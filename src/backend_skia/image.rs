// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::skia;
use usvg::try_opt;

use crate::prelude::*;
use crate::backend_utils::{self, AlphaMode, ConvTransform};


pub fn draw_raster(
    format: usvg::ImageFormat,
    data: &usvg::ImageData,
    view_box: usvg::ViewBox,
    rendering_mode: usvg::ImageRendering,
    opt: &Options,
    surface: &mut skia::Surface,
) {
    let img = try_opt!(backend_utils::image::load_raster(format, data, opt));

    let image = {
        let (w, h) = img.size.dimensions();
        let mut image = usvg::try_opt_warn_or!(
            skia::Surface::new_rgba(w, h), (),
            "Failed to create a {}x{} surface.", w, h
        );

        backend_utils::image::image_to_surface(&img, AlphaMode::AsIs, &mut image.data_mut());
        image
    };


    let mut filter = skia::FilterQuality::Low;
    if rendering_mode == usvg::ImageRendering::OptimizeSpeed {
        filter = skia::FilterQuality::None;
    }

    let mut canvas = surface.canvas_mut();
    canvas.save();

    if view_box.aspect.slice {
        let r = view_box.rect;
        canvas.set_clip_rect(r.x(), r.y(), r.width(), r.height());
    }

    let r = backend_utils::image::image_rect(&view_box, img.size);
    canvas.draw_surface_rect(&image, r.x(), r.y(), r.width(), r.height(), filter);

    // Revert.
    canvas.restore();
}

pub fn draw_svg(
    data: &usvg::ImageData,
    view_box: usvg::ViewBox,
    opt: &Options,
    surface: &mut skia::Surface,
) {
    let (tree, sub_opt) = try_opt!(backend_utils::image::load_sub_svg(data, opt));

    let img_size = tree.svg_node().size.to_screen_size();
    let (ts, clip) = backend_utils::image::prepare_sub_svg_geom(view_box, img_size);

    if let Some(clip) = clip {
        surface.canvas_mut().set_clip_rect(clip.x(), clip.y(), clip.width(), clip.height());
    }

    surface.canvas_mut().concat(&ts.to_native());
    super::render_to_canvas(&tree, &sub_opt, img_size, surface);
}
