// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![allow(non_camel_case_types)]

use std::ffi::CStr;
use std::os::raw::c_char;
use std::slice;

use log::warn;
use usvg::{NodeExt, SystemFontDB};


enum ErrorId {
    Ok = 0,
    NotAnUtf8Str,
    FileOpenFailed,
    InvalidFileSuffix,
    MalformedGZip,
    InvalidSize,
    ParsingFailed,
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
pub struct resvg_options(usvg::Options);

#[repr(C)]
pub struct resvg_render_tree(pub usvg::Tree);

#[no_mangle]
pub extern "C" fn resvg_init_log() {
    if let Ok(()) = log::set_logger(&LOGGER) {
        log::set_max_level(log::LevelFilter::Warn);
    }
}

#[no_mangle]
pub extern "C" fn resvg_options_create() -> *mut resvg_options {
    Box::into_raw(Box::new(resvg_options(usvg::Options::default())))
}

#[inline]
fn cast_opt(opt: *mut resvg_options) -> &'static mut usvg::Options {
    unsafe {
        assert!(!opt.is_null());
        &mut (*opt).0
    }
}

#[no_mangle]
pub extern "C" fn resvg_options_set_file_path(opt: *mut resvg_options, path: *const c_char) {
    if path.is_null() {
        cast_opt(opt).path = None;
    } else {
        cast_opt(opt).path = Some(cstr_to_str(path).unwrap().into());
    }
}

#[no_mangle]
pub extern "C" fn resvg_options_set_dpi(opt: *mut resvg_options, dpi: f64) {
    cast_opt(opt).dpi = dpi;
}

#[no_mangle]
pub extern "C" fn resvg_options_set_font_family(opt: *mut resvg_options, family: *const c_char) {
    cast_opt(opt).font_family = cstr_to_str(family).unwrap().to_string();
}

#[no_mangle]
pub extern "C" fn resvg_options_set_font_size(opt: *mut resvg_options, font_size: f64) {
    cast_opt(opt).font_size = font_size;
}

#[no_mangle]
pub extern "C" fn resvg_options_set_serif_family(opt: *mut resvg_options, family: *const c_char) {
    cast_opt(opt).fontdb.set_serif_family(cstr_to_str(family).unwrap().to_string());
}

#[no_mangle]
pub extern "C" fn resvg_options_set_sans_serif_family(opt: *mut resvg_options, family: *const c_char) {
    cast_opt(opt).fontdb.set_sans_serif_family(cstr_to_str(family).unwrap().to_string());
}

#[no_mangle]
pub extern "C" fn resvg_options_set_cursive_family(opt: *mut resvg_options, family: *const c_char) {
    cast_opt(opt).fontdb.set_cursive_family(cstr_to_str(family).unwrap().to_string());
}

#[no_mangle]
pub extern "C" fn resvg_options_set_fantasy_family(opt: *mut resvg_options, family: *const c_char) {
    cast_opt(opt).fontdb.set_fantasy_family(cstr_to_str(family).unwrap().to_string());
}

#[no_mangle]
pub extern "C" fn resvg_options_set_monospace_family(opt: *mut resvg_options, family: *const c_char) {
    cast_opt(opt).fontdb.set_monospace_family(cstr_to_str(family).unwrap().to_string());
}

#[no_mangle]
pub extern "C" fn resvg_options_set_languages(opt: *mut resvg_options, languages: *const c_char) {
    if languages.is_null() {
        cast_opt(opt).languages = Vec::new();
        return;
    }

    let languages_str = match cstr_to_str(languages) {
        Some(v) => v,
        None => return,
    };

    let mut languages = Vec::new();
    for lang in languages_str.split(',') {
        languages.push(lang.trim().to_string());
    }

    cast_opt(opt).languages = languages;
}

#[no_mangle]
pub extern "C" fn resvg_options_set_shape_rendering_mode(opt: *mut resvg_options, mode: i32) {
    cast_opt(opt).shape_rendering = match mode {
        0 => usvg::ShapeRendering::OptimizeSpeed,
        1 => usvg::ShapeRendering::CrispEdges,
        2 => usvg::ShapeRendering::GeometricPrecision,
        _ => return,
    }
}

#[no_mangle]
pub extern "C" fn resvg_options_set_text_rendering_mode(opt: *mut resvg_options, mode: i32) {
    cast_opt(opt).text_rendering = match mode {
        0 => usvg::TextRendering::OptimizeSpeed,
        1 => usvg::TextRendering::OptimizeLegibility,
        2 => usvg::TextRendering::GeometricPrecision,
        _ => return,
    }
}

#[no_mangle]
pub extern "C" fn resvg_options_set_image_rendering_mode(opt: *mut resvg_options, mode: i32) {
    cast_opt(opt).image_rendering = match mode {
        0 => usvg::ImageRendering::OptimizeQuality,
        1 => usvg::ImageRendering::OptimizeSpeed,
        _ => return,
    }
}

#[no_mangle]
pub extern "C" fn resvg_options_set_keep_named_groups(opt: *mut resvg_options, keep: bool) {
    cast_opt(opt).keep_named_groups = keep;
}

#[no_mangle]
pub extern "C" fn resvg_options_load_system_fonts(opt: *mut resvg_options) {
    let opt = unsafe {
        assert!(!opt.is_null());
        &mut *opt
    };

    opt.0.fontdb.load_system_fonts();
}

#[no_mangle]
pub extern "C" fn resvg_options_load_font_file(
    opt: *mut resvg_options,
    file_path: *const c_char,
) -> i32 {
    let file_path = match cstr_to_str(file_path) {
        Some(v) => v,
        None => return ErrorId::NotAnUtf8Str as i32,
    };

    let opt = unsafe {
        assert!(!opt.is_null());
        &mut *opt
    };

    if opt.0.fontdb.load_font_file(file_path).is_ok() {
        ErrorId::Ok as i32
    } else {
        ErrorId::FileOpenFailed as i32
    }
}

#[no_mangle]
pub extern "C" fn resvg_options_load_font_data(
    opt: *mut resvg_options,
    data: *const c_char,
    len: usize,
) {
    let data = unsafe { slice::from_raw_parts(data as *const u8, len) };

    let opt = unsafe {
        assert!(!opt.is_null());
        &mut *opt
    };

    opt.0.fontdb.load_font_data(data.to_vec())
}

#[no_mangle]
pub extern "C" fn resvg_options_destroy(db: *mut resvg_options) {
    unsafe {
        assert!(!db.is_null());
        Box::from_raw(db)
    };
}

#[no_mangle]
pub extern "C" fn resvg_parse_tree_from_file(
    file_path: *const c_char,
    opt: *const resvg_options,
    raw_tree: *mut *mut resvg_render_tree,
) -> i32 {
    let file_path = match cstr_to_str(file_path) {
        Some(v) => v,
        None => return ErrorId::NotAnUtf8Str as i32,
    };

    let raw_opt = unsafe {
        assert!(!opt.is_null());
        &*opt
    };

    let tree = match usvg::Tree::from_file(file_path, &raw_opt.0) {
        Ok(tree) => tree,
        Err(e) => return convert_error(e) as i32,
    };

    let tree_box = Box::new(resvg_render_tree(tree));
    unsafe { *raw_tree = Box::into_raw(tree_box); }

    ErrorId::Ok as i32
}

#[no_mangle]
pub extern "C" fn resvg_parse_tree_from_data(
    data: *const c_char,
    len: usize,
    opt: *const resvg_options,
    raw_tree: *mut *mut resvg_render_tree,
) -> i32 {
    let data = unsafe { slice::from_raw_parts(data as *const u8, len) };

    let raw_opt = unsafe {
        assert!(!opt.is_null());
        &*opt
    };

    let tree = match usvg::Tree::from_data(data, &raw_opt.0) {
        Ok(tree) => tree,
        Err(e) => return convert_error(e) as i32,
    };

    let tree_box = Box::new(resvg_render_tree(tree));
    unsafe { *raw_tree = Box::into_raw(tree_box); }

    ErrorId::Ok as i32
}

#[no_mangle]
pub extern "C" fn resvg_tree_destroy(tree: *mut resvg_render_tree) {
    unsafe {
        assert!(!tree.is_null());
        Box::from_raw(tree)
    };
}

#[no_mangle]
pub extern "C" fn resvg_is_image_empty(tree: *const resvg_render_tree) -> bool {
    let tree = unsafe {
        assert!(!tree.is_null());
        &*tree
    };

    // The root/svg node should have at least two children.
    // The first child is `defs` and it always present.
    tree.0.root().children().count() > 1
}

#[no_mangle]
pub extern "C" fn resvg_get_image_size(tree: *const resvg_render_tree) -> resvg_size {
    let tree = unsafe {
        assert!(!tree.is_null());
        &*tree
    };

    let size = tree.0.svg_node().size;

    resvg_size {
        width: size.width() as u32,
        height: size.height() as u32,
    }
}

#[no_mangle]
pub extern "C" fn resvg_get_image_viewbox(tree: *const resvg_render_tree) -> resvg_rect {
    let tree = unsafe {
        assert!(!tree.is_null());
        &*tree
    };

    let r = tree.0.svg_node().view_box.rect;

    resvg_rect {
        x: r.x(),
        y: r.y(),
        width: r.width(),
        height: r.height(),
    }
}


#[no_mangle]
pub extern "C" fn resvg_get_image_bbox(
    tree: *const resvg_render_tree,
    bbox: *mut resvg_rect,
) -> bool {
    let tree = unsafe {
        assert!(!tree.is_null());
        &*tree
    };

    if let Some(r) = tree.0.root().calculate_bbox() {
        unsafe {
            *bbox = resvg_rect {
                x: r.x(),
                y: r.y(),
                width: r.width(),
                height: r.height(),
            }
        }

        true
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn resvg_get_node_bbox(
    tree: *const resvg_render_tree,
    id: *const c_char,
    bbox: *mut resvg_rect,
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

    let tree = unsafe {
        assert!(!tree.is_null());
        &*tree
    };

    match tree.0.node_by_id(id) {
        Some(node) => {
            if let Some(r) = node.calculate_bbox() {
                unsafe {
                    *bbox = resvg_rect {
                        x: r.x(),
                        y: r.y(),
                        width: r.width(),
                        height: r.height(),
                    }
                }

                true
            } else {
                false
            }
        }
        None => {
            warn!("No node with '{}' ID is in the tree.", id);
            false
        }
    }
}

#[no_mangle]
pub extern "C" fn resvg_node_exists(
    tree: *const resvg_render_tree,
    id: *const c_char,
) -> bool {
    let id = match cstr_to_str(id) {
        Some(v) => v,
        None => {
            warn!("Provided ID is no an UTF-8 string.");
            return false;
        }
    };

    let tree = unsafe {
        assert!(!tree.is_null());
        &*tree
    };

    tree.0.node_by_id(id).is_some()
}

#[no_mangle]
pub extern "C" fn resvg_get_node_transform(
    tree: *const resvg_render_tree,
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

    let tree = unsafe {
        assert!(!tree.is_null());
        &*tree
    };

    if let Some(node) = tree.0.node_by_id(id) {
        let mut abs_ts = node.abs_transform();
        abs_ts.append(&node.transform());

        unsafe {
            *ts = resvg_transform {
                a: abs_ts.a,
                b: abs_ts.b,
                c: abs_ts.c,
                d: abs_ts.d,
                e: abs_ts.e,
                f: abs_ts.f,
            }
        }

        return true;
    }

    false
}

pub fn cstr_to_str(text: *const c_char) -> Option<&'static str> {
    let text = unsafe {
        assert!(!text.is_null());
        CStr::from_ptr(text)
    };

    text.to_str().ok()
}

fn convert_error(e: usvg::Error) -> ErrorId {
    match e {
        usvg::Error::InvalidFileSuffix => ErrorId::InvalidFileSuffix,
        usvg::Error::FileOpenFailed => ErrorId::FileOpenFailed,
        usvg::Error::NotAnUtf8Str => ErrorId::NotAnUtf8Str,
        usvg::Error::MalformedGZip => ErrorId::MalformedGZip,
        usvg::Error::InvalidSize => ErrorId::InvalidSize,
        usvg::Error::ParsingFailed(_) => ErrorId::ParsingFailed,
    }
}


/// A simple stderr logger.
static LOGGER: SimpleLogger = SimpleLogger;
struct SimpleLogger;
impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::LevelFilter::Warn
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let target = if record.target().len() > 0 {
                record.target()
            } else {
                record.module_path().unwrap_or_default()
            };

            let line = record.line().unwrap_or(0);

            match record.level() {
                log::Level::Error => eprintln!("Error (in {}:{}): {}", target, line, record.args()),
                log::Level::Warn  => eprintln!("Warning (in {}:{}): {}", target, line, record.args()),
                log::Level::Info  => eprintln!("Info (in {}:{}): {}", target, line, record.args()),
                log::Level::Debug => eprintln!("Debug (in {}:{}): {}", target, line, record.args()),
                log::Level::Trace => eprintln!("Trace (in {}:{}): {}", target, line, record.args()),
            }
        }
    }

    fn flush(&self) {}
}
