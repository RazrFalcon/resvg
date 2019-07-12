// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use usvg::try_opt;

use crate::prelude::*;
use crate::backend_utils::{self, AlphaMode, ConvTransform};


pub fn draw_raster(
    format: usvg::ImageFormat,
    data: &usvg::ImageData,
    view_box: usvg::ViewBox,
    rendering_mode: usvg::ImageRendering,
    opt: &Options,
    cr: &cairo::Context,
) {
    let img = try_opt!(backend_utils::image::load_raster(format, data, opt));

    let surface = {
        let mut surface = try_create_surface!(img.size, ());

        {
            // Unwrap is safe, because no one uses the surface.
            let mut surface_data = surface.get_data().unwrap();
            backend_utils::image::image_to_surface(&img, AlphaMode::Premultiplied, &mut surface_data);
        }

        surface
    };

    let (ts, clip) = backend_utils::image::prepare_sub_svg_geom(view_box, img.size);

    if let Some(clip) = clip {
        cr.rectangle(clip.x(), clip.y(), clip.width(), clip.height());
        cr.clip();
    } else {
        // We have to clip the image before rendering because we use `Extend::Pad`.
        let r = backend_utils::image::image_rect(&view_box, img.size);
        cr.rectangle(r.x(), r.y(), r.width(), r.height());
        cr.clip();
    }

    cr.transform(ts.to_native());

    let filter_mode = match rendering_mode {
        usvg::ImageRendering::OptimizeQuality => cairo::Filter::Gaussian,
        usvg::ImageRendering::OptimizeSpeed   => cairo::Filter::Nearest,
    };

    let patt = cairo::SurfacePattern::create(&surface);
    // Do not use `Extend::None`, because it will introduce a "transparent border".
    patt.set_extend(cairo::Extend::Pad);
    patt.set_filter(filter_mode);
    cr.set_source(&patt);
    cr.paint();
    cr.reset_clip();
}

pub fn draw_svg(
    data: &usvg::ImageData,
    view_box: usvg::ViewBox,
    opt: &Options,
    cr: &cairo::Context,
) {
    let (tree, sub_opt) = try_opt!(backend_utils::image::load_sub_svg(data, opt));

    let img_size = tree.svg_node().size.to_screen_size();
    let (ts, clip) = backend_utils::image::prepare_sub_svg_geom(view_box, img_size);

    if let Some(clip) = clip {
        cr.rectangle(clip.x(), clip.y(), clip.width(), clip.height());
        cr.clip();
    }

    cr.transform(ts.to_native());
    super::render_to_canvas(&tree, &sub_opt, img_size, cr);
    cr.reset_clip();
}
