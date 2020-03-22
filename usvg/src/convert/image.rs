// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::path;

use super::prelude::*;
use crate::{svgtree, tree, tree::prelude::*, utils};

pub fn convert(node: svgtree::Node, state: &State, parent: &mut tree::Node) {
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

    let (data, format) = try_opt!(get_href_data(
        node.element_id(),
        href,
        state.opt.path.as_ref()
    ));
    parent.append_kind(tree::NodeKind::Image(tree::Image {
        id: node.element_id().to_string(),
        transform: Default::default(),
        visibility,
        view_box,
        rendering_mode,
        data,
        format,
    }));
}

pub fn get_href_data(
    element_id: &str,
    href: &str,
    path: Option<&path::PathBuf>,
) -> Option<(tree::ImageData, tree::ImageFormat)> {
    if href.starts_with("data:image/") {
        if let Ok(url) = data_url::DataUrl::process(href) {
            let format = match (
                url.mime_type().type_.as_str(),
                url.mime_type().subtype.as_str(),
            ) {
                ("image", "jpg") | ("image", "jpeg") => tree::ImageFormat::JPEG,
                ("image", "png") => tree::ImageFormat::PNG,
                ("image", "svg+xml") => tree::ImageFormat::SVG,
                _ => return None,
            };

            if let Ok((data, _)) = url.decode_to_vec() {
                return Some((tree::ImageData::Raw(data), format));
            }
        }

        warn!(
            "Image '{}' has an invalid 'xlink:href' content.",
            element_id
        );
    } else {
        let path = match path {
            Some(path) => path.parent().unwrap().join(href),
            None => path::PathBuf::from(href),
        };

        if path.exists() {
            if let Some(format) = get_image_format(&path) {
                return Some((tree::ImageData::Path(path::PathBuf::from(href)), format));
            } else {
                warn!("'{}' is not a PNG, JPEG or SVG(Z) image.", href);
            }
        } else {
            warn!("Linked file does not exist: '{}'.", href);
        }
    }

    None
}

/// Checks that file has a PNG or a JPEG magic bytes.
/// Or an SVG(Z) extension.
fn get_image_format(path: &path::Path) -> Option<tree::ImageFormat> {
    use std::io::Read;

    let ext = utils::file_extension(path)?.to_lowercase();
    if ext == "svg" || ext == "svgz" {
        return Some(tree::ImageFormat::SVG);
    }

    let mut file = std::fs::File::open(path).ok()?;

    let mut d = Vec::new();
    d.resize(8, 0);
    file.read_exact(&mut d).ok()?;

    if d.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(tree::ImageFormat::PNG)
    } else if d.starts_with(&[0xff, 0xd8, 0xff]) {
        Some(tree::ImageFormat::JPEG)
    } else {
        None
    }
}
