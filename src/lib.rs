// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/*!
[resvg](https://github.com/RazrFalcon/resvg) is an SVG rendering library.
*/

#![warn(missing_docs)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::identity_op)]
#![allow(clippy::upper_case_acronyms)]

pub use usvg::ScreenSize;

use usvg::NodeExt;
use log::warn;

#[macro_use] mod macros;
mod clip;
#[cfg(feature = "filter")] mod filter;
mod image;
mod mask;
mod paint_server;
mod path;
mod render;

pub use crate::render::trim_transparency;


/// Renders an SVG to pixmap.
///
/// If `fit_to` size differs from `tree.svg_node().size`,
/// SVG would be scaled accordingly.
pub fn render(
    tree: &usvg::Tree,
    fit_to: usvg::FitTo,
    pixmap: tiny_skia::PixmapMut,
) -> Option<()> {
    let size = fit_to.fit_to(tree.svg_node().size.to_screen_size())?;
    let mut canvas = render::Canvas::from(pixmap);
    render::render_to_canvas(tree, size, &mut canvas);
    Some(())
}

/// Renders an SVG node to pixmap.
///
/// If `fit_to` differs from `node.calculate_bbox()`,
/// SVG would be scaled accordingly.
pub fn render_node(
    tree: &usvg::Tree,
    node: &usvg::Node,
    fit_to: usvg::FitTo,
    pixmap: tiny_skia::PixmapMut,
) -> Option<()> {
    let node_bbox = if let Some(bbox) = node.calculate_bbox() {
        bbox
    } else {
        warn!("Node '{}' has zero size.", node.id());
        return None;
    };

    let vbox = usvg::ViewBox {
        rect: node_bbox,
        aspect: usvg::AspectRatio::default(),
    };

    let size = fit_to.fit_to(node_bbox.size().to_screen_size())?;
    let mut canvas = render::Canvas::from(pixmap);
    render::render_node_to_canvas(tree, node, vbox, size, &mut render::RenderState::Ok, &mut canvas);
    Some(())
}
