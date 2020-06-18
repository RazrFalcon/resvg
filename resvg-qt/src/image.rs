// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use usvg::{Rect, FitTo, ScreenSize};
use crate::{qt, ConvTransform, Options};


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
        draw_raster(&image.data, image.view_box, image.rendering_mode, opt, p);
    }

    image.view_box.rect
}

pub fn draw_raster(
    data: &usvg::ImageData,
    view_box: usvg::ViewBox,
    rendering_mode: usvg::ImageRendering,
    opt: &Options,
    p: &mut qt::Painter,
) {
    let img = match data {
        usvg::ImageData::Path(ref path) => {
            let path = opt.usvg.get_abs_path(path);
            try_opt_warn!(
                qt::Image::from_file(&path),
                "Failed to load an external image: {:?}.", path
            )
        }
        usvg::ImageData::Raw(ref data) => {
            try_opt_warn!(
                qt::Image::from_data(data),
                "Failed to load an embedded image."
            )
        }
    };

    let img_size = try_opt!(ScreenSize::new(img.width(), img.height()));

    if rendering_mode == usvg::ImageRendering::OptimizeSpeed {
        p.set_smooth_pixmap_transform(false);
    }

    if view_box.aspect.slice {
        let r = view_box.rect;
        p.set_clip_rect(r.x(), r.y(), r.width(), r.height());
    }

    let r = image_rect(&view_box, img_size);
    p.draw_image_rect(r.x(), r.y(), r.width(), r.height(), &img);

    // Revert.
    p.set_smooth_pixmap_transform(true);
    p.reset_clip_path();
}

pub fn draw_svg(
    data: &usvg::ImageData,
    view_box: usvg::ViewBox,
    opt: &Options,
    p: &mut qt::Painter,
) {
    let (tree, sub_opt) = try_opt!(data.load_svg(&opt.usvg));

    let sub_opt = Options {
        usvg: sub_opt,
        fit_to: FitTo::Original,
        background: None,
    };

    let img_size = tree.svg_node().size.to_screen_size();
    let (ts, clip) = usvg::utils::view_box_to_transform_with_clip(&view_box, img_size);

    if let Some(clip) = clip {
        p.set_clip_rect(clip.x(), clip.y(), clip.width(), clip.height());
    }

    p.apply_transform(&ts.to_native());
    super::render_to_canvas(&tree, &sub_opt, img_size, p);
    p.reset_clip_path();
}

/// Calculates an image rect depending on the provided view box.
fn image_rect(
    view_box: &usvg::ViewBox,
    img_size: ScreenSize,
) -> Rect {
    let new_size = img_size.fit_view_box(view_box);
    let (x, y) = usvg::utils::aligned_pos(
        view_box.aspect.align,
        view_box.rect.x(),
        view_box.rect.y(),
        view_box.rect.width() - new_size.width() as f64,
        view_box.rect.height() - new_size.height() as f64,
    );

    new_size.to_size().to_rect(x, y)
}
