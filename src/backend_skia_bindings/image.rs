// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::skia;
use usvg::try_opt;

use crate::prelude::*;
use crate::backend_utils::{self, ConvTransform, Image};


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
            skia::Surface::new_raster(&skia::ImageInfo::new_unknown(Some(skia::ISize::new(w as i32, h as i32))), None, None), (),
            "Failed to create a {}x{} surface.", w, h
        );
        let surface_img = image.image_snapshot();
        let mut data = surface_img.encoded_data().unwrap();
        image_to_surface(&img, &mut data);
        image
    };


    let mut filter = skia::FilterQuality::Low;
    if rendering_mode == usvg::ImageRendering::OptimizeSpeed {
        filter = skia::FilterQuality::None;
    }

    let mut canvas = surface.canvas();
    canvas.save();

    if view_box.aspect.slice {
        let r = view_box.rect;
        let rect = skia::Rect::new(r.x() as f32, r.y() as f32, r.width() as f32, r.height() as f32);
        canvas.clip_rect(&rect, None, true);
    }

    let r = backend_utils::image::image_rect(&view_box, img.size);
    let rect = skia::Rect::new(r.x() as f32, r.y() as f32, r.width() as f32, r.height() as f32);
    canvas.draw_image_rect(&image.image_snapshot(), None, rect, &skia::Paint::default());

    // Revert.
    canvas.restore();
}

fn image_to_surface(image: &Image, surface: &mut [u8]) {
    // Surface is always ARGB.
    const SURFACE_CHANNELS: usize = 4;

    use backend_utils::image::ImageData;
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
    surface: &mut skia::Surface,
) {
    let (tree, sub_opt) = try_opt!(backend_utils::image::load_sub_svg(data, opt));

    let img_size = tree.svg_node().size.to_screen_size();
    let (ts, clip) = backend_utils::image::prepare_sub_svg_geom(view_box, img_size);

    if let Some(clip) = clip {
        let rect = skia::Rect::new(clip.x() as f32, clip.y() as f32, clip.width() as f32, clip.height() as f32);
        surface.canvas().clip_rect(&rect, None, true);
    }

    surface.canvas().concat(&ts.to_native());
    super::render_to_canvas(&tree, &sub_opt, img_size, surface);
}
