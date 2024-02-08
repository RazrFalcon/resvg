// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::render::TinySkiaPixmapMutExt;

pub fn render(
    image: &usvg::Image,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) {
    if image.visibility() != usvg::Visibility::Visible {
        return;
    }

    render_inner(
        image.kind(),
        image.view_box(),
        transform,
        image.rendering_mode(),
        pixmap,
    );
}

pub fn render_inner(
    image_kind: &usvg::ImageKind,
    view_box: usvg::ViewBox,
    transform: tiny_skia::Transform,
    #[allow(unused_variables)] rendering_mode: usvg::ImageRendering,
    pixmap: &mut tiny_skia::PixmapMut,
) {
    match image_kind {
        usvg::ImageKind::SVG(ref tree) => {
            render_vector(tree, &view_box, transform, pixmap);
        }
        #[cfg(feature = "raster-images")]
        _ => {
            raster_images::render_raster(image_kind, view_box, transform, rendering_mode, pixmap);
        }
        #[cfg(not(feature = "raster-images"))]
        _ => {
            log::warn!("Images decoding was disabled by a build feature.");
        }
    }
}

fn render_vector(
    tree: &usvg::Tree,
    view_box: &usvg::ViewBox,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) -> Option<()> {
    let img_size = tree.size().to_int_size();
    let (ts, clip) = crate::geom::view_box_to_transform_with_clip(&view_box, img_size);

    let mut sub_pixmap = tiny_skia::Pixmap::new(pixmap.width(), pixmap.height()).unwrap();

    let source_transform = transform;
    let transform = transform.pre_concat(ts);

    crate::render(tree, transform, &mut sub_pixmap.as_mut());

    let mask = if let Some(clip) = clip {
        pixmap.create_rect_mask(source_transform, clip.to_rect())
    } else {
        None
    };

    pixmap.draw_pixmap(
        0,
        0,
        sub_pixmap.as_ref(),
        &tiny_skia::PixmapPaint::default(),
        tiny_skia::Transform::identity(),
        mask.as_ref(),
    );

    Some(())
}

#[cfg(feature = "raster-images")]
mod raster_images {
    use crate::render::TinySkiaPixmapMutExt;
    use crate::OptionLog;

    fn decode_raster(image: &usvg::ImageKind) -> Option<tiny_skia::Pixmap> {
        match image {
            usvg::ImageKind::SVG(_) => None,
            usvg::ImageKind::JPEG(ref data) => {
                decode_jpeg(data).log_none(|| log::warn!("Failed to decode a JPEG image."))
            }
            usvg::ImageKind::PNG(ref data) => {
                decode_png(data).log_none(|| log::warn!("Failed to decode a PNG image."))
            }
            usvg::ImageKind::GIF(ref data) => {
                decode_gif(data).log_none(|| log::warn!("Failed to decode a GIF image."))
            }
        }
    }

    fn decode_png(data: &[u8]) -> Option<tiny_skia::Pixmap> {
        tiny_skia::Pixmap::decode_png(data).ok()
    }

    fn decode_jpeg(data: &[u8]) -> Option<tiny_skia::Pixmap> {
        let mut decoder = jpeg_decoder::Decoder::new(data);
        let img_data = decoder.decode().ok()?;
        let info = decoder.info()?;

        let size = tiny_skia::IntSize::from_wh(info.width as u32, info.height as u32)?;

        let data = match info.pixel_format {
            jpeg_decoder::PixelFormat::RGB24 => img_data,
            jpeg_decoder::PixelFormat::L8 => {
                let mut rgb_data: Vec<u8> = Vec::with_capacity(img_data.len() * 3);
                for gray in img_data {
                    rgb_data.push(gray);
                    rgb_data.push(gray);
                    rgb_data.push(gray);
                }

                rgb_data
            }
            _ => return None,
        };

        let (w, h) = size.dimensions();
        let mut pixmap = tiny_skia::Pixmap::new(w, h)?;
        rgb_to_pixmap(&data, &mut pixmap);
        Some(pixmap)
    }

    fn decode_gif(data: &[u8]) -> Option<tiny_skia::Pixmap> {
        let mut decoder = gif::DecodeOptions::new();
        decoder.set_color_output(gif::ColorOutput::RGBA);
        let mut decoder = decoder.read_info(data).ok()?;
        let first_frame = decoder.read_next_frame().ok()??;

        let size = tiny_skia::IntSize::from_wh(
            u32::from(first_frame.width),
            u32::from(first_frame.height),
        )?;

        let (w, h) = size.dimensions();
        let mut pixmap = tiny_skia::Pixmap::new(w, h)?;
        rgba_to_pixmap(&first_frame.buffer, &mut pixmap);
        Some(pixmap)
    }

    fn rgb_to_pixmap(data: &[u8], pixmap: &mut tiny_skia::Pixmap) {
        use rgb::FromSlice;

        let mut i = 0;
        let dst = pixmap.data_mut();
        for p in data.as_rgb() {
            dst[i + 0] = p.r;
            dst[i + 1] = p.g;
            dst[i + 2] = p.b;
            dst[i + 3] = 255;

            i += tiny_skia::BYTES_PER_PIXEL;
        }
    }

    fn rgba_to_pixmap(data: &[u8], pixmap: &mut tiny_skia::Pixmap) {
        use rgb::FromSlice;

        let mut i = 0;
        let dst = pixmap.data_mut();
        for p in data.as_rgba() {
            let a = p.a as f64 / 255.0;
            dst[i + 0] = (p.r as f64 * a + 0.5) as u8;
            dst[i + 1] = (p.g as f64 * a + 0.5) as u8;
            dst[i + 2] = (p.b as f64 * a + 0.5) as u8;
            dst[i + 3] = p.a;

            i += tiny_skia::BYTES_PER_PIXEL;
        }
    }

    pub(crate) fn render_raster(
        image: &usvg::ImageKind,
        view_box: usvg::ViewBox,
        transform: tiny_skia::Transform,
        rendering_mode: usvg::ImageRendering,
        pixmap: &mut tiny_skia::PixmapMut,
    ) -> Option<()> {
        let raster = decode_raster(image)?;

        let img_size = tiny_skia::IntSize::from_wh(raster.width(), raster.height())?;
        let rect = image_rect(&view_box, img_size);

        let ts = tiny_skia::Transform::from_row(
            rect.width() / raster.width() as f32,
            0.0,
            0.0,
            rect.height() / raster.height() as f32,
            rect.x(),
            rect.y(),
        );

        let mut quality = tiny_skia::FilterQuality::Bicubic;
        if rendering_mode == usvg::ImageRendering::OptimizeSpeed {
            quality = tiny_skia::FilterQuality::Nearest;
        }

        let pattern = tiny_skia::Pattern::new(
            raster.as_ref(),
            tiny_skia::SpreadMode::Pad,
            quality,
            1.0,
            ts,
        );
        let mut paint = tiny_skia::Paint::default();
        paint.shader = pattern;

        let mask = if view_box.aspect.slice {
            pixmap.create_rect_mask(transform, view_box.rect.to_rect())
        } else {
            None
        };

        pixmap.fill_rect(rect.to_rect(), &paint, transform, mask.as_ref());

        Some(())
    }

    /// Calculates an image rect depending on the provided view box.
    fn image_rect(
        view_box: &usvg::ViewBox,
        img_size: tiny_skia::IntSize,
    ) -> tiny_skia::NonZeroRect {
        let new_size = crate::geom::fit_view_box(img_size.to_size(), view_box);
        let (x, y) = usvg::utils::aligned_pos(
            view_box.aspect.align,
            view_box.rect.x(),
            view_box.rect.y(),
            view_box.rect.width() - new_size.width(),
            view_box.rect.height() - new_size.height(),
        );

        new_size.to_non_zero_rect(x, y)
    }
}
