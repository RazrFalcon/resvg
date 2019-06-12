// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use image::GenericImageView;
use usvg::{try_opt, try_opt_warn};

use crate::{prelude::*, backend_utils, backend_utils::ConvTransform};


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
        draw_raster(&image.data, image.view_box, image.rendering_mode, opt, dt);
    }

    image.view_box.rect
}

pub fn draw_raster(
    data: &usvg::ImageData,
    view_box: usvg::ViewBox,
    rendering_mode: usvg::ImageRendering,
    opt: &Options,
    dt: &mut raqote::DrawTarget,
) {
    let img = match data {
        usvg::ImageData::Path(ref path) => {
            let path = backend_utils::image::get_abs_path(path, opt);
            try_opt_warn!(
                image::open(path.clone()).ok(),
                "Failed to load an external image: {:?}.", path
            )
        }
        usvg::ImageData::Raw(ref data) => {
            match image::load_from_memory(data) {
                Ok(img) => img,
                Err(e) => {
                    log::warn!("{}", e.to_string());
                    return;
                }
            }
        }
    };

    let img_size = img.dimensions();
    let img_size = ScreenSize::new(img_size.0, img_size.1);
    let img_size = try_opt!(img_size);

    let surface = try_opt!(image_to_surface(img, img_size));

    let (ts, clip) = backend_utils::image::prepare_sub_svg_geom(view_box, img_size);

    let mut pb = raqote::PathBuilder::new();
    if let Some(clip) = clip {
        pb.rect(clip.x() as f32, clip.y() as f32, clip.width() as f32, clip.height() as f32);
    } else {
        // We have to clip the image before rendering because we use `Extend::Pad`.
        let r = backend_utils::image::image_rect(&view_box, img_size);
        pb.rect(r.x() as f32, r.y() as f32, r.width() as f32, r.height() as f32);
    }

    let img = raqote::Image {
        width: surface.width() as i32,
        height: surface.height() as i32,
        data: surface.get_data(),
    };


    let t: raqote::Transform = ts.to_native();
    let patt = raqote::Source::Image(img, raqote::ExtendMode::Repeat, t.inverse().unwrap());


    dt.fill(&pb.finish(), &patt, &raqote::DrawOptions::default());
}

fn image_to_surface(
    img: image::DynamicImage,
    img_size: ScreenSize,
) -> Option<raqote::DrawTarget> {
    let mut surface = raqote::DrawTarget::new(img_size.width() as i32, img_size.height() as i32);
    let surface_data = surface.get_data_u8_mut();

    let w = img.dimensions().0 as u32;
    let h = img.dimensions().1 as u32;
    let pixels = img.to_rgba();
    // We can't iterate over pixels directly, because width may not be equal to stride.
    let mut i = 0;
    for y in 0..h {
        for x in 0..w {
            let pixel = pixels[(x,y)];
            let r = pixel[0] as u32;
            let g = pixel[1] as u32;
            let b = pixel[2] as u32;
            let a = pixel[3] as u32;

            let tr = a * r + 0x80;
            let tg = a * g + 0x80;
            let tb = a * b + 0x80;
            surface_data[i + 0] = (((tb >> 8) + tb) >> 8) as u8;
            surface_data[i + 1] = (((tg >> 8) + tg) >> 8) as u8;
            surface_data[i + 2] = (((tr >> 8) + tr) >> 8) as u8;
            surface_data[i + 3] = a as u8; // TODO: is needed?

            // Surface is always ARGB.
            i += 4;
        }
    }

    Some(surface)
}

pub fn draw_svg(
    data: &usvg::ImageData,
    view_box: usvg::ViewBox,
    opt: &Options,
    dt: &mut raqote::DrawTarget,
) {
    let (tree, sub_opt) = try_opt!(backend_utils::image::load_sub_svg(data, opt));

    let img_size = tree.svg_node().size.to_screen_size();
    let (ts, clip) = backend_utils::image::prepare_sub_svg_geom(view_box, img_size);

    if let Some(clip) = clip {
        let mut pb = raqote::PathBuilder::new();

        pb.rect(clip.x() as f32, clip.y() as f32, clip.width() as f32, clip.height() as f32);
        dt.push_clip(&pb.finish());
    }

    let ctm = dt.get_transform().pre_mul(&ts.to_native());
    dt.set_transform(&ctm);
    super::render_to_canvas(&tree, &sub_opt, img_size, dt);

    if let Some(_) = clip {
        dt.pop_clip();
    }
}
