// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::path;

use crate::{svgtree, tree, tree::prelude::*, utils};
use super::prelude::*;


#[derive(Clone, Copy, PartialEq, Debug)]
enum ImageFormat {
    PNG,
    JPEG,
    SVG,
}


pub fn convert(
    node: svgtree::Node,
    state: &State,
    parent: &mut tree::Node,
) {
    let visibility = node.find_attribute(AId::Visibility).unwrap_or_default();
    let rendering_mode = node
        .find_attribute(AId::ImageRendering)
        .unwrap_or(state.opt.image_rendering);

    let rect = Rect::new(
        node.convert_user_length(AId::X, state, Length::zero()),
        node.convert_user_length(AId::Y, state, Length::zero()),
        node.convert_user_length(AId::Width, state, Length::zero()),
        node.convert_user_length(AId::Height, state, Length::zero()),
    );
    let rect = try_opt_warn!(rect, "Image has an invalid size. Skipped.");

    let view_box = tree::ViewBox {
        rect,
        aspect: node.attribute(AId::PreserveAspectRatio).unwrap_or_default(),
    };

    let href = try_opt_warn!(
        node.attribute(AId::Href),
        "The 'image' element lacks the 'xlink:href' attribute. Skipped."
    );

    let kind = try_opt!(get_href_data(node.element_id(), href, state.opt));
    parent.append_kind(tree::NodeKind::Image(tree::Image {
        id: node.element_id().to_string(),
        transform: Default::default(),
        visibility,
        view_box,
        rendering_mode,
        kind,
    }));
}

pub fn get_href_data(
    element_id: &str,
    href: &str,
    opt: &Options,
) -> Option<tree::ImageKind> {
    if let Ok(url) = data_url::DataUrl::process(href) {
        let (data, _) = url.decode_to_vec().ok()?;
        match (url.mime_type().type_.as_str(), url.mime_type().subtype.as_str()) {
            ("image", "jpg") | ("image", "jpeg") => Some(tree::ImageKind::JPEG(data)),
            ("image", "png") => Some(tree::ImageKind::PNG(data)),
            ("image", "svg+xml") => load_sub_svg(&data, opt),
            ("text", "plain") => {
                match get_image_data_format(&data) {
                    Some(ImageFormat::JPEG) => {
                        Some(tree::ImageKind::JPEG(data))
                    }
                    Some(ImageFormat::PNG) => {
                        Some(tree::ImageKind::PNG(data))
                    }
                    _ => {
                        load_sub_svg(&data, opt)
                    }
                }
            }
            _ => None,
        }
    } else {
        let path = opt.get_abs_path(&path::Path::new(href));
        if path.exists() {
            let data = match std::fs::read(&path) {
                Ok(data) => data,
                Err(_) => {
                    warn!("Failed to load '{}'. Skipped.", href);
                    return None;
                }
            };

            match get_image_file_format(&path, &data) {
                Some(ImageFormat::JPEG) => {
                    Some(tree::ImageKind::JPEG(data))
                }
                Some(ImageFormat::PNG) => {
                    Some(tree::ImageKind::PNG(data))
                }
                Some(ImageFormat::SVG) => {
                    load_sub_svg(&data, opt)
                }
                _ => {
                    warn!("'{}' is not a PNG, JPEG or SVG(Z) image.", href);
                    None
                }
            }
        } else {
            warn!("Image '{}' has an invalid 'xlink:href' content.", element_id);
            None
        }
    }
}

/// Checks that file has a PNG or a JPEG magic bytes.
/// Or an SVG(Z) extension.
fn get_image_file_format(path: &path::Path, data: &[u8]) -> Option<ImageFormat> {
    let ext = utils::file_extension(path)?.to_lowercase();
    if ext == "svg" || ext == "svgz" {
        return Some(ImageFormat::SVG);
    }

    get_image_data_format(data.get(0..8)?)
}

/// Checks that file has a PNG or a JPEG magic bytes.
fn get_image_data_format(data: &[u8]) -> Option<ImageFormat> {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(ImageFormat::PNG)
    } else if data.starts_with(&[0xff, 0xd8, 0xff]) {
        Some(ImageFormat::JPEG)
    } else {
        None
    }
}


/// Tries to load the `ImageData` content as an SVG image.
///
/// Unlike `Tree::from_*` methods, this one will also remove all `image` elements
/// from the loaded SVG, as required by the spec.
pub fn load_sub_svg(data: &[u8], opt: &Options) -> Option<tree::ImageKind> {
    let sub_opt = Options {
        resources_dir: None,
        dpi: opt.dpi,
        font_family: opt.font_family.clone(),
        font_size: opt.font_size,
        languages: opt.languages.clone(),
        shape_rendering: opt.shape_rendering,
        text_rendering: opt.text_rendering,
        image_rendering: opt.image_rendering,
        keep_named_groups: false,
        #[cfg(feature = "text")]
        fontdb: opt.fontdb.clone(),
    };

    let tree = match tree::Tree::from_data(data, &sub_opt) {
        Ok(tree) => tree,
        Err(_) => {
            warn!("Failed to load subsvg image.");
            return None;
        }
    };

    sanitize_sub_svg(&tree);
    Some(tree::ImageKind::SVG(tree))
}

fn sanitize_sub_svg(tree: &crate::Tree) {
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
            // TODO: feImage?
            if let tree::NodeKind::Image(_) = *node.borrow() {
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
