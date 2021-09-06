// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::svgtree::{self, AId};
use crate::{AspectRatio, ImageRendering, converter};
use super::Kind;

/// An image filter primitive.
///
/// `feImage` element in the SVG.
#[derive(Clone, Debug)]
pub struct Image {
    /// Value of the `preserveAspectRatio` attribute.
    pub aspect: AspectRatio,

    /// Rendering method.
    ///
    /// `image-rendering` in SVG.
    pub rendering_mode: ImageRendering,

    /// Image data.
    pub data: ImageKind,
}

/// Kind of the `feImage` data.
#[derive(Clone, Debug)]
pub enum ImageKind {
    /// An image data.
    Image(crate::ImageKind),

    /// A reference to an SVG object.
    ///
    /// `feImage` can reference any SVG object, just like `use` element.
    Use(String),
}

pub(crate) fn convert(fe: svgtree::Node, state: &converter::State) -> Kind {
    let aspect = fe.attribute(AId::PreserveAspectRatio).unwrap_or_default();
    let rendering_mode = fe
        .find_attribute(AId::ImageRendering)
        .unwrap_or(state.opt.image_rendering);

    if let Some(node) = fe.attribute::<svgtree::Node>(AId::Href) {
        // If `feImage` references an existing SVG element,
        // simply store its ID and do not attempt to convert the element itself.
        // The problem is that `feImage` can reference an element outside `defs`,
        // and we should not create it manually.
        // Instead, after document conversion is finished, we should search for this ID
        // and if it does not exist - create it inside `defs`.
        return Kind::Image(Image {
            aspect,
            rendering_mode,
            data: ImageKind::Use(node.element_id().to_string()),
        });
    }

    let href = match fe.attribute(AId::Href) {
        Some(s) => s,
        _ => {
            log::warn!("The 'feImage' element lacks the 'xlink:href' attribute. Skipped.");
            return super::create_dummy_primitive();
        }
    };

    let href = crate::image::get_href_data(fe.element_id(), href, state.opt);
    let img_data = match href {
        Some(data) => data,
        None => return super::create_dummy_primitive(),
    };

    Kind::Image(Image {
        aspect,
        rendering_mode,
        data: ImageKind::Image(img_data),
    })
}
