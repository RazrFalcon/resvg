// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::skia;

use crate::prelude::*;
use crate::image;
use crate::ConvTransform;


pub fn draw(
    image: &usvg::Image,
    opt: &Options,
    canvas: &mut skia::Canvas,
) -> Rect {
    if image.visibility != usvg::Visibility::Visible {
        return image.view_box.rect;
    }

    if image.format == usvg::ImageFormat::SVG {
        draw_svg(&image.data, image.view_box, opt, canvas);
    } else {
        draw_raster(image.format, &image.data, image.view_box, image.rendering_mode, opt, canvas);
    }

    image.view_box.rect
}

pub fn draw_raster(
    format: usvg::ImageFormat,
    data: &usvg::ImageData,
    view_box: usvg::ViewBox,
    rendering_mode: usvg::ImageRendering,
    opt: &Options,
    canvas: &mut skia::Canvas,
) {
    let img = try_opt!(image::load_raster(format, data, opt));

    let image = {
        let (w, h) = img.size.dimensions();
        let mut image = try_opt_warn_or!(
            skia::Surface::new_rgba(w, h), (),
            "Failed to create a {}x{} surface.", w, h
        );

        image_to_surface(&img, &mut image.data_mut());
        image
    };


    let mut filter = skia::FilterQuality::Low;
    if rendering_mode == usvg::ImageRendering::OptimizeSpeed {
        filter = skia::FilterQuality::None;
    }

    canvas.save();

    if view_box.aspect.slice {
        let r = view_box.rect;
        canvas.set_clip_rect(r.x(), r.y(), r.width(), r.height());
    }

    let r = image::image_rect(&view_box, img.size);
    canvas.draw_surface_rect(&image, r.x(), r.y(), r.width(), r.height(), filter);

    // Revert.
    canvas.restore();
}

fn image_to_surface(image: &image::Image, surface: &mut [u8]) {
    // Surface is always ARGB.
    const SURFACE_CHANNELS: usize = 4;

    use crate::image::ImageData;
    use rgb::FromSlice;

    let mut i = 0;
    if skia::Surface::is_bgra() {
        match &image.data {
            ImageData::RGB(data) => {
                for p in data.as_rgb() {
                    surface[i + 0] = p.b;
                    surface[i + 1] = p.g;
                    surface[i + 2] = p.r;
                    surface[i + 3] = 255;

                    i += SURFACE_CHANNELS;
                }
            }
            ImageData::RGBA(data) => {
                for p in data.as_rgba() {
                    surface[i + 0] = p.b;
                    surface[i + 1] = p.g;
                    surface[i + 2] = p.r;
                    surface[i + 3] = p.a;

                    i += SURFACE_CHANNELS;
                }
            }
        }
    } else {
        match &image.data {
            ImageData::RGB(data) => {
                for p in data.as_rgb() {
                    surface[i + 0] = p.r;
                    surface[i + 1] = p.g;
                    surface[i + 2] = p.b;
                    surface[i + 3] = 255;

                    i += SURFACE_CHANNELS;
                }
            }
            ImageData::RGBA(data) => {
                for p in data.as_rgba() {
                    surface[i + 0] = p.r;
                    surface[i + 1] = p.g;
                    surface[i + 2] = p.b;
                    surface[i + 3] = p.a;

                    i += SURFACE_CHANNELS;
                }
            }
        }
    }
}

pub fn draw_svg(
    data: &usvg::ImageData,
    view_box: usvg::ViewBox,
    opt: &Options,
    canvas: &mut skia::Canvas,
) {
    let (tree, sub_opt) = try_opt!(image::load_sub_svg(data, opt));

    let img_size = tree.svg_node().size.to_screen_size();
    let (ts, clip) = image::prepare_sub_svg_geom(view_box, img_size);

    canvas.save();

    if let Some(clip) = clip {
        canvas.set_clip_rect(clip.x(), clip.y(), clip.width(), clip.height());
    }

    canvas.concat(&ts.to_native());
    super::render_to_canvas(&tree, &sub_opt, img_size, canvas);

    canvas.restore();
}
