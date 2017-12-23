// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use cairo;
use image;

use dom;
use math::{
    Rect,
};


pub fn draw(
    image: &dom::Image,
    cr: &cairo::Context,
) -> Rect {
    let img = match image.data {
        dom::ImageData::Path(ref path) => {
            match image::open(path) {
                Ok(v) => v,
                Err(_) => {
                    warn!("Failed to load an external image: {:?}.", path);
                    return image.rect;
                }
            }
        }
        dom::ImageData::Raw(ref data, _) => {
            match image::load_from_memory(data) {
                Ok(v) => v,
                Err(_) => {
                    warn!("Failed to load an embedded image.");
                    return image.rect;
                }
            }
        }
    };

    let img = img.resize(
        image.rect.w as u32,
        image.rect.h as u32,
        image::FilterType::Lanczos3
    );
    let img = img.to_rgba();

    let surface = cairo::ImageSurface::create(
        cairo::Format::ARgb32,
        img.width() as i32,
        img.height() as i32
    );

    let mut surface = match surface {
        Ok(v) => v,
        Err(_) => {
            warn!("Failed to create a surface for bitmap image.");
            return image.rect;
        }
    };

    {
        let mut surface_data = surface.get_data().unwrap();

        let mut i = 0;
        for p in img.chunks(4) {
            let r = p[0] as u32;
            let g = p[1] as u32;
            let b = p[2] as u32;
            let a = p[3] as u32;

            // https://www.cairographics.org/manual/cairo-Image-Surfaces.html#cairo-format-t
            let tr = a * r + 0x80;
            let tg = a * g + 0x80;
            let tb = a * b + 0x80;
            surface_data[i + 0] = (((tb >> 8) + tb) >> 8) as u8;
            surface_data[i + 1] = (((tg >> 8) + tg) >> 8) as u8;
            surface_data[i + 2] = (((tr >> 8) + tr) >> 8) as u8;
            surface_data[i + 3] = a as u8;

            i += 4;
        }
    }

    cr.set_source_surface(&surface, image.rect.x, image.rect.y);
    cr.paint();

    image.rect
}
