// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::sync::Arc;

use svgtypes::Length;
use usvg_tree::{Group, Image, ImageKind, Node, NonZeroRect, Size, Transform, Tree, ViewBox};

use crate::svgtree::{AId, SvgNode};
use crate::{converter, OptionLog, Options, TreeParsing};

/// A shorthand for [ImageHrefResolver]'s data function.
pub type ImageHrefDataResolverFn =
    Box<dyn Fn(&str, Arc<Vec<u8>>, &Options) -> Option<ImageKind> + Send + Sync>;
/// A shorthand for [ImageHrefResolver]'s string function.
pub type ImageHrefStringResolverFn = Box<dyn Fn(&str, &Options) -> Option<ImageKind> + Send + Sync>;

/// An `xlink:href` resolver for `<image>` elements.
///
/// This type can be useful if you want to have an alternative `xlink:href` handling
/// to the default one. For example, you can forbid access to local files (which is allowed by default)
/// or add support for resolving actual URLs (usvg doesn't do any network requests).
pub struct ImageHrefResolver {
    /// Resolver function that will be used when `xlink:href` contains a
    /// [Data URL](https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/Data_URIs).
    ///
    /// A function would be called with mime, decoded base64 data and parsing options.
    pub resolve_data: ImageHrefDataResolverFn,

    /// Resolver function that will be used to handle an arbitrary string in `xlink:href`.
    pub resolve_string: ImageHrefStringResolverFn,
}

impl Default for ImageHrefResolver {
    fn default() -> Self {
        ImageHrefResolver {
            resolve_data: ImageHrefResolver::default_data_resolver(),
            resolve_string: ImageHrefResolver::default_string_resolver(),
        }
    }
}

impl ImageHrefResolver {
    /// Creates a default
    /// [Data URL](https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/Data_URIs)
    /// resolver closure.
    ///
    /// base64 encoded data is already decoded.
    ///
    /// The default implementation would try to load JPEG, PNG, GIF, SVG and SVGZ types.
    /// Note that it will simply match the `mime` or data's magic.
    /// The actual images would not be decoded. It's up to the renderer.
    pub fn default_data_resolver() -> ImageHrefDataResolverFn {
        Box::new(
            move |mime: &str, data: Arc<Vec<u8>>, opts: &Options| match mime {
                "image/jpg" | "image/jpeg" => Some(ImageKind::JPEG(data)),
                "image/png" => Some(ImageKind::PNG(data)),
                "image/gif" => Some(ImageKind::GIF(data)),
                "image/svg+xml" => load_sub_svg(&data, opts),
                "text/plain" => match get_image_data_format(&data) {
                    Some(ImageFormat::JPEG) => Some(ImageKind::JPEG(data)),
                    Some(ImageFormat::PNG) => Some(ImageKind::PNG(data)),
                    Some(ImageFormat::GIF) => Some(ImageKind::GIF(data)),
                    _ => load_sub_svg(&data, opts),
                },
                _ => None,
            },
        )
    }

    /// Creates a default string resolver.
    ///
    /// The default implementation treats an input string as a file path and tries to open.
    /// If a string is an URL or something else it would be ignored.
    ///
    /// Paths have to be absolute or relative to the input SVG file or relative to
    /// [Options::resources_dir](crate::Options::resources_dir).
    pub fn default_string_resolver() -> ImageHrefStringResolverFn {
        Box::new(move |href: &str, opts: &Options| {
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
                    Some(ImageFormat::GIF) => Some(ImageKind::GIF(Arc::new(data))),
                    Some(ImageFormat::SVG) => load_sub_svg(&data, opts),
                    _ => {
                        log::warn!("'{}' is not a PNG, JPEG, GIF or SVG(Z) image.", href);
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

#[derive(Clone, Copy, PartialEq, Debug)]
enum ImageFormat {
    PNG,
    JPEG,
    GIF,
    SVG,
}

pub(crate) fn convert(node: SvgNode, state: &converter::State, parent: &mut Group) -> Option<()> {
    let href = node
        .try_attribute(AId::Href)
        .log_none(|| log::warn!("Image lacks the 'xlink:href' attribute. Skipped."))?;

    let kind = get_href_data(href, state.opt)?;

    let visibility = node.find_attribute(AId::Visibility).unwrap_or_default();
    let rendering_mode = node
        .find_attribute(AId::ImageRendering)
        .unwrap_or(state.opt.image_rendering);

    let actual_size = match kind {
        ImageKind::JPEG(ref data) | ImageKind::PNG(ref data) | ImageKind::GIF(ref data) => {
            imagesize::blob_size(data)
                .ok()
                .and_then(|size| Size::from_wh(size.width as f32, size.height as f32))
                .log_none(|| log::warn!("Image has an invalid size. Skipped."))?
        }
        ImageKind::SVG(ref svg) => svg.size,
    };

    let rect = NonZeroRect::from_xywh(
        node.convert_user_length(AId::X, state, Length::zero()),
        node.convert_user_length(AId::Y, state, Length::zero()),
        node.convert_user_length(
            AId::Width,
            state,
            Length::new_number(actual_size.width() as f64),
        ),
        node.convert_user_length(
            AId::Height,
            state,
            Length::new_number(actual_size.height() as f64),
        ),
    );
    let rect = rect.log_none(|| log::warn!("Image has an invalid size. Skipped."))?;

    let view_box = ViewBox {
        rect,
        aspect: node.attribute(AId::PreserveAspectRatio).unwrap_or_default(),
    };

    // Nodes generated by markers must not have an ID. Otherwise we would have duplicates.
    let id = if state.parent_markers.is_empty() {
        node.element_id().to_string()
    } else {
        String::new()
    };

    parent.children.push(Node::Image(Box::new(Image {
        id,
        visibility,
        view_box,
        rendering_mode,
        kind,
        abs_transform: Transform::default(),
        bounding_box: None,
    })));

    Some(())
}

pub(crate) fn get_href_data(href: &str, opt: &Options) -> Option<ImageKind> {
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

/// Checks that file has a PNG, a GIF or a JPEG magic bytes.
/// Or an SVG(Z) extension.
fn get_image_file_format(path: &std::path::Path, data: &[u8]) -> Option<ImageFormat> {
    let ext = path.extension().and_then(|e| e.to_str())?.to_lowercase();
    if ext == "svg" || ext == "svgz" {
        return Some(ImageFormat::SVG);
    }

    get_image_data_format(data)
}

/// Checks that file has a PNG, a GIF or a JPEG magic bytes.
fn get_image_data_format(data: &[u8]) -> Option<ImageFormat> {
    match imagesize::image_type(data).ok()? {
        imagesize::ImageType::Gif => Some(ImageFormat::GIF),
        imagesize::ImageType::Jpeg => Some(ImageFormat::JPEG),
        imagesize::ImageType::Png => Some(ImageFormat::PNG),
        _ => None,
    }
}

/// Tries to load the `ImageData` content as an SVG image.
///
/// Unlike `Tree::from_*` methods, this one will also remove all `image` elements
/// from the loaded SVG, as required by the spec.
pub(crate) fn load_sub_svg(data: &[u8], opt: &Options) -> Option<ImageKind> {
    let mut sub_opt = Options::default();
    sub_opt.resources_dir = None;
    sub_opt.dpi = opt.dpi;
    sub_opt.font_size = opt.font_size;
    sub_opt.languages = opt.languages.clone();
    sub_opt.shape_rendering = opt.shape_rendering;
    sub_opt.text_rendering = opt.text_rendering;
    sub_opt.image_rendering = opt.image_rendering;
    sub_opt.default_size = opt.default_size;

    // The referenced SVG image cannot have any 'image' elements by itself.
    // Not only recursive. Any. Don't know why.
    sub_opt.image_href_resolver = ImageHrefResolver {
        resolve_data: Box::new(|_, _, _| None),
        resolve_string: Box::new(|_, _| None),
    };

    let mut tree = match Tree::from_data(data, &sub_opt) {
        Ok(tree) => tree,
        Err(_) => {
            log::warn!("Failed to load subsvg image.");
            return None;
        }
    };
    tree.calculate_abs_transforms();
    tree.calculate_bounding_boxes();

    Some(ImageKind::SVG(tree))
}
