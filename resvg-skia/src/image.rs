// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use log::warn;
use usvg::{Rect, FitTo, ScreenSize};
use crate::{skia, ConvTransform, Options};


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
    let img = try_opt!(load_raster(format, data, opt));

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

    let r = image_rect(&view_box, img.size);
    canvas.draw_surface_rect(&image, r.x(), r.y(), r.width(), r.height(), filter);

    // Revert.
    canvas.restore();
}

fn image_to_surface(image: &Image, surface: &mut [u8]) {
    // Surface is always ARGB.
    const SURFACE_CHANNELS: usize = 4;

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
    let (tree, sub_opt) = try_opt!(data.load_svg(&opt.usvg));

    let sub_opt = Options {
        usvg: sub_opt,
        fit_to: FitTo::Original,
        background: None,
    };

    let img_size = tree.svg_node().size.to_screen_size();
    let (ts, clip) = usvg::utils::view_box_to_transform_with_clip(&view_box, img_size);

    canvas.save();

    if let Some(clip) = clip {
        canvas.set_clip_rect(clip.x(), clip.y(), clip.width(), clip.height());
    }

    canvas.concat(&ts.to_native());
    super::render_to_canvas(&tree, &sub_opt, img_size, canvas);

    canvas.restore();
}

/// A raster image data.
#[allow(missing_docs)]
pub struct Image {
    pub data: ImageData,
    pub size: ScreenSize,
}


/// A raster image data kind.
#[allow(missing_docs)]
pub enum ImageData {
    RGB(Vec<u8>),
    RGBA(Vec<u8>),
}

/// Loads a raster image.
fn load_raster(
    format: usvg::ImageFormat,
    data: &usvg::ImageData,
    opt: &Options,
) -> Option<Image> {
    let img = _load_raster(format, data, opt);

    if img.is_none() {
        match data {
            usvg::ImageData::Path(ref path) => {
                let path = opt.usvg.get_abs_path(path);
                warn!("Failed to load an external image: {:?}.", path);
            }
            usvg::ImageData::Raw(_) => {
                warn!("Failed to load an embedded image.");
            }
        }
    }

    img
}

fn _load_raster(
    format: usvg::ImageFormat,
    data: &usvg::ImageData,
    opt: &Options,
) -> Option<Image> {
    debug_assert!(format != usvg::ImageFormat::SVG);

    match data {
        usvg::ImageData::Path(ref path) => {
            let path = opt.usvg.get_abs_path(path);
            let data = std::fs::read(path).ok()?;

            if format == usvg::ImageFormat::JPEG {
                read_jpeg(&data)
            } else {
                read_png(&data)
            }
        }
        usvg::ImageData::Raw(ref data) => {
            if format == usvg::ImageFormat::JPEG {
                read_jpeg(data)
            } else {
                read_png(data)
            }
        }
    }
}

fn read_png(data: &[u8]) -> Option<Image> {
    let decoder = png::Decoder::new(data);
    let (info, mut reader) = decoder.read_info().ok()?;

    let size = ScreenSize::new(info.width, info.height)?;

    let mut img_data = vec![0; info.buffer_size()];
    reader.next_frame(&mut img_data).ok()?;

    let data = match info.color_type {
        png::ColorType::RGB => ImageData::RGB(img_data),
        png::ColorType::RGBA => ImageData::RGBA(img_data),
        png::ColorType::Grayscale => {
            let mut rgb_data = Vec::with_capacity(img_data.len() * 3);
            for gray in img_data {
                rgb_data.push(gray);
                rgb_data.push(gray);
                rgb_data.push(gray);
            }

            ImageData::RGB(rgb_data)
        }
        png::ColorType::GrayscaleAlpha => {
            let mut rgba_data = Vec::with_capacity(img_data.len() * 2);
            for slice in img_data.chunks(2) {
                let gray = slice[0];
                let alpha = slice[1];
                rgba_data.push(gray);
                rgba_data.push(gray);
                rgba_data.push(gray);
                rgba_data.push(alpha);
            }

            ImageData::RGBA(rgba_data)
        }
        png::ColorType::Indexed => {
            warn!("Indexed PNG is not supported.");
            return None
        }
    };

    Some(Image {
        data,
        size,
    })
}

fn read_jpeg(data: &[u8]) -> Option<Image> {
    let mut decoder = jpeg_decoder::Decoder::new(data);
    let img_data = decoder.decode().ok()?;
    let info = decoder.info()?;

    let size = ScreenSize::new(info.width as u32, info.height as u32)?;

    let data = match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => ImageData::RGB(img_data),
        jpeg_decoder::PixelFormat::L8 => {
            let mut rgb_data = Vec::with_capacity(img_data.len() * 3);
            for gray in img_data {
                rgb_data.push(gray);
                rgb_data.push(gray);
                rgb_data.push(gray);
            }

            ImageData::RGB(rgb_data)
        }
        _ => return None,
    };

    Some(Image {
        data,
        size,
    })
}

/// Calculates an image rect depending on the provided view box.
fn image_rect(
    view_box: &usvg::ViewBox,
    img_size: ScreenSize,
) -> Rect {
    let new_size = img_size.fit_view_box(view_box);
    let (x, y) = usvg::utils::aligned_pos(
        view_box.aspect.align,
        view_box.rect.x(),
        view_box.rect.y(),
        view_box.rect.width() - new_size.width() as f64,
        view_box.rect.height() - new_size.height() as f64,
    );

    new_size.to_size().to_rect(x, y)
}
