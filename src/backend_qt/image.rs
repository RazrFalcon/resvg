// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::qt;
use usvg::try_opt;

use crate::prelude::*;
use crate::image;
use crate::ConvTransform;


pub fn draw(
    image: &usvg::Image,
    opt: &Options,
    p: &mut qt::Painter,
) -> Rect {
    if image.visibility != usvg::Visibility::Visible {
        return image.view_box.rect;
    }

    if image.format == usvg::ImageFormat::SVG {
        draw_svg(&image.data, image.view_box, opt, p);
    } else {
        draw_raster(image.format, &image.data, image.view_box, image.rendering_mode, opt, p);
    }

    image.view_box.rect
}

pub fn draw_raster(
    format: usvg::ImageFormat,
    data: &usvg::ImageData,
    view_box: usvg::ViewBox,
    rendering_mode: usvg::ImageRendering,
    opt: &Options,
    p: &mut qt::Painter,
) {
    let img = try_opt!(image::load_raster(format, data, opt));

    let image = {
        let (w, h) = img.size.dimensions();
        let mut image = usvg::try_opt_warn_or!(
            qt::Image::new_rgba(w, h), (),
            "Failed to create a {}x{} image.", w, h
        );
        image_to_surface(&img, &mut image.data_mut());
        image
    };

    if rendering_mode == usvg::ImageRendering::OptimizeSpeed {
        p.set_smooth_pixmap_transform(false);
    }

    if view_box.aspect.slice {
        let r = view_box.rect;
        p.set_clip_rect(r.x(), r.y(), r.width(), r.height());
    }

    let r = image::image_rect(&view_box, img.size);
    p.draw_image_rect(r.x(), r.y(), r.width(), r.height(), &image);

    // Revert.
    p.set_smooth_pixmap_transform(true);
    p.reset_clip_path();
}

fn image_to_surface(image: &image::Image, surface: &mut [u8]) {
    // Surface is always ARGB.
    const SURFACE_CHANNELS: usize = 4;

    use crate::image::ImageData;
    use rgb::FromSlice;

    let mut i = 0;

    let mut to_surface = |r, g, b, a| {
        surface[i + 0] = b;
        surface[i + 1] = g;
        surface[i + 2] = r;
        surface[i + 3] = a;

        i += SURFACE_CHANNELS;
    };

    match &image.data {
        ImageData::RGB(data) => {
            for p in data.as_rgb() {
                to_surface(p.r, p.g, p.b, 255);
            }
        }
        ImageData::RGBA(data) => {
            for p in data.as_rgba() {
                to_surface(p.r, p.g, p.b, p.a);
            }
        }
    }
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
