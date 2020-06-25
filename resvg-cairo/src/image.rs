// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use log::warn;
use crate::render::prelude::*;

pub fn draw(image: &usvg::Image, cr: &cairo::Context) -> Rect {
    if image.visibility != usvg::Visibility::Visible {
        return image.view_box.rect;
    }

    draw_kind(&image.kind, image.view_box, image.rendering_mode, cr);
    image.view_box.rect
}

pub fn draw_kind(
    kind: &usvg::ImageKind,
    view_box: usvg::ViewBox,
    rendering_mode: usvg::ImageRendering,
    cr: &cairo::Context,
) {
    match kind {
        usvg::ImageKind::JPEG(ref data) => {
            match read_jpeg(data) {
                Some(image) => draw_raster(&image, view_box, rendering_mode, &cr),
                None => warn!("Failed to load an embedded image."),
            }
        }
        usvg::ImageKind::PNG(ref data) => {
            match read_png(data) {
                Some(image) => draw_raster(&image, view_box, rendering_mode, &cr),
                None => warn!("Failed to load an embedded image."),
            }
        }
        usvg::ImageKind::SVG(ref subtree) => {
            draw_svg(subtree, view_box, &cr);
        }
    }
}

fn draw_raster(
    img: &Image,
    view_box: usvg::ViewBox,
    rendering_mode: usvg::ImageRendering,
    cr: &cairo::Context,
) {
    let surface = {
        let mut surface = try_opt!(crate::render::create_subsurface(img.size));

        {
            // Unwrap is safe, because no one uses the surface.
            let mut surface_data = surface.get_data().unwrap();
            image_to_surface(&img, &mut surface_data);
        }

        surface
    };

    let (ts, clip) = usvg::utils::view_box_to_transform_with_clip(&view_box, img.size);

    if let Some(clip) = clip {
        cr.rectangle(clip.x(), clip.y(), clip.width(), clip.height());
        cr.clip();
    } else {
        // We have to clip the image before rendering because we use `Extend::Pad`.
        let r = image_rect(&view_box, img.size);
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

fn draw_svg(
    tree: &usvg::Tree,
    view_box: usvg::ViewBox,
    cr: &cairo::Context,
) {
    let img_size = tree.svg_node().size.to_screen_size();
    let (ts, clip) = usvg::utils::view_box_to_transform_with_clip(&view_box, img_size);

    if let Some(clip) = clip {
        cr.rectangle(clip.x(), clip.y(), clip.width(), clip.height());
        cr.clip();
    }

    cr.transform(ts.to_native());
    super::render_to_canvas(&tree, img_size, cr);
    cr.reset_clip();
}

/// A raster image data.
#[allow(missing_docs)]
struct Image {
    data: ImageData,
    size: ScreenSize,
}


/// A raster image data kind.
#[allow(missing_docs)]
enum ImageData {
    RGB(Vec<u8>),
    RGBA(Vec<u8>),
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
