// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use usvg::try_opt;

use crate::prelude::*;
use crate::image;
use crate::ConvTransform;
use super::RaqoteDrawTargetExt;


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
    let img = try_opt!(image::load_raster(format, data, opt));

    let sub_dt = {
        let mut sub_dt = raqote::DrawTarget::new(img.size.width() as i32, img.size.height() as i32);
        let surface_data = sub_dt.get_data_u8_mut();
        image_to_surface(&img, surface_data);
        sub_dt
    };

    let (ts, clip) = image::prepare_sub_svg_geom(view_box, img.size);

    let mut pb = raqote::PathBuilder::new();
    if let Some(clip) = clip {
        pb.rect(clip.x() as f32, clip.y() as f32, clip.width() as f32, clip.height() as f32);
    } else {
        // We have to clip the image before rendering because we use `Extend::Pad`.
        let r = image::image_rect(&view_box, img.size);
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

fn image_to_surface(image: &image::Image, surface: &mut [u8]) {
    // Surface is always ARGB.
    const SURFACE_CHANNELS: usize = 4;

    use crate::image::ImageData;
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
    let (tree, sub_opt) = try_opt!(image::load_sub_svg(data, opt));

    let img_size = tree.svg_node().size.to_screen_size();
    let (ts, clip) = image::prepare_sub_svg_geom(view_box, img_size);

    if let Some(clip) = clip {
        let mut pb = raqote::PathBuilder::new();

        pb.rect(clip.x() as f32, clip.y() as f32, clip.width() as f32, clip.height() as f32);
        dt.push_clip(&pb.finish());
    }

    dt.transform(&ts.to_native());
    super::render_to_canvas(&tree, &sub_opt, img_size, dt);

    if let Some(_) = clip {
        dt.pop_clip();
    }
}
