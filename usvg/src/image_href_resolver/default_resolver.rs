//! Default resolver that `usvg` uses to handle `<image>` elements `xlink:href` strings.

use std::sync::Arc;

use crate::{
    image_href_resolver::{ImageHrefDataResolver, ImageHrefStringResolver},
    ImageKind, NodeKind, Options, OptionsRef, Tree,
};

#[derive(Clone, Copy, PartialEq, Debug)]
enum ImageFormat {
    PNG,
    JPEG,
    SVG,
}

/// Create DataUrl resolver function that handles standard mime types for JPEG, PNG and SVG.
pub fn create_default_data_resolver<'a>(
    options: Options<'a>,
) -> Box<dyn ImageHrefDataResolver + 'a> {
    Box::new(move |mime: &str, data: Arc<Vec<u8>>| {
        let options_ref = options.to_ref();

        match mime {
            "image/jpg" | "image/jpeg" => Some(ImageKind::JPEG(data)),
            "image/png" => Some(ImageKind::PNG(data)),
            "image/svg+xml" => load_sub_svg(&data, &options_ref),
            "text/plain" => match get_image_data_format(&data) {
                Some(ImageFormat::JPEG) => Some(ImageKind::JPEG(data)),
                Some(ImageFormat::PNG) => Some(ImageKind::PNG(data)),
                _ => load_sub_svg(&data, &options_ref),
            },
            _ => None,
        }
    })
}

/// Create resolver function that handles `href` string as path to local JPEG, PNG or SVG file.
pub fn create_default_string_resolver<'a>(
    options: Options<'a>,
) -> Box<dyn ImageHrefStringResolver + 'a> {
    Box::new(move |href: &str| {
        let options_ref = options.to_ref();

        let path = options_ref.get_abs_path(std::path::Path::new(href));

        if path.exists() {
            let data = match std::fs::read(&path) {
                Ok(data) => data,
                Err(_) => {
                    log::warn!("Failed to load '{}'. Skipped.", href);
                    return None;
                }
            };

            match get_image_file_format(&path, &data) {
                Some(ImageFormat::JPEG) => Some(ImageKind::JPEG(Arc::new(data))),
                Some(ImageFormat::PNG) => Some(ImageKind::PNG(Arc::new(data))),
                Some(ImageFormat::SVG) => load_sub_svg(&data, &options_ref),
                _ => {
                    log::warn!("'{}' is not a PNG, JPEG or SVG(Z) image.", href);
                    None
                }
            }
        } else {
            log::warn!("'{}' is not a path to an image.", href);
            None
        }
    })
}

/// Checks that file has a PNG or a JPEG magic bytes.
/// Or an SVG(Z) extension.
fn get_image_file_format(path: &std::path::Path, data: &[u8]) -> Option<ImageFormat> {
    let ext = crate::utils::file_extension(path)?.to_lowercase();
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
pub(crate) fn load_sub_svg(data: &[u8], opt: &OptionsRef) -> Option<ImageKind> {
    let mut sub_opt = opt.clone();
    sub_opt.resources_dir = None;
    sub_opt.keep_named_groups = false;

    let tree = match Tree::from_data(data, &sub_opt) {
        Ok(tree) => tree,
        Err(_) => {
            log::warn!("Failed to load subsvg image.");
            return None;
        }
    };

    sanitize_sub_svg(&tree);
    Some(ImageKind::SVG(tree))
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
            if let NodeKind::Image(_) = *node.borrow() {
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
