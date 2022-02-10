// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::sync::Arc;
use svgtypes::Length;

use crate::geom::{Rect, Transform, ViewBox};
use crate::svgtree::{self, AId};
use crate::{
    converter, ImageRendering, Node, NodeExt, NodeKind, OptionLog, OptionsRef, Visibility,
};

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

    let href = node
        .attribute(AId::Href)
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
    if let Some(href_resolver) = opt.image_href_resolver {
        if let Ok(url) = data_url::DataUrl::process(href) {
            let (data, _) = url.decode_to_vec().ok()?;

            let mime = format!(
                "{}/{}",
                url.mime_type().type_.as_str(),
                url.mime_type().subtype.as_str()
            );

            (href_resolver.resolve_data)(&mime, Arc::new(data))
        } else {
            (href_resolver.resolve_string)(href)
        }
    } else {
        log::warn!("ImageHrefResolver is not set in the Options!");
        None
    }
}
