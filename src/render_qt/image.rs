// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use qt;
use usvg;

// self
use geom::*;
use utils;

pub fn draw(
    image: &usvg::Image,
    p: &qt::Painter,
) -> Rect {
    let r = image.view_box.rect;

    let img = match image.data {
        usvg::ImageData::Path(ref path) => {
            try_opt_warn!(qt::Image::from_file(path), r,
                "Failed to load an external image: {:?}.", path)
        }
        usvg::ImageData::Raw(ref data, _) => {
            try_opt_warn!(qt::Image::from_data(data), r,
                "Failed to load an embedded image.")
        }
    };

    let new_size = utils::apply_view_box(
        &image.view_box,
        ScreenSize::new(img.width(), img.height()),
    );

    let img = try_opt_warn!(
        img.resize(new_size.width, new_size.height, qt::AspectRatioMode::IgnoreAspectRatio), r,
        "Failed to scale an image.",
    );

    if image.view_box.aspect.slice {
        // Scaled image will be bigger than viewbox, so we have to
        // cut only the part specified by align rule.

        let pos = utils::aligned_pos(
            image.view_box.aspect.align,
            0.0, 0.0, new_size.width as f64 - r.width(), new_size.height as f64 - r.height(),
        );

        let img = try_opt_warn!(
            img.copy(pos.x as u32, pos.y as u32, r.width() as u32, r.height() as u32), r,
            "Failed to copy a part of an image."
        );

        p.draw_image(r.x(), r.y(), &img);
    } else {
        let pos = utils::aligned_pos(
            image.view_box.aspect.align,
            r.x(), r.y(), r.width() - new_size.width as f64, r.height() - new_size.height as f64,
        );

        p.draw_image(pos.x, pos.y, &img);
    }

    r
}
