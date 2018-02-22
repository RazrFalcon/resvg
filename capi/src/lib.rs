// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate resvg;
extern crate log;
extern crate fern;

#[cfg(feature = "cairo-backend")]
extern crate glib;
#[cfg(feature = "cairo-backend")]
extern crate cairo_sys;

use std::fmt;
use std::ptr;
use std::ffi::{
    CStr,
    CString,
};
use std::os::raw::{
    c_char,
};

#[cfg(feature = "qt-backend")]
use resvg::qt;

#[cfg(feature = "cairo-backend")]
use resvg::cairo;

use resvg::RectExt;
use resvg::tree::prelude::*;

#[repr(C)]
pub struct Rect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[repr(C)]
pub struct resvg_render_tree(resvg::tree::RenderTree);

macro_rules! on_err {
    ($err:expr, $msg:expr) => ({
        let c_str = CString::new($msg).unwrap();
        unsafe { *$err = c_str.into_raw(); }
        return ptr::null_mut();
    })
}

macro_rules! from_raw_str {
    ($raw_str:expr, $err:expr, $msg:expr) => ({
        let rstr = unsafe {
            assert!(!$raw_str.is_null());
            CStr::from_ptr($raw_str)
        };

        let rstr = match rstr.to_str() {
            Ok(rstr) => rstr,
            Err(_) => on_err!($err, $msg),
        };

        rstr
    })
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
    dpi: f64,
    error: *mut *mut c_char,
) -> *mut resvg_render_tree {
    let file_path = from_raw_str!(
        file_path,
        error,
        "Error: the file path is not an UTF-8 string."
    );

    let opt = resvg::Options {
        path: Some(file_path.into()),
        dpi: dpi,
        .. resvg::Options::default()
    };

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
    dpi: f64,
    error: *mut *mut c_char,
) -> *mut resvg_render_tree {
    let text = from_raw_str!(
        text,
        error,
        "Error: the SVG data is not an UTF-8 string."
    );

    let opt = resvg::Options {
        dpi: dpi,
        .. resvg::Options::default()
    };

    let rtree = match resvg::parse_rtree_from_data(text, &opt) {
        Ok(rtree) => rtree,
        Err(e) => on_err!(error, e.to_string()),
    };

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
pub extern fn resvg_qt_render_to_canvas(
    rtree: *mut resvg_render_tree,
    view: Rect,
    painter: *mut qt::qtc_qpainter,
) {
    let rtree = unsafe {
        assert!(!rtree.is_null());
        &mut *rtree
    };

    let painter = unsafe { qt::Painter::from_raw(painter) };
    let rect = resvg::Rect::from_xywh(view.x, view.y, view.width, view.height);

    // TODO: to a proper options
    let opt = resvg::Options::default();

    resvg::render_qt::render_to_canvas(&rtree.0, &opt, rect, &painter);
}

#[cfg(feature = "cairo-backend")]
#[no_mangle]
pub extern fn resvg_cairo_render_to_canvas(
    rtree: *mut resvg_render_tree,
    view: Rect,
    cr: *mut cairo_sys::cairo_t,
) {
    let rtree = unsafe {
        assert!(!rtree.is_null());
        &mut *rtree
    };

    use glib::translate::FromGlibPtrNone;

    let cr = unsafe { cairo::Context::from_glib_none(cr) };
    let rect = resvg::Rect::from_xywh(view.x, view.y, view.width, view.height);

    // TODO: to a proper options
    let opt = resvg::Options::default();

    resvg::render_cairo::render_to_canvas(&rtree.0, &opt, rect, &cr);
}

#[no_mangle]
pub extern fn resvg_get_image_size(
    rtree: *mut resvg_render_tree,
    width: *mut f64,
    height: *mut f64,
) {
    let rtree = unsafe {
        assert!(!rtree.is_null());
        &mut *rtree
    };

    let size = rtree.0.svg_node().size;

    unsafe {
        *width = size.width;
        *height = size.height;
    }
}
