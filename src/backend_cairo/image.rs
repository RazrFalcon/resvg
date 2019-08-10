// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use usvg::try_opt;

use crate::prelude::*;
use crate::image;
use crate::ConvTransform;


pub fn draw_raster(
    format: usvg::ImageFormat,
    data: &usvg::ImageData,
    view_box: usvg::ViewBox,
    rendering_mode: usvg::ImageRendering,
    opt: &Options,
    cr: &cairo::Context,
) {
    let img = try_opt!(image::load_raster(format, data, opt));

    let surface = {
        let mut surface = try_create_surface!(img.size, ());

        {
            // Unwrap is safe, because no one uses the surface.
            let mut surface_data = surface.get_data().unwrap();
            image_to_surface(&img, &mut surface_data);
        }

        surface
    };

    let (ts, clip) = image::prepare_sub_svg_geom(view_box, img.size);

    if let Some(clip) = clip {
        cr.rectangle(clip.x(), clip.y(), clip.width(), clip.height());
        cr.clip();
    } else {
        // We have to clip the image before rendering because we use `Extend::Pad`.
        let r = image::image_rect(&view_box, img.size);
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

fn image_to_surface(image: &image::Image, surface: &mut [u8]) {
    // Surface is always ARGB.
    const SURFACE_CHANNELS: usize = 4;

    use rgb::FromSlice;

    let mut i = 0;

    let mut to_surface = |r, g, b, a| {
        let tr = a * r + 0x80;
        let tg = a * g + 0x80;
        let tb = a * b + 0x80;
        surface[i + 0] = (((tb >> 8) + tb) >> 8) as u8;
        surface[i + 1] = (((tg >> 8) + tg) >> 8) as u8;
        surface[i + 2] = (((tr >> 8) + tr) >> 8) as u8;
        surface[i + 3] = a as u8;

        i += SURFACE_CHANNELS;
    };

    match &image.data {
        image::ImageData::RGB(data) => {
            for p in data.as_rgb() {
                to_surface(p.r as u32, p.g as u32, p.b as u32, 255);
            }
        }
        image::ImageData::RGBA(data) => {
            for p in data.as_rgba() {
                to_surface(p.r as u32, p.g as u32, p.b as u32, p.a as u32);
            }
        }
    }
}

pub fn draw_svg(
    data: &usvg::ImageData,
    view_box: usvg::ViewBox,
    opt: &Options,
    cr: &cairo::Context,
) {
    let (tree, sub_opt) = try_opt!(image::load_sub_svg(data, opt));

    let img_size = tree.svg_node().size.to_screen_size();
    let (ts, clip) = image::prepare_sub_svg_geom(view_box, img_size);

    if let Some(clip) = clip {
        cr.rectangle(clip.x(), clip.y(), clip.width(), clip.height());
        cr.clip();
    }

    cr.transform(ts.to_native());
    super::render_to_canvas(&tree, &sub_opt, img_size, cr);
    cr.reset_clip();
}
