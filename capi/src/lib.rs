// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate resvg;
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


#[repr(C)]
pub struct resvg_document(resvg::Document);

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
        .level(resvg::log::LevelFilter::Warn)
        .chain(std::io::stderr())
        .apply().unwrap();
}

fn log_format(out: fern::FormatCallback, message: &fmt::Arguments, record: &resvg::log::Record) {
    use resvg::log;

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
pub extern fn resvg_parse_doc_from_file(
    file_path: *const c_char,
    dpi: f64,
    error: *mut *mut c_char,
) -> *mut resvg_document {
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

    let doc = match resvg::parse_doc_from_file(file_path, &opt) {
        Ok(doc) => doc,
        Err(e) => on_err!(error, e.to_string()),
    };

    let doc_box = Box::new(resvg_document(doc));
    Box::into_raw(doc_box)
}

#[no_mangle]
pub extern fn resvg_parse_doc_from_data(
    text: *const c_char,
    dpi: f64,
    error: *mut *mut c_char,
) -> *mut resvg_document {
    let text = from_raw_str!(
        text,
        error,
        "Error: the SVG data is not an UTF-8 string."
    );

    let opt = resvg::Options {
        dpi: dpi,
        .. resvg::Options::default()
    };

    let doc = match resvg::parse_doc_from_data(text, &opt) {
        Ok(doc) => doc,
        Err(e) => on_err!(error, e.to_string()),
    };

    let doc_box = Box::new(resvg_document(doc));
    Box::into_raw(doc_box)
}

#[no_mangle]
pub extern fn resvg_error_msg_destroy(msg: *mut c_char) {
    unsafe {
        assert!(!msg.is_null());
        CString::from_raw(msg)
    };
}

#[no_mangle]
pub extern fn resvg_doc_destroy(doc: *mut resvg_document) {
    unsafe {
        assert!(!doc.is_null());
        Box::from_raw(doc)
    };
}

#[cfg(feature = "qt-backend")]
#[no_mangle]
pub extern fn resvg_qt_render_to_canvas(
    painter: *mut qt::qtc_qpainter,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    doc: *mut resvg_document,
) {
    let doc = unsafe {
        assert!(!doc.is_null());
        &mut *doc
    };

    let painter = unsafe { qt::Painter::from_raw(painter) };
    let rect = resvg::Rect::new(x, y, width, height);

    resvg::render_qt::render_to_canvas(&painter, rect, &doc.0);
}

#[cfg(feature = "cairo-backend")]
#[no_mangle]
pub extern fn resvg_cairo_render_to_canvas(
    cr: *mut cairo_sys::cairo_t,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    doc: *mut resvg_document,
) {
    let doc = unsafe {
        assert!(!doc.is_null());
        &mut *doc
    };

    use glib::translate::FromGlibPtrNone;

    let cr = unsafe { cairo::Context::from_glib_none(cr) };
    let rect = resvg::Rect::new(x, y, width, height);

    resvg::render_cairo::render_to_canvas(&cr, rect, &doc.0);
}

#[no_mangle]
pub extern fn resvg_get_image_size(
    doc: *mut resvg_document,
    width: *mut f64,
    height: *mut f64,
) {
    let doc = unsafe {
        assert!(!doc.is_null());
        &mut *doc
    };

    unsafe {
        *width = doc.0.size.w;
        *height = doc.0.size.h;
    }
}
