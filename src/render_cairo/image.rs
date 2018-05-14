// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use cairo;
use piston_image::{
    self,
    GenericImage,
};
use usvg;

// self
use geom::*;
use utils;
use traits::{
    ConvTransform,
};
use {
    Options,
};


pub fn draw(
    image: &usvg::Image,
    opt: &Options,
    cr: &cairo::Context,
) -> Rect {
    if image.format == usvg::ImageFormat::SVG {
        draw_svg(image, opt, cr);
    } else {
        draw_raster(image, cr);
    }

    image.view_box.rect
}

pub fn draw_raster(
    image: &usvg::Image,
    cr: &cairo::Context,
) {
    let r = image.view_box.rect;

    let img = match image.data {
        usvg::ImageData::Path(ref path) => {
            try_opt_warn!(piston_image::open(path).ok(), (),
                "Failed to load an external image: {:?}.", path)
        }
        usvg::ImageData::Raw(ref data) => {
            try_opt_warn!(piston_image::load_from_memory(data).ok(), (),
                "Failed to load an embedded image.")
        }
    };

    let new_size = utils::apply_view_box(
        &image.view_box,
        ScreenSize::new(img.width(), img.height()),
    );
    let img = img.resize_exact(new_size.width, new_size.height, piston_image::FilterType::Triangle);
    let img = img.to_rgba();

    let mut surface = try_create_surface!(new_size, ());

    {
        // Scaled image will be bigger than viewbox, so we have to
        // cut only the part specified by align rule.
        let (start_x, start_y, end_x, end_y) = if image.view_box.aspect.slice {
            let pos = utils::aligned_pos(
                image.view_box.aspect.align,
                0.0, 0.0, new_size.width as f64 - r.width(), new_size.height as f64 - r.height(),
            );

            (pos.x as u32, pos.y as u32, (pos.x + r.width()) as u32, (pos.y + r.height()) as u32)
        } else {
            (0, 0, img.width(), img.height())
        };

        let mut surface_data = surface.get_data().unwrap();

        let mut i = 0;
        let mut x = 0;
        let mut y = 0;
        for p in img.chunks(4) {
            if x >= start_x && y >= start_y && x <= end_x && y <= end_y {
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
            }

            i += 4;
            x += 1;
            if x == img.width() {
                x = 0;
                y += 1;
            }
        }
    }

    let pos = utils::aligned_pos(
        image.view_box.aspect.align,
        r.x(), r.y(), r.width() - img.width() as f64, r.height() - img.height() as f64,
    );

    cr.set_source_surface(&surface, pos.x, pos.y);
    cr.paint();
}

fn draw_svg(
    image: &usvg::Image,
    opt: &Options,
    cr: &cairo::Context,
) {
    let (tree, sub_opt) = try_opt!(utils::load_sub_svg(image, opt), ());

    let img_size = tree.svg_node().size.to_screen_size();
    let (ts, clip) = utils::prepare_sub_svg_geom(image, img_size);

    if let Some(clip) = clip {
        cr.rectangle(clip.x(), clip.y(), clip.width(), clip.height());
        cr.clip();
    }

    cr.transform(ts.to_native());
    super::render_to_canvas(&tree, &sub_opt, img_size, cr);
    cr.reset_clip();
}
