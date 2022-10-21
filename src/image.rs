// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{render::Canvas, ConvTransform};

pub fn draw(image: &usvg::Image, canvas: &mut Canvas) -> usvg::PathBbox {
    if image.visibility != usvg::Visibility::Visible {
        return image.view_box.rect.to_path_bbox();
    }

    draw_kind(&image.kind, image.view_box, image.rendering_mode, canvas);
    image.view_box.rect.to_path_bbox()
}

pub fn draw_kind(
    kind: &usvg::ImageKind,
    view_box: usvg::ViewBox,
    #[allow(unused_variables)] rendering_mode: usvg::ImageRendering,
    canvas: &mut Canvas,
) {
    match kind {
        usvg::ImageKind::SVG(ref subtree) => {
            draw_svg(subtree, view_box, canvas);
        }
        #[cfg(feature = "raster-images")]
        usvg::ImageKind::JPEG(ref data) => match raster_images::read_jpeg(data) {
            Some(image) => {
                raster_images::draw_raster(&image, view_box, rendering_mode, canvas);
            }
            None => log::warn!("Failed to decode a JPEG image."),
        },
        #[cfg(feature = "raster-images")]
        usvg::ImageKind::PNG(ref data) => match raster_images::read_png(data) {
            Some(image) => {
                raster_images::draw_raster(&image, view_box, rendering_mode, canvas);
            }
            None => log::warn!("Failed to decode a PNG image."),
        },
        #[cfg(feature = "raster-images")]
        usvg::ImageKind::GIF(ref data) => match raster_images::read_gif(data) {
            Some(image) => {
                raster_images::draw_raster(&image, view_box, rendering_mode, canvas);
            }
            None => log::warn!("Failed to decode a GIF image."),
        },
        #[cfg(not(feature = "raster-images"))]
        _ => {
            log::warn!("Images decoding was disabled by a build feature.");
        }
    }
}

fn draw_svg(tree: &usvg::Tree, view_box: usvg::ViewBox, canvas: &mut Canvas) -> Option<()> {
    let img_size = tree.size.to_screen_size();
    let (ts, clip) = usvg::utils::view_box_to_transform_with_clip(&view_box, img_size);

    let mut sub_pixmap = canvas.pixmap.to_owned();
    sub_pixmap.fill(tiny_skia::Color::TRANSPARENT);
    let mut sub_canvas = Canvas::from(sub_pixmap.as_mut());
    sub_canvas.transform = canvas.transform;
    sub_canvas.apply_transform(ts.to_native());
    crate::render::render_to_canvas(tree, img_size, &mut sub_canvas);

    if let Some(clip) = clip {
        let rr = tiny_skia::Rect::from_xywh(
            clip.x() as f32,
            clip.y() as f32,
            clip.width() as f32,
            clip.height() as f32,
        )?;
        canvas.set_clip_rect(rr);
    }

    canvas.pixmap.draw_pixmap(
        0,
        0,
        sub_pixmap.as_ref(),
        &tiny_skia::PixmapPaint::default(),
        tiny_skia::Transform::identity(),
        canvas.clip.as_ref(),
    );
    canvas.clip = None;

    Some(())
}

#[cfg(feature = "raster-images")]
mod raster_images {
    use crate::render::Canvas;

    pub fn draw_raster(
        img: &Image,
        view_box: usvg::ViewBox,
        rendering_mode: usvg::ImageRendering,
        canvas: &mut Canvas,
    ) -> Option<()> {
        let (w, h) = img.size.dimensions();
        let mut pixmap = tiny_skia::Pixmap::new(w, h)?;
        image_to_pixmap(img, pixmap.data_mut());

        let mut filter = tiny_skia::FilterQuality::Bicubic;
        if rendering_mode == usvg::ImageRendering::OptimizeSpeed {
            filter = tiny_skia::FilterQuality::Nearest;
        }

        let r = image_rect(&view_box, img.size);
        let rect = tiny_skia::Rect::from_xywh(
            r.x() as f32,
            r.y() as f32,
            r.width() as f32,
            r.height() as f32,
        )?;

        let ts = tiny_skia::Transform::from_row(
            rect.width() as f32 / pixmap.width() as f32,
            0.0,
            0.0,
            rect.height() as f32 / pixmap.height() as f32,
            r.x() as f32,
            r.y() as f32,
        );

        let pattern =
            tiny_skia::Pattern::new(pixmap.as_ref(), tiny_skia::SpreadMode::Pad, filter, 1.0, ts);
        let mut paint = tiny_skia::Paint::default();
        paint.shader = pattern;

        if view_box.aspect.slice {
            let r = view_box.rect;
            let rect = tiny_skia::Rect::from_xywh(
                r.x() as f32,
                r.y() as f32,
                r.width() as f32,
                r.height() as f32,
            )?;

            canvas.set_clip_rect(rect);
        }

        canvas
            .pixmap
            .fill_rect(rect, &paint, canvas.transform, canvas.clip.as_ref());
        canvas.clip = None;

        Some(())
    }

    fn image_to_pixmap(image: &Image, pixmap: &mut [u8]) {
        use rgb::FromSlice;

        let mut i = 0;
        match &image.data {
            ImageData::RGB(data) => {
                for p in data.as_rgb() {
                    pixmap[i + 0] = p.r;
                    pixmap[i + 1] = p.g;
                    pixmap[i + 2] = p.b;
                    pixmap[i + 3] = 255;

                    i += tiny_skia::BYTES_PER_PIXEL;
                }
            }
            ImageData::RGBA(data) => {
                for p in data.as_rgba() {
                    let a = p.a as f64 / 255.0;
                    pixmap[i + 0] = (p.r as f64 * a + 0.5) as u8;
                    pixmap[i + 1] = (p.g as f64 * a + 0.5) as u8;
                    pixmap[i + 2] = (p.b as f64 * a + 0.5) as u8;
                    pixmap[i + 3] = p.a;

                    i += tiny_skia::BYTES_PER_PIXEL;
                }
            }
        }
    }

    pub struct Image {
        data: ImageData,
        size: usvg::ScreenSize,
    }

    enum ImageData {
        RGB(Vec<u8>),
        RGBA(Vec<u8>),
    }

    pub fn read_png(data: &[u8]) -> Option<Image> {
        let mut decoder = png::Decoder::new(data);
        decoder.set_transformations(png::Transformations::normalize_to_color8());
        let mut reader = decoder.read_info().ok()?;
        let mut img_data = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut img_data).ok()?;

        let size = usvg::ScreenSize::new(info.width, info.height)?;

        let data = match info.color_type {
            png::ColorType::Rgb => ImageData::RGB(img_data),
            png::ColorType::Rgba => ImageData::RGBA(img_data),
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
                log::warn!("Indexed PNG is not supported.");
                return None;
            }
        };

        Some(Image { data, size })
    }

    pub fn read_jpeg(data: &[u8]) -> Option<Image> {
        let mut decoder = jpeg_decoder::Decoder::new(data);
        let img_data = decoder.decode().ok()?;
        let info = decoder.info()?;

        let size = usvg::ScreenSize::new(info.width as u32, info.height as u32)?;

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

        Some(Image { data, size })
    }

    pub fn read_gif(data: &[u8]) -> Option<Image> {
        let mut decoder = gif::DecodeOptions::new();
        decoder.set_color_output(gif::ColorOutput::RGBA);
        let mut decoder = decoder.read_info(data).ok()?;
        let first_frame = decoder.read_next_frame().ok()??;

        let size =
            usvg::ScreenSize::new(u32::from(first_frame.width), u32::from(first_frame.height))?;

        Some(Image {
            data: ImageData::RGBA(first_frame.buffer.to_vec()),
            size,
        })
    }

    /// Calculates an image rect depending on the provided view box.
    fn image_rect(view_box: &usvg::ViewBox, img_size: usvg::ScreenSize) -> usvg::Rect {
        let new_size = img_size.to_size().fit_view_box(view_box);
        let (x, y) = usvg::utils::aligned_pos(
            view_box.aspect.align,
            view_box.rect.x(),
            view_box.rect.y(),
            view_box.rect.width() - new_size.width(),
            view_box.rect.height() - new_size.height(),
        );

        new_size.to_rect(x, y)
    }
}
