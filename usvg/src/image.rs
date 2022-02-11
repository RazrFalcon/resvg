// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::sync::Arc;
use svgtypes::Length;

use crate::{ImageRendering, Node, NodeExt, NodeKind, OptionLog, OptionsRef, Tree, Visibility, converter};
use crate::geom::{Rect, Transform, ViewBox};
use crate::svgtree::{self, AId};

#[derive(Clone, Copy, PartialEq, Debug)]
enum ImageFormat {
    PNG,
    JPEG,
    SVG,
}

/// An embedded image kind.
#[derive(Clone)]
pub enum ImageKind {
    /// A reference to raw JPEG data. Should be decoded by the caller.
    JPEG(Arc<Vec<u8>>),
    /// A reference to raw PNG data. Should be decoded by the caller.
    PNG(Arc<Vec<u8>>),
    /// A preprocessed SVG tree. Can be rendered as is.
    SVG(crate::Tree),
}

impl std::fmt::Debug for ImageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ImageKind::JPEG(_) => f.write_str("ImageKind::JPEG(..)"),
            ImageKind::PNG(_) => f.write_str("ImageKind::PNG(..)"),
            ImageKind::SVG(_) => f.write_str("ImageKind::SVG(..)"),
        }
    }
}

/// Functions that accept various representations of `xlink:href` value of the `<image>`
/// element and return ImageKind that holds reference to the image buffer determined by these functions.
#[allow(clippy::type_complexity)]
pub struct ImageHrefResolver {
    /// Resolver function that will be used if `xlink:href` is a DataUrl with encoded base64 string.
    pub resolve_data: Box<dyn Fn(&str, Arc<Vec<u8>>, &OptionsRef) -> Option<ImageKind>>,

    /// Resolver function that will be used to handle arbitrary string in `xlink:href`.
    pub resolve_string: Box<dyn Fn(&str, &OptionsRef) -> Option<ImageKind>>,
}

impl ImageHrefResolver {
    /// Create DataUrl resolver function that handles standard mime types for JPEG, PNG and SVG.
    #[allow(clippy::type_complexity)]
    pub fn default_data_resolver<'a>() -> Box<dyn Fn(&str, Arc<Vec<u8>>, &OptionsRef) -> Option<ImageKind> + 'a> {
        Box::new(
            move |mime: &str, data: Arc<Vec<u8>>, opts: &OptionsRef| match mime {
                "image/jpg" | "image/jpeg" => Some(ImageKind::JPEG(data)),
                "image/png" => Some(ImageKind::PNG(data)),
                "image/svg+xml" => load_sub_svg(&data, opts),
                "text/plain" => match get_image_data_format(&data) {
                    Some(ImageFormat::JPEG) => Some(ImageKind::JPEG(data)),
                    Some(ImageFormat::PNG) => Some(ImageKind::PNG(data)),
                    _ => load_sub_svg(&data, opts),
                },
                _ => None,
            },
        )
    }

    /// Create resolver function that handles `href` string as path to local JPEG, PNG or SVG file.
    pub fn default_string_resolver<'a>() -> Box<dyn Fn(&str, &OptionsRef) -> Option<ImageKind> + 'a> {
        Box::new(move |href: &str, opts: &OptionsRef| {
            let path = opts.get_abs_path(std::path::Path::new(href));

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
                    Some(ImageFormat::SVG) => load_sub_svg(&data, opts),
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
}

impl std::fmt::Debug for ImageHrefResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ImageHrefResolver { .. }")
    }
}

impl Default for ImageHrefResolver {
    fn default() -> Self {
        ImageHrefResolver {
            resolve_data: ImageHrefResolver::default_data_resolver(),
            resolve_string: ImageHrefResolver::default_string_resolver()
        }
    }
}

/// A raster image element.
///
/// `image` element in SVG.
#[derive(Clone, Debug)]
pub struct Image {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Isn't automatically generated.
    /// Can be empty.
    pub id: String,

    /// Element transform.
    pub transform: Transform,

    /// Element visibility.
    pub visibility: Visibility,

    /// An image rectangle in which it should be fit.
    ///
    /// Combination of the `x`, `y`, `width`, `height` and `preserveAspectRatio`
    /// attributes.
    pub view_box: ViewBox,

    /// Rendering mode.
    ///
    /// `image-rendering` in SVG.
    pub rendering_mode: ImageRendering,

    /// Image data.
    pub kind: ImageKind,
}

pub(crate) fn convert(
    node: svgtree::Node,
    state: &converter::State,
    parent: &mut Node,
) -> Option<()> {
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
    let rect = rect.log_none(|| log::warn!("Image has an invalid size. Skipped."))?;

    let view_box = ViewBox {
        rect,
        aspect: node.attribute(AId::PreserveAspectRatio).unwrap_or_default(),
    };

    let href = node.attribute(AId::Href)
        .log_none(|| log::warn!("Image lacks the 'xlink:href' attribute. Skipped."))?;

    let kind = get_href_data(href, state.opt)?;

    parent.append_kind(NodeKind::Image(Image {
        id: node.element_id().to_string(),
        transform: Default::default(),
        visibility,
        view_box,
        rendering_mode,
        kind,
    }));

    Some(())
}

pub(crate) fn get_href_data(href: &str, opt: &OptionsRef) -> Option<ImageKind> {
    if let Ok(url) = data_url::DataUrl::process(href) {
        let (data, _) = url.decode_to_vec().ok()?;

        let mime = format!(
            "{}/{}",
            url.mime_type().type_.as_str(),
            url.mime_type().subtype.as_str()
        );

        (opt.image_href_resolver.resolve_data)(&mime, Arc::new(data), opt)
    } else {
        (opt.image_href_resolver.resolve_string)(href, opt)
    }
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
