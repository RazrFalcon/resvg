// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![allow(non_camel_case_types)]

use std::ffi::CStr;
use std::os::raw::c_char;
use std::path;
use std::ptr;
use std::slice;

use log::warn;
use usvg::NodeExt;

const DEFAULT_FONT_FAMILY: &str = "Times New Roman";

#[repr(C)]
pub struct resvg_options {
    pub path: *const c_char,
    pub dpi: f64,
    pub font_family: *const c_char,
    pub font_size: f64,
    pub languages: *const c_char,
    pub shape_rendering: resvg_shape_rendering,
    pub text_rendering: resvg_text_rendering,
    pub image_rendering: resvg_image_rendering,
    pub keep_named_groups: bool,
}

impl resvg_options {
    pub fn to_usvg(&self) -> usvg::Options {
        let mut path: Option<path::PathBuf> = None;
        if !self.path.is_null() {
            if let Some(p) = cstr_to_str(self.path) {
                if !p.is_empty() {
                    path = Some(p.into());
                }
            }
        };

        let shape_rendering = match self.shape_rendering {
            resvg_shape_rendering::RESVG_SHAPE_RENDERING_OPTIMIZE_SPEED => {
                usvg::ShapeRendering::OptimizeSpeed
            }
            resvg_shape_rendering::RESVG_SHAPE_RENDERING_CRISP_EDGES => {
                usvg::ShapeRendering::CrispEdges
            }
            resvg_shape_rendering::RESVG_SHAPE_RENDERING_GEOMETRIC_PRECISION => {
                usvg::ShapeRendering::GeometricPrecision
            }
        };

        let text_rendering = match self.text_rendering {
            resvg_text_rendering::RESVG_TEXT_RENDERING_OPTIMIZE_SPEED => {
                usvg::TextRendering::OptimizeSpeed
            }
            resvg_text_rendering::RESVG_TEXT_RENDERING_OPTIMIZE_LEGIBILITY => {
                usvg::TextRendering::OptimizeLegibility
            }
            resvg_text_rendering::RESVG_TEXT_RENDERING_GEOMETRIC_PRECISION => {
                usvg::TextRendering::GeometricPrecision
            }
        };

        let image_rendering = match self.image_rendering {
            resvg_image_rendering::RESVG_IMAGE_RENDERING_OPTIMIZE_QUALITY => {
                usvg::ImageRendering::OptimizeQuality
            }
            resvg_image_rendering::RESVG_IMAGE_RENDERING_OPTIMIZE_SPEED => {
                usvg::ImageRendering::OptimizeSpeed
            }
        };

        let ff = DEFAULT_FONT_FAMILY;
        let font_family = match cstr_to_str(self.font_family) {
            Some(v) => {
                if v.is_empty() {
                    warn!("Provided 'font_family' option is empty. Fallback to '{}'.", ff);
                    ff
                } else {
                    v
                }
            }
            None => {
                warn!("Provided 'font_family' option is no an UTF-8 string. Fallback to '{}'.", ff);
                ff
            }
        };

        let languages_str = match cstr_to_str(self.languages) {
            Some(v) => v,
            None => {
                warn!("Provided 'languages' option is no an UTF-8 string. Fallback to 'en'.");
                "en"
            }
        };

        let mut languages = Vec::new();
        for lang in languages_str.split(',') {
            languages.push(lang.trim().to_string());
        }

        if languages.is_empty() {
            warn!("Provided 'languages' option is empty. Fallback to 'en'.");
            languages = vec!["en".to_string()]
        }

        usvg::Options {
            path,
            dpi: self.dpi,
            font_family: font_family.to_string(),
            font_size: self.font_size,
            languages,
            shape_rendering,
            text_rendering,
            image_rendering,
            keep_named_groups: self.keep_named_groups,
        }
    }
}

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
#[allow(dead_code)]
pub enum resvg_shape_rendering {
    RESVG_SHAPE_RENDERING_OPTIMIZE_SPEED,
    RESVG_SHAPE_RENDERING_CRISP_EDGES,
    RESVG_SHAPE_RENDERING_GEOMETRIC_PRECISION,
}

#[repr(C)]
#[allow(dead_code)]
pub enum resvg_text_rendering {
    RESVG_TEXT_RENDERING_OPTIMIZE_SPEED,
    RESVG_TEXT_RENDERING_OPTIMIZE_LEGIBILITY,
    RESVG_TEXT_RENDERING_GEOMETRIC_PRECISION,
}

#[repr(C)]
#[allow(dead_code)]
pub enum resvg_image_rendering {
    RESVG_IMAGE_RENDERING_OPTIMIZE_QUALITY,
    RESVG_IMAGE_RENDERING_OPTIMIZE_SPEED,
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
pub struct resvg_render_tree(pub usvg::Tree);

#[no_mangle]
pub extern "C" fn resvg_init_log() {
    if let Ok(()) = log::set_logger(&LOGGER) {
        log::set_max_level(log::LevelFilter::Warn);
    }
}

#[no_mangle]
pub extern "C" fn resvg_init_options(opt: *mut resvg_options) {
    unsafe {
        (*opt).path = ptr::null();
        (*opt).dpi = 96.0;
        (*opt).font_family = ptr::null();
        (*opt).font_size = 12.0;
        (*opt).languages = ptr::null();
        (*opt).shape_rendering = resvg_shape_rendering::RESVG_SHAPE_RENDERING_GEOMETRIC_PRECISION;
        (*opt).text_rendering = resvg_text_rendering::RESVG_TEXT_RENDERING_OPTIMIZE_LEGIBILITY;
        (*opt).image_rendering = resvg_image_rendering::RESVG_IMAGE_RENDERING_OPTIMIZE_QUALITY;
        (*opt).keep_named_groups = false;
    }
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

    let tree = match usvg::Tree::from_file(file_path, &raw_opt.to_usvg()) {
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

    let tree = match usvg::Tree::from_data(data, &raw_opt.to_usvg()) {
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
