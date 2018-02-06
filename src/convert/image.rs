// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::path;

// external
use base64;
use svgdom;

// self
use tree;
use short::{
    AId,
};
use traits::{
    GetValue,
};
use math::{
    Rect,
};
use {
    Options,
};


pub fn convert(
    node: &svgdom::Node,
    opt: &Options,
    depth: usize,
    rtree: &mut tree::RenderTree,
) {
    let attrs = node.attributes();

    let ts = attrs.get_transform(AId::Transform).unwrap_or_default();

    let x = attrs.get_number(AId::X).unwrap_or(0.0);
    let y = attrs.get_number(AId::Y).unwrap_or(0.0);

    macro_rules! get_attr {
        ($aid:expr) => (
            if let Some(v) = attrs.get_type($aid) {
                v
            } else {
                warn!("The 'image' element lacks '{}' attribute. Skipped.", $aid);
                return;
            }
        )
    }

    let w: f64 = *get_attr!(AId::Width);
    let h: f64 = *get_attr!(AId::Height);

    let href: &String = get_attr!(AId::XlinkHref);

    if let Some(data) = get_href_data(href, opt.path.as_ref()) {
        rtree.append_node(depth, tree::NodeKind::Image(tree::Image {
            id: node.id().clone(),
            transform: ts,
            rect: Rect::new(x, y, w, h),
            data: data,
        }));
    }
}

fn get_href_data(
    href: &str,
    path: Option<&path::PathBuf>,
) -> Option<tree::ImageData> {
    if href.starts_with("data:image") {
        if let Some(idx) = href.find(',') {
            let kind = if href[..idx].contains("image/jpg") {
                tree::ImageDataKind::JPEG
            } else if href[..idx].contains("image/png") {
                tree::ImageDataKind::PNG
            } else {
                return None;
            };

            let base_data = &href[(idx + 1)..];

            let conf = base64::Config::new(
                base64::CharacterSet::Standard,
                true,
                true,
                base64::LineWrap::NoWrap,
            );

            if let Ok(data) = base64::decode_config(base_data, conf) {
                return Some(tree::ImageData::Raw(data.to_owned(), kind));
            }
        }

        warn!("Invalid xlink:href content.");
    } else {
        let path = match path {
            Some(path) => path.parent().unwrap().join(href),
            None => path::PathBuf::from(href),
        };

        if path.exists() {
            if is_valid_image_format(&path) {
                return Some(tree::ImageData::Path(path.to_owned()));
            } else {
                warn!("'{}' is not a PNG or a JPEG image.", href);
            }
        } else {
            warn!("Linked file does not exist: '{}'.", href);
        }
    }

    None
}

/// Checks that file has a PNG or a JPEG magic bytes.
fn is_valid_image_format(path: &path::Path) -> bool {
    use std::fs;
    use std::io::Read;

    macro_rules! try_bool {
        ($e:expr) => {
            match $e {
                Ok(v) => v,
                Err(_) => return false,
            }
        };
    }

    let mut file = try_bool!(fs::File::open(path));

    let mut d = Vec::new();
    d.resize(8, 0);
    try_bool!(file.read_exact(&mut d));

    d.starts_with(b"\x89PNG\r\n\x1a\n") || d.starts_with(&[0xff, 0xd8, 0xff])
}
