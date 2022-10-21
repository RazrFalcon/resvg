// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::svgtree::{self, AId};
use crate::{AspectRatio, ImageRendering, converter, Node, NodeKind, Group};
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

    /// An SVG node.
    ///
    /// Isn't inside a dummy group like clip, mask and pattern because
    /// `feImage` can reference only a single element.
    Use(Node),
}

pub(crate) fn convert(
    fe: svgtree::Node,
    state: &converter::State,
    cache: &mut converter::Cache,
) -> Kind {
    let aspect = fe.attribute(AId::PreserveAspectRatio).unwrap_or_default();
    let rendering_mode = fe
        .find_attribute(AId::ImageRendering)
        .unwrap_or(state.opt.image_rendering);

    if let Some(node) = fe.attribute::<svgtree::Node>(AId::Href) {
        let mut state = state.clone();
        state.fe_image_link = true;
        let mut root = Node::new(NodeKind::Group(Group::default()));
        crate::converter::convert_element(node, &state, cache, &mut root);
        return if let Some(mut node) = root.first_child() {
            node.detach(); // drops `root` node
            Kind::Image(Image {
                aspect,
                rendering_mode,
                data: ImageKind::Use(node),
            })
        } else {
            super::create_dummy_primitive()
        };
    }

    let href = match fe.attribute(AId::Href) {
        Some(s) => s,
        _ => {
            log::warn!("The 'feImage' element lacks the 'xlink:href' attribute. Skipped.");
            return super::create_dummy_primitive();
        }
    };

    let href = crate::image::get_href_data(href, state.opt);
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
