// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::render::TinySkiaPixmapMutExt;
use crate::tree::{BBoxes, ConvTransform, Node, Tree};
use crate::IntSize;

pub enum ImageKind {
    #[cfg(feature = "raster-images")]
    Raster(tiny_skia::Pixmap),
    Vector(Tree),
}

pub struct Image {
    pub transform: tiny_skia::Transform,
    pub view_box: usvg::ViewBox,
    pub quality: tiny_skia::FilterQuality,
    pub kind: ImageKind,
}

pub fn convert(image: &usvg::Image, children: &mut Vec<Node>) -> Option<BBoxes> {
    let object_bbox = image.view_box.rect.to_path_bbox();
    let bboxes = BBoxes {
        object: object_bbox,
        transformed_object: object_bbox.transform(&image.transform)?,
        layer: object_bbox,
    };

    if image.visibility != usvg::Visibility::Visible {
        return Some(bboxes);
    }

    let mut quality = tiny_skia::FilterQuality::Bicubic;
    if image.rendering_mode == usvg::ImageRendering::OptimizeSpeed {
        quality = tiny_skia::FilterQuality::Nearest;
    }

    let kind = match image.kind {
        usvg::ImageKind::SVG(ref utree) => ImageKind::Vector(Tree::from_usvg(utree)),
        #[cfg(feature = "raster-images")]
        _ => ImageKind::Raster(raster_images::decode_raster(image)?),
        #[cfg(not(feature = "raster-images"))]
        _ => {
            log::warn!("Images decoding was disabled by a build feature.");
            return None;
        }
    };

    children.push(Node::Image(Image {
        transform: image.transform.to_native(),
        view_box: image.view_box,
        quality,
        kind,
    }));

    Some(bboxes)
}

pub fn render_image(
    image: &Image,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) {
    match image.kind {
        #[cfg(feature = "raster-images")]
        ImageKind::Raster(ref raster) => {
            raster_images::render_raster(image, raster, transform, pixmap);
        }
        ImageKind::Vector(ref rtree) => {
            render_vector(image, rtree, transform, pixmap);
        }
    }
}

fn render_vector(
    image: &Image,
    tree: &Tree,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) -> Option<()> {
    let img_size = IntSize::from_usvg(tree.size);
    let (ts, clip) = crate::geom::view_box_to_transform_with_clip(&image.view_box, img_size);

    let mut sub_pixmap = tiny_skia::Pixmap::new(pixmap.width(), pixmap.height()).unwrap();

    let source_transform = transform;
    let transform = transform
        .pre_concat(image.transform)
        .pre_concat(ts.to_native());

    tree.render(transform, &mut sub_pixmap.as_mut());

    let mask = if let Some(clip) = clip {
        let rr = tiny_skia::Rect::from_xywh(
            clip.x() as f32,
            clip.y() as f32,
            clip.width() as f32,
            clip.height() as f32,
        )?;
        pixmap.create_rect_mask(source_transform, rr)
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
    use super::Image;
    use crate::render::TinySkiaPixmapMutExt;
    use crate::tree::OptionLog;
    use crate::IntSize;

    pub fn decode_raster(image: &usvg::Image) -> Option<tiny_skia::Pixmap> {
        match image.kind {
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

        let size = IntSize::new(info.width as u32, info.height as u32)?;

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

        let size = IntSize::new(u32::from(first_frame.width), u32::from(first_frame.height))?;

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
        image: &Image,
        raster: &tiny_skia::Pixmap,
        transform: tiny_skia::Transform,
        pixmap: &mut tiny_skia::PixmapMut,
    ) -> Option<()> {
        let img_size = IntSize::new(raster.width(), raster.height())?;
        let r = image_rect(&image.view_box, img_size);
        let rect = tiny_skia::Rect::from_xywh(
            r.x() as f32,
            r.y() as f32,
            r.width() as f32,
            r.height() as f32,
        )?;

        let ts = tiny_skia::Transform::from_row(
            rect.width() / raster.width() as f32,
            0.0,
            0.0,
            rect.height() / raster.height() as f32,
            r.x() as f32,
            r.y() as f32,
        );

        let pattern = tiny_skia::Pattern::new(
            raster.as_ref(),
            tiny_skia::SpreadMode::Pad,
            image.quality,
            1.0,
            ts,
        );
        let mut paint = tiny_skia::Paint::default();
        paint.shader = pattern;

        let mask = if image.view_box.aspect.slice {
            let r = image.view_box.rect;
            let rect = tiny_skia::Rect::from_xywh(
                r.x() as f32,
                r.y() as f32,
                r.width() as f32,
                r.height() as f32,
            )?;

            pixmap.create_rect_mask(transform, rect)
        } else {
            None
        };

        let transform = transform.pre_concat(image.transform);
        pixmap.fill_rect(rect, &paint, transform, mask.as_ref());

        Some(())
    }

    /// Calculates an image rect depending on the provided view box.
    fn image_rect(view_box: &usvg::ViewBox, img_size: IntSize) -> usvg::Rect {
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
