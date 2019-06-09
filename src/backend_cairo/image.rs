// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use cairo::PatternTrait;
use gdk_pixbuf::PixbufLoaderExt;
use usvg::{try_opt, try_opt_warn};

use crate::{prelude::*, backend_utils::*};


pub fn draw(
    image: &usvg::Image,
    opt: &Options,
    cr: &cairo::Context,
) -> Rect {
    if image.visibility != usvg::Visibility::Visible {
        return image.view_box.rect;
    }

    if image.format == usvg::ImageFormat::SVG {
        draw_svg(&image.data, image.view_box, opt, cr);
    } else {
        draw_raster(&image.data, image.view_box, image.rendering_mode, opt, cr);
    }

    image.view_box.rect
}

pub fn draw_raster(
    data: &usvg::ImageData,
    view_box: usvg::ViewBox,
    rendering_mode: usvg::ImageRendering,
    opt: &Options,
    cr: &cairo::Context,
) {
    let img = match data {
        usvg::ImageData::Path(ref path) => {
            let path = image::get_abs_path(path, opt);
            try_opt_warn!(
                gdk_pixbuf::Pixbuf::new_from_file(path.clone()).ok(),
                "Failed to load an external image: {:?}.", path
            )
        }
        usvg::ImageData::Raw(ref data) => {
            try_opt_warn!(
                load_raster_data(data),
                "Failed to load an embedded image."
            )
        }
    };

    let img_size = ScreenSize::new(img.get_width() as u32, img.get_height() as u32);
    let img_size = try_opt!(img_size);

    let surface = try_opt!(gdk_pixbuf_to_surface(img, img_size));

    let (ts, clip) = image::prepare_sub_svg_geom(view_box, img_size);

    if let Some(clip) = clip {
        cr.rectangle(clip.x(), clip.y(), clip.width(), clip.height());
        cr.clip();
    } else {
        // We have to clip the image before rendering because we use `Extend::Pad`.
        let r = image::image_rect(&view_box, img_size);
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
    cr.set_source(&cairo::Pattern::SurfacePattern(patt));
    cr.paint();
    cr.reset_clip();
}

fn load_raster_data(data: &[u8]) -> Option<gdk_pixbuf::Pixbuf> {
    let loader = gdk_pixbuf::PixbufLoader::new();
    loader.write(data).ok()?;
    loader.close().ok()?;
    loader.get_pixbuf()
}

fn gdk_pixbuf_to_surface(
    img: gdk_pixbuf::Pixbuf,
    img_size: ScreenSize,
) -> Option<cairo::ImageSurface> {
    let mut surface = try_create_surface!(img_size, None);

    {
        // Unwrap is safe, because no one uses the surface.
        let mut surface_data = surface.get_data().unwrap();

        let channels = img.get_n_channels() as u32;
        let w = img.get_width() as u32;
        let h = img.get_height() as u32;
        let stride = img.get_rowstride() as u32;
        let img_pixels = unsafe { img.get_pixels() };
        // We can't iterate over pixels directly, because width may not be equal to stride.
        let mut i = 0;
        for y in 0..h {
            for x in 0..w {
                let idx = (y * stride + x * channels) as usize;

                // NOTE: will not work on big endian.
                if channels == 4 {
                    let r = img_pixels[idx + 0] as u32;
                    let g = img_pixels[idx + 1] as u32;
                    let b = img_pixels[idx + 2] as u32;
                    let a = img_pixels[idx + 3] as u32;

                    // https://www.cairographics.org/manual/cairo-Image-Surfaces.html#cairo-format-t
                    let tr = a * r + 0x80;
                    let tg = a * g + 0x80;
                    let tb = a * b + 0x80;
                    surface_data[i + 0] = (((tb >> 8) + tb) >> 8) as u8;
                    surface_data[i + 1] = (((tg >> 8) + tg) >> 8) as u8;
                    surface_data[i + 2] = (((tr >> 8) + tr) >> 8) as u8;
                    surface_data[i + 3] = a as u8; // TODO: is needed?
                } else {
                    surface_data[i + 0] = img_pixels[idx + 2];
                    surface_data[i + 1] = img_pixels[idx + 1];
                    surface_data[i + 2] = img_pixels[idx + 0];
                    surface_data[i + 3] = 255;
                }

                // Surface is always ARGB.
                i += 4;
            }
        }
    }

    Some(surface)
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
