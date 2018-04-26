// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![allow(non_camel_case_types)]

extern crate resvg;
#[macro_use] extern crate log;
extern crate fern;

#[cfg(feature = "cairo-backend")]
extern crate glib;
#[cfg(feature = "cairo-backend")]
extern crate cairo_sys;

use std::fmt;
use std::path;
use std::ptr;
use std::ffi::{ CStr, CString };
use std::os::raw::c_char;

#[cfg(feature = "qt-backend")]
use resvg::qt;

#[cfg(feature = "cairo-backend")]
use resvg::cairo;

use resvg::usvg;
use resvg::tree;
use resvg::geom::*;


#[repr(C)]
pub struct resvg_options {
    pub path: *const c_char,
    pub dpi: f64,
    pub fit_to: resvg_fit_to,
    pub draw_background: bool,
    pub background: resvg_color,
    pub keep_named_groups: bool,
}

#[repr(C)]
pub struct resvg_color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[repr(C)]
pub enum resvg_fit_to_type {
    RESVG_FIT_TO_ORIGINAL,
    RESVG_FIT_TO_WIDTH,
    RESVG_FIT_TO_HEIGHT,
    RESVG_FIT_TO_ZOOM,
}

#[repr(C)]
pub struct resvg_fit_to {
    kind: resvg_fit_to_type,
    value: f32,
}

#[repr(C)]
pub struct resvg_rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[repr(C)]
pub struct resvg_size {
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
pub struct resvg_transform {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

#[repr(C)]
pub struct resvg_render_tree(resvg::tree::Tree);

#[repr(C)]
pub struct resvg_handle(resvg::InitObject);

macro_rules! on_err {
    ($err:expr, $msg:expr) => ({
        let c_str = CString::new($msg).unwrap();
        unsafe { *$err = c_str.into_raw(); }
        return ptr::null_mut();
    })
}

#[no_mangle]
pub extern fn resvg_init() -> *mut resvg_handle {
    let handle = Box::new(resvg_handle(resvg::init()));
    Box::into_raw(handle)
}

#[no_mangle]
pub extern fn resvg_destroy(handle: *mut resvg_handle) {
    unsafe {
        assert!(!handle.is_null());
        Box::from_raw(handle)
    };
}

#[no_mangle]
pub extern fn resvg_init_log() {
    fern::Dispatch::new()
        .format(log_format)
        .level(log::LevelFilter::Warn)
        .chain(std::io::stderr())
        .apply().unwrap();
}

fn log_format(out: fern::FormatCallback, message: &fmt::Arguments, record: &log::Record) {
    let lvl = match record.level() {
        log::Level::Error => "Error",
        log::Level::Warn => "Warning",
        log::Level::Info => "Info",
        log::Level::Debug => "Debug",
        log::Level::Trace => "Trace",
    };

    out.finish(format_args!(
        "{} (in {}:{}): {}",
        lvl,
        record.target(),
        record.line().unwrap_or(0),
        message
    ))
}

#[no_mangle]
pub extern fn resvg_parse_rtree_from_file(
    file_path: *const c_char,
    opt: *const resvg_options,
    error: *mut *mut c_char,
) -> *mut resvg_render_tree {
    let file_path = match cstr_to_str(file_path) {
        Some(v) => v,
        None => on_err!(error, "Error: file path is not an UTF-8 string."),
    };

    let opt = to_native_opt(unsafe {
        assert!(!opt.is_null());
        &*opt
    });

    let rtree = match resvg::parse_rtree_from_file(file_path, &opt) {
        Ok(rtree) => rtree,
        Err(e) => on_err!(error, e.to_string()),
    };

    let rtree_box = Box::new(resvg_render_tree(rtree));
    Box::into_raw(rtree_box)
}

#[no_mangle]
pub extern fn resvg_parse_rtree_from_data(
    text: *const c_char,
    opt: *const resvg_options,
    error: *mut *mut c_char,
) -> *mut resvg_render_tree {
    let text = match cstr_to_str(text) {
        Some(v) => v,
        None => on_err!(error, "Error: SVG data is not an UTF-8 string."),
    };

    let opt = to_native_opt(unsafe {
        assert!(!opt.is_null());
        &*opt
    });

    let rtree = resvg::parse_rtree_from_data(text, &opt);

    let rtree_box = Box::new(resvg_render_tree(rtree));
    Box::into_raw(rtree_box)
}

#[no_mangle]
pub extern fn resvg_error_msg_destroy(msg: *mut c_char) {
    unsafe {
        assert!(!msg.is_null());
        CString::from_raw(msg)
    };
}

#[no_mangle]
pub extern fn resvg_rtree_destroy(rtree: *mut resvg_render_tree) {
    unsafe {
        assert!(!rtree.is_null());
        Box::from_raw(rtree)
    };
}

#[cfg(feature = "qt-backend")]
#[no_mangle]
pub extern fn resvg_qt_render_to_image(
    rtree: *const resvg_render_tree,
    opt: *const resvg_options,
    file_path: *const c_char,
) -> bool {
    let backend = Box::new(resvg::render_qt::Backend);
    render_to_image(rtree, opt, file_path, backend)
}

#[cfg(feature = "cairo-backend")]
#[no_mangle]
pub extern fn resvg_cairo_render_to_image(
    rtree: *const resvg_render_tree,
    opt: *const resvg_options,
    file_path: *const c_char,
) -> bool {
    let backend = Box::new(resvg::render_cairo::Backend);
    render_to_image(rtree, opt, file_path, backend)
}

fn render_to_image(
    rtree: *const resvg_render_tree,
    opt: *const resvg_options,
    file_path: *const c_char,
    backend: Box<resvg::Render>,
) -> bool {
    let rtree = unsafe {
        assert!(!rtree.is_null());
        &*rtree
    };

    let file_path = match cstr_to_str(file_path) {
        Some(v) => v,
        None => return false,
    };

    let opt = to_native_opt(unsafe {
        assert!(!opt.is_null());
        &*opt
    });

    let img = backend.render_to_image(&rtree.0, &opt);
    let img = match img {
        Ok(img) => img,
        Err(e) => {
            warn!("{}", e);
            return false;
        }
    };

    img.save(path::Path::new(file_path))
}

#[cfg(feature = "qt-backend")]
#[no_mangle]
pub extern fn resvg_qt_render_to_canvas(
    rtree: *const resvg_render_tree,
    opt: *const resvg_options,
    size: resvg_size,
    painter: *mut qt::qtc_qpainter,
) {
    let rtree = unsafe {
        assert!(!rtree.is_null());
        &*rtree
    };

    let painter = unsafe { qt::Painter::from_raw(painter) };
    let size = resvg::ScreenSize::new(size.width, size.height);
    let opt = to_native_opt(unsafe {
        assert!(!opt.is_null());
        &*opt
    });

    resvg::render_qt::render_to_canvas(&rtree.0, &opt, size, &painter);
}

#[cfg(feature = "cairo-backend")]
#[no_mangle]
pub extern fn resvg_cairo_render_to_canvas(
    rtree: *const resvg_render_tree,
    opt: *const resvg_options,
    size: resvg_size,
    cr: *mut cairo_sys::cairo_t,
) {
    let rtree = unsafe {
        assert!(!rtree.is_null());
        &*rtree
    };

    use glib::translate::FromGlibPtrNone;

    let cr = unsafe { cairo::Context::from_glib_none(cr) };
    let size = resvg::ScreenSize::new(size.width, size.height);

    let opt = to_native_opt(unsafe {
        assert!(!opt.is_null());
        &*opt
    });

    resvg::render_cairo::render_to_canvas(&rtree.0, &opt, size, &cr);
}

#[cfg(feature = "qt-backend")]
#[no_mangle]
pub extern fn resvg_qt_render_to_canvas_by_id(
    rtree: *const resvg_render_tree,
    opt: *const resvg_options,
    size: resvg_size,
    id: *const c_char,
    painter: *mut qt::qtc_qpainter,
) {
    let rtree = unsafe {
        assert!(!rtree.is_null());
        &*rtree
    };

    let painter = unsafe { qt::Painter::from_raw(painter) };
    let size = resvg::ScreenSize::new(size.width, size.height);
    let opt = to_native_opt(unsafe {
        assert!(!opt.is_null());
        &*opt
    });

    let id = match cstr_to_str(id) {
        Some(v) => v,
        None => return,
    };

    if id.is_empty() {
        warn!("Node with an empty ID can not be painted.");
        return;
    }

    if let Some(node) = rtree.0.node_by_svg_id(id) {
        if let Some(bbox) = resvg::render_qt::calc_node_bbox(&node, &opt) {
            let vbox = tree::ViewBox {
                rect: bbox,
                aspect: tree::AspectRatio::default(),
            };

            resvg::render_qt::render_node_to_canvas(&node, &opt, vbox, size, &painter);
        } else {
            warn!("A node with '{}' ID doesn't have a valid bounding box.", id);
        }
    } else {
        warn!("A node with '{}' ID wasn't found.", id);
    }
}

#[cfg(feature = "cairo-backend")]
#[no_mangle]
pub extern fn resvg_cairo_render_to_canvas_by_id(
    rtree: *const resvg_render_tree,
    opt: *const resvg_options,
    size: resvg_size,
    id: *const c_char,
    cr: *mut cairo_sys::cairo_t,
) {
    let rtree = unsafe {
        assert!(!rtree.is_null());
        &*rtree
    };

    let id = match cstr_to_str(id) {
        Some(v) => v,
        None => return,
    };

    if id.is_empty() {
        warn!("Node with an empty ID can not be painted.");
        return;
    }

    use glib::translate::FromGlibPtrNone;

    let cr = unsafe { cairo::Context::from_glib_none(cr) };
    let size = resvg::ScreenSize::new(size.width, size.height);

    let opt = to_native_opt(unsafe {
        assert!(!opt.is_null());
        &*opt
    });

    if let Some(node) = rtree.0.node_by_svg_id(id) {
        if let Some(bbox) = resvg::render_cairo::calc_node_bbox(&node, &opt) {
            let vbox = tree::ViewBox {
                rect: bbox,
                aspect: tree::AspectRatio::default(),
            };

            resvg::render_cairo::render_node_to_canvas(&node, &opt, vbox, size, &cr);
        } else {
            warn!("A node with '{}' ID doesn't have a valid bounding box.", id);
        }
    } else {
        warn!("A node with '{}' ID wasn't found.", id);
    }
}

#[no_mangle]
pub extern fn resvg_get_image_size(
    rtree: *const resvg_render_tree,
) -> resvg_size {
    let rtree = unsafe {
        assert!(!rtree.is_null());
        &*rtree
    };

    let size = rtree.0.svg_node().size;

    resvg_size {
        width: size.width as u32,
        height: size.height as u32,
    }
}

#[no_mangle]
pub extern fn resvg_get_image_viewbox(
    rtree: *const resvg_render_tree,
) -> resvg_rect {
    let rtree = unsafe {
        assert!(!rtree.is_null());
        &*rtree
    };

    let r = rtree.0.svg_node().view_box.rect;

    resvg_rect {
        x: r.x(),
        y: r.y(),
        width: r.width(),
        height: r.height(),
    }
}

#[cfg(feature = "qt-backend")]
#[no_mangle]
pub extern fn resvg_qt_get_node_bbox(
    rtree: *const resvg_render_tree,
    opt: *const resvg_options,
    id: *const c_char,
    bbox: *mut resvg_rect,
) -> bool {
    let backend = Box::new(resvg::render_qt::Backend);
    get_node_bbox(rtree, opt, id, bbox, backend)
}

#[cfg(feature = "cairo-backend")]
#[no_mangle]
pub extern fn resvg_cairo_get_node_bbox(
    rtree: *const resvg_render_tree,
    opt: *const resvg_options,
    id: *const c_char,
    bbox: *mut resvg_rect,
) -> bool {
    let backend = Box::new(resvg::render_cairo::Backend);
    get_node_bbox(rtree, opt, id, bbox, backend)
}

fn get_node_bbox(
    rtree: *const resvg_render_tree,
    opt: *const resvg_options,
    id: *const c_char,
    bbox: *mut resvg_rect,
    backend: Box<resvg::Render>,
) -> bool {
    let id = match cstr_to_str(id) {
        Some(v) => v,
        None => {
            warn!("Provided ID is no an UTF-8 string.");
            return false;
        }
    };

    if id.is_empty() {
        warn!("Node ID must not be empty.");
        return false;
    }

    let rtree = unsafe {
        assert!(!rtree.is_null());
        &*rtree
    };


    let opt = to_native_opt(unsafe {
        assert!(!opt.is_null());
        &*opt
    });

    match rtree.0.node_by_svg_id(id) {
        Some(node) => {
            if let Some(r) = backend.calc_node_bbox(&node, &opt) {
                unsafe {
                    (*bbox).x = r.x();
                    (*bbox).y = r.y();
                    (*bbox).width = r.width();
                    (*bbox).height = r.height();
                }

                true
            } else {
                false
            }
        }
        None => {
            warn!("No node with '{}' ID in the tree.", id);
            false
        }
    }
}

#[no_mangle]
pub extern fn resvg_node_exists(
    rtree: *const resvg_render_tree,
    id: *const c_char,
) -> bool {
    let id = match cstr_to_str(id) {
        Some(v) => v,
        None => {
            warn!("Provided ID is no an UTF-8 string.");
            return false;
        }
    };

    let rtree = unsafe {
        assert!(!rtree.is_null());
        &*rtree
    };

    rtree.0.node_by_svg_id(id).is_some()
}

#[no_mangle]
pub extern fn resvg_get_node_transform(
    rtree: *const resvg_render_tree,
    id: *const c_char,
    ts: *mut resvg_transform,
) -> bool {
    let id = match cstr_to_str(id) {
        Some(v) => v,
        None => {
            warn!("Provided ID is no an UTF-8 string.");
            return false;
        }
    };

    let rtree = unsafe {
        assert!(!rtree.is_null());
        &*rtree
    };

    if let Some(node) = rtree.0.node_by_svg_id(id) {
        let abs_ts = resvg::utils::abs_transform(&node);

        unsafe {
            (*ts).a = abs_ts.a;
            (*ts).b = abs_ts.b;
            (*ts).c = abs_ts.c;
            (*ts).d = abs_ts.d;
            (*ts).e = abs_ts.e;
            (*ts).f = abs_ts.f;
        }

        return true;
    }

    false
}

fn cstr_to_str(text: *const c_char) -> Option<&'static str> {
    let text = unsafe {
        assert!(!text.is_null());
        CStr::from_ptr(text)
    };

    text.to_str().ok()
}

fn to_native_opt(opt: &resvg_options) -> resvg::Options {
    let mut path: Option<path::PathBuf> = None;

    if !opt.path.is_null() {
        if let Some(p) = cstr_to_str(opt.path) {
            if !p.is_empty() {
                path = Some(p.into());
            }
        }
    };

    let fit_to = match opt.fit_to.kind {
        resvg_fit_to_type::RESVG_FIT_TO_ORIGINAL => {
            resvg::FitTo::Original
        }
        resvg_fit_to_type::RESVG_FIT_TO_WIDTH => {
            assert!(opt.fit_to.value > 0.0);
            resvg::FitTo::Width(opt.fit_to.value as u32)
        }
        resvg_fit_to_type::RESVG_FIT_TO_HEIGHT => {
            assert!(opt.fit_to.value > 0.0);
            resvg::FitTo::Height(opt.fit_to.value as u32)
        }
        resvg_fit_to_type::RESVG_FIT_TO_ZOOM => {
            assert!(opt.fit_to.value > 0.0);
            resvg::FitTo::Zoom(opt.fit_to.value)
        }
    };

    let background = if opt.draw_background {
        Some(resvg::tree::Color::new(
            opt.background.r,
            opt.background.g,
            opt.background.b,
        ))
    } else {
        None
    };

    resvg::Options {
        usvg: usvg::Options {
            path,
            dpi: opt.dpi,
            keep_named_groups: opt.keep_named_groups,
        },
        fit_to,
        background,
    }
}
