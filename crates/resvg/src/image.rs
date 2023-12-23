// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::render::TinySkiaPixmapMutExt;
use crate::tree::{Node, Tree};

pub enum ImageKind {
    #[cfg(feature = "raster-images")]
    Raster(tiny_skia::Pixmap),
    Vector(Tree),
}

pub struct Image {
    pub view_box: usvg::ViewBox,
    pub quality: tiny_skia::FilterQuality,
    pub kind: ImageKind,
}

pub fn convert(image: &usvg::Image, children: &mut Vec<Node>) -> Option<usvg::BBox> {
    let object_bbox = image.bounding_box?.to_rect();
    let layer_bbox = usvg::BBox::from(object_bbox);

    if image.visibility != usvg::Visibility::Visible {
        return Some(layer_bbox);
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
        view_box: image.view_box,
        quality,
        kind,
    }));

    Some(layer_bbox)
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
    let img_size = tree.size.to_int_size();
    let (ts, clip) = crate::geom::view_box_to_transform_with_clip(&image.view_box, img_size);

    let mut sub_pixmap = tiny_skia::Pixmap::new(pixmap.width(), pixmap.height()).unwrap();

    let source_transform = transform;
    let transform = transform.pre_concat(ts);

    tree.render(transform, &mut sub_pixmap.as_mut());

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
    use image::{Rgb, Rgba};
    use super::Image;
    use crate::render::TinySkiaPixmapMutExt;
    use crate::tree::OptionLog;

    pub fn decode_raster(image: &usvg::Image) -> Option<tiny_skia::Pixmap> {
        match image.kind {
            usvg::ImageKind::SVG(_) => None,
            usvg::ImageKind::JPEG(ref data) => {
                decode(data).log_none(|| log::warn!("Failed to decode a JPEG image."))
            }
            usvg::ImageKind::PNG(ref data) => {
                decode_png(data).log_none(|| log::warn!("Failed to decode a PNG image."))
            }
            usvg::ImageKind::GIF(ref data) => {
                // decode_gif(data).log_none(|| log::warn!("Failed to decode a GIF image."))
                None
            }
            _ => None
        }
    }

    fn decode_png(data: &[u8]) -> Option<tiny_skia::Pixmap> {
        tiny_skia::Pixmap::decode_png(data).ok()
    }

    fn decode(data: &[u8]) -> Option<tiny_skia::Pixmap> {
        let dynamic_image = image::load_from_memory(data).ok()?;
        let size = tiny_skia::IntSize::from_wh(dynamic_image.width(), dynamic_image.height())?;
        let res: Vec<u8> = dynamic_image.to_rgb8().pixels().flat_map(|&Rgb(c)| c).collect();

        let (w, h) = size.dimensions();
        let mut pixmap = tiny_skia::Pixmap::new(w, h)?;
        rgb_to_pixmap(&res, &mut pixmap);
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
        let img_size = tiny_skia::IntSize::from_wh(raster.width(), raster.height())?;
        let rect = image_rect(&image.view_box, img_size);

        let ts = tiny_skia::Transform::from_row(
            rect.width() / raster.width() as f32,
            0.0,
            0.0,
            rect.height() / raster.height() as f32,
            rect.x(),
            rect.y(),
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
            pixmap.create_rect_mask(transform, image.view_box.rect.to_rect())
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
