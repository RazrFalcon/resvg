// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::os::raw::c_char;

use log::warn;
use usvg::NodeExt;

mod usvg_capi;
use usvg_capi::*;

#[no_mangle]
pub extern "C" fn resvg_cairo_render_to_canvas(
    tree: *const resvg_render_tree,
    size: resvg_size,
    cr: *mut cairo_sys::cairo_t,
) {
    let tree = unsafe {
        assert!(!tree.is_null());
        &*tree
    };

    let cr = unsafe { cairo::Context::from_raw_none(cr) };
    let size = usvg::ScreenSize::new(size.width, size.height).unwrap();

    resvg_cairo::render_to_canvas(&tree.0, size, &cr);
}

#[no_mangle]
pub extern "C" fn resvg_cairo_render_to_canvas_by_id(
    tree: *const resvg_render_tree,
    size: resvg_size,
    id: *const c_char,
    cr: *mut cairo_sys::cairo_t,
) {
    let tree = unsafe {
        assert!(!tree.is_null());
        &*tree
    };

    let cr = unsafe { cairo::Context::from_raw_none(cr) };
    let size = usvg::ScreenSize::new(size.width, size.height).unwrap();

    let id = match cstr_to_str(id) {
        Some(v) => v,
        None => return,
    };

    if id.is_empty() {
        warn!("Node with an empty ID cannot be painted.");
        return;
    }

    if let Some(node) = tree.0.node_by_id(id) {
        if let Some(bbox) = node.calculate_bbox() {
            let vbox = usvg::ViewBox {
                rect: bbox,
                aspect: usvg::AspectRatio::default(),
            };

            resvg_cairo::render_node_to_canvas(&node, vbox, size, &cr);
        } else {
            warn!("A node with '{}' ID doesn't have a valid bounding box.", id);
        }
    } else {
        warn!("A node with '{}' ID wasn't found.", id);
    }
}
