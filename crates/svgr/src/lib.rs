// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/*!
[svgr](https://github.com/RazrFalcon/svgr) is an SVG rendering library.
*/

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::identity_op)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::wrong_self_convention)]

pub use tiny_skia;
pub use usvgr;

mod cache;
mod clip;
mod filter;
mod geom;
mod image;
mod mask;
mod path;
mod render;

pub use cache::*;
pub use render::Context;

/// Renders a tree onto the pixmap.
///
/// `transform` will be used as a root transform.
/// Can be used to position SVG inside the `pixmap`.
///
/// The produced content is in the sRGB color space.
pub fn render(
    tree: &usvgr::Tree,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
    cache: &mut cache::SvgrCache,
    ctx: &render::Context,
) {
    let ts = tree.view_box().to_transform(tree.size());
    let root_transform = transform.pre_concat(ts);

    render::render_nodes(tree.root(), &ctx, root_transform, pixmap, cache);
}

/// Renders a node onto the pixmap.
///
/// `transform` will be used as a root transform.
/// Can be used to position SVG inside the `pixmap`.
///
/// The expected pixmap size can be retrieved from `usvgr::Node::abs_layer_bounding_box()`.
///
/// Returns `None` when `node` has a zero size.
///
/// The produced content is in the sRGB color space.
pub fn render_node(
    node: &usvgr::Node,
    mut transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
    cache: &mut cache::SvgrCache,
    ctx: &render::Context,
) -> Option<()> {
    let bbox = node.abs_layer_bounding_box()?;
    transform = transform.pre_translate(-bbox.x(), -bbox.y());

    render::render_node(node, &ctx, transform, pixmap, cache);
    Some(())
}

pub(crate) trait OptionLog {
    fn log_none<F: FnOnce()>(self, f: F) -> Self;
}

impl<T> OptionLog for Option<T> {
    #[inline]
    fn log_none<F: FnOnce()>(self, f: F) -> Self {
        self.or_else(|| {
            f();
            None
        })
    }
}
