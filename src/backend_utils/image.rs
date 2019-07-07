// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::{fs, path};

use log::warn;

use crate::prelude::*;


pub struct Image {
    pub data: ImageData,
    pub size: ScreenSize,
}

pub enum ImageData {
    RGB(Vec<u8>),
    RGBA(Vec<u8>),
}

pub fn load_raster(
    format: usvg::ImageFormat,
    data: &usvg::ImageData,
    opt: &Options,
) -> Option<Image> {
    let img = _load_raster(format, data, opt);

    if img.is_none() {
        match data {
            usvg::ImageData::Path(ref path) => {
                let path = get_abs_path(path, opt);
                warn!("Failed to load an external image: {:?}.", path);
            }
            usvg::ImageData::Raw(_) => {
                warn!("Failed to load an embedded image.");
            }
        }
    }

    img
}

fn _load_raster(
    format: usvg::ImageFormat,
    data: &usvg::ImageData,
    opt: &Options,
) -> Option<Image> {
    debug_assert!(format != usvg::ImageFormat::SVG);

    match data {
        usvg::ImageData::Path(ref path) => {
            let path = get_abs_path(path, opt);
            let file = fs::File::open(path).ok()?;

            if format == usvg::ImageFormat::JPEG {
                read_jpeg(file)
            } else {
                read_png(file)
            }
        }
        usvg::ImageData::Raw(ref data) => {
            if format == usvg::ImageFormat::JPEG {
                read_jpeg(data.as_slice())
            } else {
                read_png(data.as_slice())
            }
        }
    }
}

fn read_png<R: std::io::Read>(r: R) -> Option<Image> {
    let decoder = png::Decoder::new(r);
    let (info, mut reader) = decoder.read_info().ok()?;

    match info.color_type {
        png::ColorType::RGB | png::ColorType::RGBA => {}
        _ => return None,
    }

    let size = ScreenSize::new(info.width, info.height)?;

    let mut img_data = vec![0; info.buffer_size()];
    reader.next_frame(&mut img_data).ok()?;

    let data = match info.color_type {
        png::ColorType::RGB => ImageData::RGB(img_data),
        png::ColorType::RGBA => ImageData::RGBA(img_data),
        _ => return None,
    };

    Some(Image {
        data,
        size,
    })
}

fn read_jpeg<R: std::io::Read>(r: R) -> Option<Image> {
    let mut decoder = jpeg_decoder::Decoder::new(r);
    let img_data = decoder.decode().ok()?;
    let info = decoder.info()?;

    let size = ScreenSize::new(info.width as u32, info.height as u32)?;

    let data = match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => ImageData::RGB(img_data),
        _ => return None,
    };

    Some(Image {
        data,
        size,
    })
}

pub fn image_to_surface(
    image: &Image,
    surface: &mut [u8],
) {
    // Surface is always ARGB.
    const SURFACE_CHANNELS: usize = 4;

    debug_assert!(surface.len() % SURFACE_CHANNELS == 0);

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
            let mut j = 0;
            while j < data.len() {
                let r = data[j + 0] as u32;
                let g = data[j + 1] as u32;
                let b = data[j + 2] as u32;
                to_surface(r, g, b, 255);
                j += 3;
            }
        }
        ImageData::RGBA(data) => {
            let mut j = 0;
            while j < data.len() {
                let r = data[j + 0] as u32;
                let g = data[j + 1] as u32;
                let b = data[j + 2] as u32;
                let a = data[j + 3] as u32;
                to_surface(r, g, b, a);
                j += 4;
            }
        }
    }
}

pub fn load_sub_svg(
    data: &usvg::ImageData,
    opt: &Options,
) -> Option<(usvg::Tree, Options)> {
    let mut sub_opt = Options {
        usvg: usvg::Options {
            path: None,
            dpi: opt.usvg.dpi,
            font_family: opt.usvg.font_family.clone(),
            font_size: opt.usvg.font_size,
            languages: opt.usvg.languages.clone(),
            shape_rendering: opt.usvg.shape_rendering,
            text_rendering: opt.usvg.text_rendering,
            image_rendering: opt.usvg.image_rendering,
            keep_named_groups: false,
        },
        fit_to: FitTo::Original,
        background: None,
    };

    let tree = match data {
        usvg::ImageData::Path(ref path) => {
            let path = get_abs_path(path, opt);
            sub_opt.usvg.path = Some(path.clone());
            usvg::Tree::from_file(path, &sub_opt.usvg).ok()?
        }
        usvg::ImageData::Raw(ref data) => {
            usvg::Tree::from_data(data, &sub_opt.usvg).ok()?
        }
    };

    sanitize_sub_svg(&tree);

    Some((tree, sub_opt))
}

fn sanitize_sub_svg(
    tree: &usvg::Tree,
) {
    // Remove all Image nodes.
    //
    // The referenced SVG image cannot have any 'image' elements by itself.
    // Not only recursive. Any. Don't know why.

    // TODO: implement drain or something to the rctree.
    let mut changed = true;
    while changed {
        changed = false;

        for mut node in tree.root().descendants() {
            let mut rm = false;
            if let usvg::NodeKind::Image(_) = *node.borrow() {
                rm = true;
            };

            if rm {
                node.detach();
                changed = true;
                break;
            }
        }
    }
}

pub fn prepare_sub_svg_geom(
    view_box: usvg::ViewBox,
    img_size: ScreenSize,
) -> (usvg::Transform, Option<Rect>) {
    let r = view_box.rect;

    let new_size = utils::apply_view_box(&view_box, img_size);

    let (tx, ty, clip) = if view_box.aspect.slice {
        let (dx, dy) = utils::aligned_pos(
            view_box.aspect.align,
            0.0, 0.0, new_size.width() as f64 - r.width(), new_size.height() as f64 - r.height(),
        );

        (r.x() - dx, r.y() - dy, Some(r))
    } else {
        let (dx, dy) = utils::aligned_pos(
            view_box.aspect.align,
            r.x(), r.y(), r.width() - new_size.width() as f64, r.height() - new_size.height() as f64,
        );

        (dx, dy, None)
    };

    let sx = new_size.width() as f64 / img_size.width() as f64;
    let sy = new_size.height() as f64 / img_size.height() as f64;
    let ts = usvg::Transform::new(sx, 0.0, 0.0, sy, tx, ty);

    (ts, clip)
}

pub fn image_rect(
    view_box: &usvg::ViewBox,
    img_size: ScreenSize,
) -> Rect {
    let new_size = utils::apply_view_box(view_box, img_size);
    let (x, y) = utils::aligned_pos(
        view_box.aspect.align,
        view_box.rect.x(),
        view_box.rect.y(),
        view_box.rect.width() - new_size.width() as f64,
        view_box.rect.height() - new_size.height() as f64,
    );

    new_size.to_size().to_rect(x, y)
}

fn get_abs_path(
    rel_path: &path::Path,
    opt: &Options,
) -> path::PathBuf {
    match opt.usvg.path {
        Some(ref path) => path.parent().unwrap().join(rel_path),
        None => rel_path.into(),
    }
}
