// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use qt;
use usvg;

// self
use super::prelude::*;
use backend_utils::image;


pub fn draw(
    image: &usvg::Image,
    opt: &Options,
    p: &mut qt::Painter,
) -> Rect {
    if image.visibility != usvg::Visibility::Visible {
        return image.view_box.rect;
    }

    if image.format == usvg::ImageFormat::SVG {
        draw_svg(&image.data, image.view_box, opt, p);
    } else {
        draw_raster(&image.data, image.view_box, opt, p);
    }

    image.view_box.rect
}

pub fn draw_raster(
    data: &usvg::ImageData,
    mut view_box: usvg::ViewBox,
    opt: &Options,
    p: &mut qt::Painter,
) {
    let img = match data {
        usvg::ImageData::Path(ref path) => {
            let path = image::get_abs_path(path, opt);
            try_opt_warn!(qt::Image::from_file(&path), (),
                "Failed to load an external image: {:?}.", path)
        }
        usvg::ImageData::Raw(ref data) => {
            try_opt_warn!(qt::Image::from_data(data), (),
                "Failed to load an embedded image.")
        }
    };

    let img_size = ScreenSize::new(img.width(), img.height());
    image::prepare_image_viewbox(img_size, &mut view_box);
    let r = view_box.rect;

    let new_size = utils::apply_view_box(&view_box, img_size);

    let img = try_opt_warn!(
        img.resize(new_size.width, new_size.height, qt::AspectRatioMode::Ignore), (),
        "Failed to scale an image.",
    );

    if view_box.aspect.slice {
        // Scaled image will be bigger than viewbox, so we have to
        // cut only the part specified by align rule.

        let pos = utils::aligned_pos(
            view_box.aspect.align,
            0.0, 0.0, new_size.width as f64 - r.width, new_size.height as f64 - r.height,
        );

        let img = try_opt_warn!(
            img.copy(pos.x as u32, pos.y as u32, r.width as u32, r.height as u32), (),
            "Failed to copy a part of an image."
        );

        p.draw_image(r.x, r.y, &img);
    } else {
        let pos = utils::aligned_pos(
            view_box.aspect.align,
            r.x, r.y, r.width - new_size.width as f64, r.height - new_size.height as f64,
        );

        p.draw_image(pos.x, pos.y, &img);
    }
}

pub fn draw_svg(
    data: &usvg::ImageData,
    view_box: usvg::ViewBox,
    opt: &Options,
    p: &mut qt::Painter,
) {
    let (tree, sub_opt) = try_opt!(image::load_sub_svg(data, opt), ());

    let img_size = tree.svg_node().size.to_screen_size();
    let (ts, clip) = image::prepare_sub_svg_geom(view_box, img_size);

    if let Some(clip) = clip {
        p.set_clip_rect(clip.x, clip.y, clip.width, clip.height);
    }

    p.apply_transform(&ts.to_native());
    super::render_to_canvas(&tree, &sub_opt, img_size, p);
    p.reset_clip_path();
}
