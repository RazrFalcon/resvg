// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/*!
[resvg](https://github.com/RazrFalcon/resvg) backend implementation
using the [raqote](https://github.com/jrmuizel/raqote) library.
*/

#![doc(html_root_url = "https://docs.rs/resvg-raqote/0.10.0")]

#![warn(missing_docs)]

/// Unwraps `Option` and invokes `return` on `None`.
macro_rules! try_opt {
    ($task:expr) => {
        match $task {
            Some(v) => v,
            None => return,
        }
    };
}

/// Unwraps `Option` and invokes `return $ret` on `None`.
macro_rules! try_opt_or {
    ($task:expr, $ret:expr) => {
        match $task {
            Some(v) => v,
            None => return $ret,
        }
    };
}


use usvg::{NodeExt, ScreenSize};
use log::warn;

mod clip;
mod filter;
mod image;
mod layers;
mod mask;
mod paint_server;
mod path;
mod render;


/// Renders SVG to image.
pub fn render_to_image(
    tree: &usvg::Tree,
    fit_to: usvg::FitTo,
    background: Option<usvg::Color>,
) -> Option<raqote::DrawTarget> {
    let (mut dt, img_view)
        = render::create_root_target(tree.svg_node().size.to_screen_size(), fit_to, background)?;

    render_to_canvas(tree, img_view, &mut dt);

    Some(dt)
}

/// Renders SVG node to image.
pub fn render_node_to_image(
    node: &usvg::Node,
    fit_to: usvg::FitTo,
    background: Option<usvg::Color>,
) -> Option<raqote::DrawTarget> {
    let node_bbox = if let Some(bbox) = node.calculate_bbox() {
        bbox
    } else {
        warn!("Node '{}' has a zero size.", node.id());
        return None;
    };

    let (mut dt, img_size)
        = render::create_root_target(node_bbox.to_screen_size(), fit_to, background)?;

    let vbox = usvg::ViewBox {
        rect: node_bbox,
        aspect: usvg::AspectRatio::default(),
    };

    render_node_to_canvas(node, vbox, img_size, &mut dt);

    Some(dt)
}

/// Renders `tree` onto the canvas.
///
/// The caller must guarantee that `img_size` is large enough.
///
/// Canvas must not have a transform.
pub fn render_to_canvas(
    tree: &usvg::Tree,
    img_size: ScreenSize,
    dt: &mut raqote::DrawTarget,
) {
    render_node_to_canvas(&tree.root(), tree.svg_node().view_box, img_size, dt);
}

/// Renders `node` onto the canvas.
///
/// The caller must guarantee that `img_size` is large enough.
///
/// Canvas must not have a transform.
pub fn render_node_to_canvas(
    node: &usvg::Node,
    view_box: usvg::ViewBox,
    img_size: ScreenSize,
    dt: &mut raqote::DrawTarget,
) {
    render::render_node_to_canvas(node, view_box, img_size, &mut render::RenderState::Ok, dt)
}
