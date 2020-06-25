// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/*!
[resvg](https://github.com/RazrFalcon/resvg) backend implementation
using the [Skia](https://skia.org/) library.
*/

#![doc(html_root_url = "https://docs.rs/resvg-skia/0.10.0")]

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

/// Unwraps `Option` and invokes `return $ret` on `None` with a warning.
macro_rules! try_opt_warn_or {
    ($task:expr, $ret:expr, $msg:expr) => {
        match $task {
            Some(v) => v,
            None => {
                log::warn!($msg);
                return $ret;
            }
        }
    };
    ($task:expr, $ret:expr, $fmt:expr, $($arg:tt)*) => {
        match $task {
            Some(v) => v,
            None => {
                log::warn!($fmt, $($arg)*);
                return $ret;
            }
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
mod skia;


/// Renders SVG to image.
pub fn render_to_image(
    tree: &usvg::Tree,
    opt: &usvg::Options,
    fit_to: usvg::FitTo,
    background: Option<usvg::Color>,
) -> Option<skia::Surface> {
    let (mut img, img_size)
        = render::create_root_image(tree.svg_node().size.to_screen_size(), fit_to, background)?;
    render_to_canvas(tree, opt, img_size, &mut img);
    Some(img)
}

/// Renders SVG node to image.
pub fn render_node_to_image(
    node: &usvg::Node,
    opt: &usvg::Options,
    fit_to: usvg::FitTo,
    background: Option<usvg::Color>,
) -> Option<skia::Surface> {
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

    let (mut img, img_size)
        = render::create_root_image(node_bbox.size().to_screen_size(), fit_to, background)?;

    render_node_to_canvas(node, opt, vbox, img_size, &mut img);
    Some(img)
}

/// Renders `tree` onto the canvas.
///
/// The caller must guarantee that `img_size` is large enough.
///
/// Canvas must not have a transform.
pub fn render_to_canvas(
    tree: &usvg::Tree,
    opt: &usvg::Options,
    img_size: ScreenSize,
    canvas: &mut skia::Canvas,
) {
    render_node_to_canvas(&tree.root(), opt, tree.svg_node().view_box, img_size, canvas);
}

/// Renders `node` onto the canvas.
///
/// The caller must guarantee that `img_size` is large enough.
///
/// Canvas must not have a transform.
pub fn render_node_to_canvas(
    node: &usvg::Node,
    opt: &usvg::Options,
    view_box: usvg::ViewBox,
    img_size: ScreenSize,
    canvas: &mut skia::Canvas,
) {
    render::render_node_to_canvas(node, opt, view_box, img_size, &mut render::RenderState::Ok, canvas)
}

/// Converts a raw pointer into a Skia Canvas object.
///
/// Used only by C-API.
pub unsafe fn canvas_from_ptr(painter: *mut std::ffi::c_void) -> skia::Canvas {
    skia::Canvas::from_ptr(painter as _).unwrap()
}
