// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use log::warn;
use crate::render::prelude::*;

pub fn draw(
    image: &usvg::Image,
    opt: &Options,
    dt: &mut raqote::DrawTarget,
) -> Rect {
    if image.visibility != usvg::Visibility::Visible {
        return image.view_box.rect;
    }

    if image.format == usvg::ImageFormat::SVG {
        draw_svg(&image.data, image.view_box, opt, dt);
    } else {
        draw_raster(image.format, &image.data, image.view_box, image.rendering_mode, opt, dt);
    }

    image.view_box.rect
}

pub fn draw_raster(
    format: usvg::ImageFormat,
    data: &usvg::ImageData,
    view_box: usvg::ViewBox,
    rendering_mode: usvg::ImageRendering,
    opt: &Options,
    dt: &mut raqote::DrawTarget,
) {
    let img = try_opt!(load_raster(format, data, opt));

    let sub_dt = {
        let mut sub_dt = raqote::DrawTarget::new(img.size.width() as i32, img.size.height() as i32);
        let surface_data = sub_dt.get_data_u8_mut();
        image_to_surface(&img, surface_data);
        sub_dt
    };

    let (ts, clip) = usvg::utils::view_box_to_transform_with_clip(&view_box, img.size);

    let mut pb = raqote::PathBuilder::new();
    if let Some(clip) = clip {
        pb.rect(clip.x() as f32, clip.y() as f32, clip.width() as f32, clip.height() as f32);
    } else {
        // We have to clip the image before rendering because we use `Extend::Pad`.
        let r = image_rect(&view_box, img.size);
        pb.rect(r.x() as f32, r.y() as f32, r.width() as f32, r.height() as f32);
    }

    let filter_mode = if rendering_mode == usvg::ImageRendering::OptimizeSpeed {
        raqote::FilterMode::Nearest
    } else {
        raqote::FilterMode::Bilinear
    };

    let t: raqote::Transform = ts.to_native();
    let patt = raqote::Source::Image(
        sub_dt.as_image(),
        raqote::ExtendMode::Pad,
        filter_mode,
        t.inverse().unwrap(),
    );

    dt.fill(&pb.finish(), &patt, &raqote::DrawOptions::default());
}

fn image_to_surface(image: &Image, surface: &mut [u8]) {
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
        ImageData::RGB(data) => {
            for p in data.as_rgb() {
                to_surface(p.r as u32, p.g as u32, p.b as u32, 255);
            }
        }
        ImageData::RGBA(data) => {
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
    dt: &mut raqote::DrawTarget,
) {
    let (tree, sub_opt) = try_opt!(data.load_svg(&opt.usvg));

    let sub_opt = Options {
        usvg: sub_opt,
        fit_to: FitTo::Original,
        background: None,
    };

    let img_size = tree.svg_node().size.to_screen_size();
    let (ts, clip) = usvg::utils::view_box_to_transform_with_clip(&view_box, img_size);

    if let Some(clip) = clip {
        let mut pb = raqote::PathBuilder::new();

        pb.rect(clip.x() as f32, clip.y() as f32, clip.width() as f32, clip.height() as f32);
        dt.push_clip(&pb.finish());
    }

    dt.transform(&ts.to_native());
    crate::render_to_canvas(&tree, &sub_opt, img_size, dt);

    if let Some(_) = clip {
        dt.pop_clip();
    }
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
pub fn load_raster(
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
pub fn image_rect(
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
