// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![allow(non_camel_case_types)]

use std::ffi::CStr;
use std::mem::drop;
use std::os::raw::c_char;
use std::panic::AssertUnwindSafe;
use std::ptr;
use std::slice;
use std::str;

use usvg::NodeExt;

use macro_rules_attribute::apply;

#[allow(dead_code)]
enum Error {
    NotAnUtf8Str = 1,
    FileOpenFailed,
    MalformedGZip,
    ElementsLimitReached,
    InvalidSize,
    ParsingFailed,
    PointerIsNull,
    InvalidFitValue,
    InvalidEnumValue,
    RenderFailed,
    BboxCalcFailed,
    EmptyNodeId,
    NodeNotFound,
    PixmapCreationFailed,
    NotImplemented,
    PanicCaught,
}

macro_rules! make_c_api_call {
    (
        $(#[$fn_meta:meta])*
        $fn_vis:vis fn $fn_name:ident($( $arg_name:ident : $arg_type:ty ),* $(,)?) -> Result<(), Error>
        $fn_body:block
    ) => (
        $(#[$fn_meta])*
        #[no_mangle]
        $fn_vis extern "C" fn $fn_name($( $arg_name : $arg_type, )*) -> i32 {
            std::panic::catch_unwind(move || -> Result<(), Error> { $fn_body })
                .map(|res| match res {
                    Ok(()) => 0,
                    Err(err) => err as i32,
                })
                .unwrap_or(Error::PanicCaught as i32)
        }
    );
    (
        $(#[$fn_meta:meta])*
        $fn_vis:vis fn $fn_name:ident($( $arg_name:ident : $arg_type:ty ),* $(,)?) -> Result<$ret_type:ty, Error>
        $fn_body:block
    ) => (
        $(#[$fn_meta])*
        #[no_mangle]
        $fn_vis extern "C" fn $fn_name($( $arg_name : $arg_type, )* output: *mut $ret_type) -> i32 {
            std::panic::catch_unwind(move || -> Result<$ret_type, Error> { $fn_body })
                .map(|res| match res {
                    Ok(ret) => {
                        if output.is_null() {
                            Error::PointerIsNull as i32
                        } else {
                            unsafe { ptr::write(output, ret); }
                            0
                        }
                    },
                    Err(err) => err as i32,
                })
                .unwrap_or(Error::PanicCaught as i32)
        }
    );
}

impl From<str::Utf8Error> for Error {
    fn from(_: str::Utf8Error) -> Error {
        Error::NotAnUtf8Str
    }
}

impl From<usvg::Error> for Error {
    fn from(err: usvg::Error) -> Error {
        match err {
            usvg::Error::NotAnUtf8Str => Error::NotAnUtf8Str,
            usvg::Error::MalformedGZip => Error::MalformedGZip,
            usvg::Error::ElementsLimitReached => Error::ElementsLimitReached,
            usvg::Error::InvalidSize => Error::InvalidSize,
            usvg::Error::ParsingFailed(_) => Error::ParsingFailed,
        }
    }
}

#[inline]
fn ptr_to_ref<'a, T>(ptr: *const T) -> Result<&'a T, Error> {
    unsafe { ptr.as_ref() }.ok_or(Error::PointerIsNull)
}

#[inline]
fn ptr_to_mut<'a, T>(ptr: *mut T) -> Result<&'a mut T, Error> {
    unsafe { ptr.as_mut() }.ok_or(Error::PointerIsNull)
}

#[inline]
fn cstr_to_str(text: *const c_char) -> Result<&'static str, Error> {
    if text.is_null() {
        return Err(Error::PointerIsNull);
    }
    let text = unsafe { CStr::from_ptr(text) };

    Ok(text.to_str()?)
}

#[inline]
fn cstr_to_node_id(text: *const c_char) -> Result<&'static str, Error> {
    let text = cstr_to_str(text)?;
    if text.is_empty() {
        Err(Error::EmptyNodeId)
    } else {
        Ok(text)
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct resvg_path_bbox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct resvg_rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct resvg_size {
    pub width: f64,
    pub height: f64,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct resvg_transform {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub enum resvg_fit_to_type {
    RESVG_FIT_TO_ORIGINAL,
    RESVG_FIT_TO_WIDTH,
    RESVG_FIT_TO_HEIGHT,
    RESVG_FIT_TO_ZOOM,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct resvg_fit_to {
    kind: resvg_fit_to_type,
    value: f32,
}

impl resvg_fit_to {
    #[inline]
    fn to_usvg(&self) -> Result<usvg::FitTo, Error> {
        match self.kind {
            resvg_fit_to_type::RESVG_FIT_TO_ORIGINAL => Ok(usvg::FitTo::Original),
            resvg_fit_to_type::RESVG_FIT_TO_WIDTH => {
                if self.value >= 1.0 {
                    Ok(usvg::FitTo::Width(self.value as u32))
                } else {
                    Err(Error::InvalidFitValue)
                }
            }
            resvg_fit_to_type::RESVG_FIT_TO_HEIGHT => {
                if self.value >= 1.0 {
                    Ok(usvg::FitTo::Height(self.value as u32))
                } else {
                    Err(Error::InvalidFitValue)
                }
            }
            resvg_fit_to_type::RESVG_FIT_TO_ZOOM => Ok(usvg::FitTo::Zoom(self.value)),
        }
    }
}

#[apply(make_c_api_call!)]
pub fn resvg_init_log() -> Result<(), Error> {
    if let Ok(()) = log::set_logger(&LOGGER) {
        log::set_max_level(log::LevelFilter::Warn);
    }
    Ok(())
}

#[repr(C)]
pub struct resvg_options(AssertUnwindSafe<usvg::Options>);

#[inline]
fn cast_opt<'a>(opt: *const resvg_options) -> Result<&'a usvg::Options, Error> {
    Ok(&ptr_to_ref(opt)?.0)
}

#[inline]
fn cast_opt_mut<'a>(opt: *mut resvg_options) -> Result<&'a mut usvg::Options, Error> {
    Ok(&mut ptr_to_mut(opt)?.0)
}

#[apply(make_c_api_call!)]
pub fn resvg_options_create() -> Result<*mut resvg_options, Error> {
    Ok(Box::into_raw(Box::new(resvg_options(AssertUnwindSafe(
        usvg::Options::default(),
    )))))
}

#[apply(make_c_api_call!)]
pub fn resvg_options_set_resources_dir(
    opt: *mut resvg_options,
    path: *const c_char,
) -> Result<(), Error> {
    let opt = cast_opt_mut(opt)?;
    if path.is_null() {
        opt.resources_dir = None;
    } else {
        opt.resources_dir = Some(cstr_to_str(path)?.into());
    }
    Ok(())
}

#[apply(make_c_api_call!)]
pub fn resvg_options_set_dpi(opt: *mut resvg_options, dpi: f64) -> Result<(), Error> {
    let opt = cast_opt_mut(opt)?;
    opt.dpi = dpi;
    Ok(())
}

#[apply(make_c_api_call!)]
pub fn resvg_options_set_font_family(
    opt: *mut resvg_options,
    family: *const c_char,
) -> Result<(), Error> {
    let opt = cast_opt_mut(opt)?;
    let family = cstr_to_str(family)?;
    opt.font_family = family.to_string();
    Ok(())
}

#[apply(make_c_api_call!)]
pub fn resvg_options_set_font_size(opt: *mut resvg_options, font_size: f64) -> Result<(), Error> {
    let opt = cast_opt_mut(opt)?;
    opt.font_size = font_size;
    Ok(())
}

#[apply(make_c_api_call!)]
#[allow(unused_variables)]
pub fn resvg_options_set_serif_family(
    opt: *mut resvg_options,
    family: *const c_char,
) -> Result<(), Error> {
    #[cfg(feature = "text")]
    {
        let opt = cast_opt_mut(opt)?;
        let family = cstr_to_str(family)?;
        opt.fontdb.set_serif_family(family.to_string());
        Ok(())
    }

    #[cfg(not(feature = "text"))]
    {
        Err(Error::NotImplemented)
    }
}

#[apply(make_c_api_call!)]
#[allow(unused_variables)]
pub fn resvg_options_set_sans_serif_family(
    opt: *mut resvg_options,
    family: *const c_char,
) -> Result<(), Error> {
    #[cfg(feature = "text")]
    {
        let opt = cast_opt_mut(opt)?;
        let family = cstr_to_str(family)?;
        opt.fontdb.set_sans_serif_family(family.to_string());
        Ok(())
    }

    #[cfg(not(feature = "text"))]
    {
        Err(Error::NotImplemented)
    }
}

#[apply(make_c_api_call!)]
#[allow(unused_variables)]
pub fn resvg_options_set_cursive_family(
    opt: *mut resvg_options,
    family: *const c_char,
) -> Result<(), Error> {
    #[cfg(feature = "text")]
    {
        let opt = cast_opt_mut(opt)?;
        let family = cstr_to_str(family)?;
        opt.fontdb.set_cursive_family(family.to_string());
        Ok(())
    }

    #[cfg(not(feature = "text"))]
    {
        Err(Error::NotImplemented)
    }
}

#[apply(make_c_api_call!)]
#[allow(unused_variables)]
pub fn resvg_options_set_fantasy_family(
    opt: *mut resvg_options,
    family: *const c_char,
) -> Result<(), Error> {
    #[cfg(feature = "text")]
    {
        let opt = cast_opt_mut(opt)?;
        let family = cstr_to_str(family)?;
        opt.fontdb.set_fantasy_family(family.to_string());
        Ok(())
    }

    #[cfg(not(feature = "text"))]
    {
        Err(Error::NotImplemented)
    }
}

#[apply(make_c_api_call!)]
#[allow(unused_variables)]
pub fn resvg_options_set_monospace_family(
    opt: *mut resvg_options,
    family: *const c_char,
) -> Result<(), Error> {
    #[cfg(feature = "text")]
    {
        let opt = cast_opt_mut(opt)?;
        let family = cstr_to_str(family)?;
        opt.fontdb.set_monospace_family(family.to_string());
        Ok(())
    }

    #[cfg(not(feature = "text"))]
    {
        Err(Error::NotImplemented)
    }
}

#[apply(make_c_api_call!)]
pub fn resvg_options_set_languages(
    opt: *mut resvg_options,
    languages: *const c_char,
) -> Result<(), Error> {
    let opt = cast_opt_mut(opt)?;

    if languages.is_null() {
        opt.languages = Vec::new();
        return Ok(());
    }

    let languages_str = cstr_to_str(languages)?;

    let mut languages = Vec::new();
    for lang in languages_str.split(',') {
        languages.push(lang.trim().to_string());
    }

    opt.languages = languages;
    Ok(())
}

#[apply(make_c_api_call!)]
pub fn resvg_options_set_shape_rendering_mode(
    opt: *mut resvg_options,
    mode: i32,
) -> Result<(), Error> {
    let opt = cast_opt_mut(opt)?;
    opt.shape_rendering = match mode {
        0 => usvg::ShapeRendering::OptimizeSpeed,
        1 => usvg::ShapeRendering::CrispEdges,
        2 => usvg::ShapeRendering::GeometricPrecision,
        _ => return Err(Error::InvalidEnumValue),
    };
    Ok(())
}

#[apply(make_c_api_call!)]
pub fn resvg_options_set_text_rendering_mode(
    opt: *mut resvg_options,
    mode: i32,
) -> Result<(), Error> {
    let opt = cast_opt_mut(opt)?;
    opt.text_rendering = match mode {
        0 => usvg::TextRendering::OptimizeSpeed,
        1 => usvg::TextRendering::OptimizeLegibility,
        2 => usvg::TextRendering::GeometricPrecision,
        _ => return Err(Error::InvalidEnumValue),
    };
    Ok(())
}

#[apply(make_c_api_call!)]
pub fn resvg_options_set_image_rendering_mode(
    opt: *mut resvg_options,
    mode: i32,
) -> Result<(), Error> {
    let opt = cast_opt_mut(opt)?;
    opt.image_rendering = match mode {
        0 => usvg::ImageRendering::OptimizeQuality,
        1 => usvg::ImageRendering::OptimizeSpeed,
        _ => return Err(Error::InvalidEnumValue),
    };
    Ok(())
}

#[apply(make_c_api_call!)]
pub fn resvg_options_set_keep_named_groups(
    opt: *mut resvg_options,
    keep: bool,
) -> Result<(), Error> {
    let opt = cast_opt_mut(opt)?;
    opt.keep_named_groups = keep;
    Ok(())
}

#[apply(make_c_api_call!)]
#[allow(unused_variables)]
pub fn resvg_options_load_system_fonts(opt: *mut resvg_options) -> Result<(), Error> {
    #[cfg(feature = "text")]
    {
        let opt = cast_opt_mut(opt)?;
        opt.fontdb.load_system_fonts();
        Ok(())
    }

    #[cfg(not(feature = "text"))]
    {
        Err(Error::NotImplemented)
    }
}

#[apply(make_c_api_call!)]
#[allow(unused_variables)]
pub fn resvg_options_load_font_file(
    opt: *mut resvg_options,
    file_path: *const c_char,
) -> Result<(), Error> {
    #[cfg(feature = "text")]
    {
        let file_path = cstr_to_str(file_path)?;
        let opt = cast_opt_mut(opt)?;
        opt.fontdb
            .load_font_file(file_path)
            .map_err(|_| Error::FileOpenFailed)
    }

    #[cfg(not(feature = "text"))]
    {
        Err(Error::NotImplemented)
    }
}

#[apply(make_c_api_call!)]
#[allow(unused_variables)]
pub fn resvg_options_load_font_data(
    opt: *mut resvg_options,
    data: *const c_char,
    len: usize,
) -> Result<(), Error> {
    #[cfg(feature = "text")]
    {
        if data.is_null() {
            return Err(Error::PointerIsNull);
        }
        let data = unsafe { slice::from_raw_parts(data as *const u8, len) };
        let opt = cast_opt_mut(opt)?;
        opt.fontdb.load_font_data(data.to_vec());
        Ok(())
    }

    #[cfg(not(feature = "text"))]
    {
        Err(Error::NotImplemented)
    }
}

#[apply(make_c_api_call!)]
pub fn resvg_options_destroy(opt: *mut resvg_options) -> Result<(), Error> {
    if opt.is_null() {
        return Err(Error::PointerIsNull);
    }
    drop(unsafe { Box::from_raw(opt) });
    Ok(())
}

#[repr(C)]
pub struct resvg_render_tree(AssertUnwindSafe<usvg::Tree>);

#[inline]
fn cast_tree<'a>(tree: *const resvg_render_tree) -> Result<&'a usvg::Tree, Error> {
    Ok(&ptr_to_ref(tree)?.0)
}

#[apply(make_c_api_call!)]
pub fn resvg_parse_tree_from_file(
    file_path: *const c_char,
    opt: *const resvg_options,
) -> Result<*mut resvg_render_tree, Error> {
    let file_path = cstr_to_str(file_path)?;
    let opt = cast_opt(opt)?;
    let file_data = std::fs::read(file_path).map_err(|_| Error::FileOpenFailed)?;
    let tree = usvg::Tree::from_data(&file_data, &opt.to_ref())?;
    let tree_box = Box::new(resvg_render_tree(AssertUnwindSafe(tree)));
    Ok(Box::into_raw(tree_box))
}

#[apply(make_c_api_call!)]
pub fn resvg_parse_tree_from_data(
    data: *const c_char,
    len: usize,
    opt: *const resvg_options,
) -> Result<*mut resvg_render_tree, Error> {
    if data.is_null() {
        return Err(Error::PointerIsNull);
    }
    let data = unsafe { slice::from_raw_parts(data as *const u8, len) };
    let opt = cast_opt(opt)?;
    let tree = usvg::Tree::from_data(data, &opt.to_ref())?;
    let tree_box = Box::new(resvg_render_tree(AssertUnwindSafe(tree)));
    Ok(Box::into_raw(tree_box))
}

#[apply(make_c_api_call!)]
pub fn resvg_tree_destroy(tree: *mut resvg_render_tree) -> Result<(), Error> {
    if tree.is_null() {
        return Err(Error::PointerIsNull);
    }
    drop(unsafe { Box::from_raw(tree) });
    Ok(())
}

#[apply(make_c_api_call!)]
pub fn resvg_is_image_empty(tree: *const resvg_render_tree) -> Result<bool, Error> {
    let tree = cast_tree(tree)?;

    // The root/svg node should have at least two children.
    // The first child is `defs` and it always present.
    Ok(tree.root().children().count() <= 1)
}

#[apply(make_c_api_call!)]
pub fn resvg_get_image_size(tree: *const resvg_render_tree) -> Result<resvg_size, Error> {
    let tree = cast_tree(tree)?;
    let s = tree.svg_node().size;
    Ok(resvg_size {
        width: s.width(),
        height: s.height(),
    })
}

#[apply(make_c_api_call!)]
pub fn resvg_get_image_viewbox(tree: *const resvg_render_tree) -> Result<resvg_rect, Error> {
    let tree = cast_tree(tree)?;
    let r = tree.svg_node().view_box.rect;
    Ok(resvg_rect {
        x: r.x(),
        y: r.y(),
        width: r.width(),
        height: r.height(),
    })
}

#[apply(make_c_api_call!)]
pub fn resvg_get_image_bbox(tree: *const resvg_render_tree) -> Result<resvg_rect, Error> {
    let tree = cast_tree(tree)?;
    let r = tree.root().calculate_bbox().ok_or(Error::BboxCalcFailed)?;
    Ok(resvg_rect {
        x: r.x(),
        y: r.y(),
        width: r.width(),
        height: r.height(),
    })
}

#[apply(make_c_api_call!)]
pub fn resvg_get_node_bbox(
    tree: *const resvg_render_tree,
    id: *const c_char,
) -> Result<resvg_path_bbox, Error> {
    let id = cstr_to_node_id(id)?;
    let tree = cast_tree(tree)?;
    let node = tree.node_by_id(id).ok_or(Error::NodeNotFound)?;
    let r = node.calculate_bbox().ok_or(Error::BboxCalcFailed)?;
    Ok(resvg_path_bbox {
        x: r.x(),
        y: r.y(),
        width: r.width(),
        height: r.height(),
    })
}

#[apply(make_c_api_call!)]
pub fn resvg_node_exists(tree: *const resvg_render_tree, id: *const c_char) -> Result<bool, Error> {
    let id = cstr_to_node_id(id)?;
    let tree = cast_tree(tree)?;
    Ok(tree.node_by_id(id).is_some())
}

#[apply(make_c_api_call!)]
pub fn resvg_get_node_transform(
    tree: *const resvg_render_tree,
    id: *const c_char,
) -> Result<resvg_transform, Error> {
    let id = cstr_to_node_id(id)?;
    let tree = cast_tree(tree)?;
    let node = tree.node_by_id(id).ok_or(Error::NodeNotFound)?;
    let abs_ts = node.abs_transform();
    Ok(resvg_transform {
        a: abs_ts.a,
        b: abs_ts.b,
        c: abs_ts.c,
        d: abs_ts.d,
        e: abs_ts.e,
        f: abs_ts.f,
    })
}

fn create_pixmap<'a>(
    width: u32,
    height: u32,
    data: *const c_char,
) -> Result<tiny_skia::PixmapMut<'a>, Error> {
    if data.is_null() {
        return Err(Error::PointerIsNull);
    }
    if width == 0 || height == 0 {
        return Err(Error::InvalidSize);
    }
    let len = width as usize * height as usize * tiny_skia::BYTES_PER_PIXEL;
    let data: &mut [u8] = unsafe { std::slice::from_raw_parts_mut(data as *mut u8, len) };
    tiny_skia::PixmapMut::from_bytes(data, width, height).ok_or(Error::PixmapCreationFailed)
}

#[apply(make_c_api_call!)]
pub fn resvg_render(
    tree: *const resvg_render_tree,
    fit_to: resvg_fit_to,
    width: u32,
    height: u32,
    pixmap: *const c_char,
) -> Result<(), Error> {
    let tree = cast_tree(tree)?;
    let pixmap = create_pixmap(width, height, pixmap)?;
    let fit_to = fit_to.to_usvg()?;
    resvg::render(&tree, fit_to, pixmap).ok_or(Error::RenderFailed)
}

#[apply(make_c_api_call!)]
pub fn resvg_render_node(
    tree: *const resvg_render_tree,
    id: *const c_char,
    fit_to: resvg_fit_to,
    width: u32,
    height: u32,
    pixmap: *const c_char,
) -> Result<(), Error> {
    let tree = cast_tree(tree)?;
    let id = cstr_to_node_id(id)?;
    if let Some(node) = tree.node_by_id(id) {
        let pixmap = create_pixmap(width, height, pixmap)?;
        let fit_to = fit_to.to_usvg()?;
        resvg::render_node(&tree, &node, fit_to, pixmap).ok_or(Error::RenderFailed)
    } else {
        Err(Error::NodeNotFound)
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
                log::Level::Warn => {
                    eprintln!("Warning (in {}:{}): {}", target, line, record.args())
                }
                log::Level::Info => eprintln!("Info (in {}:{}): {}", target, line, record.args()),
                log::Level::Debug => eprintln!("Debug (in {}:{}): {}", target, line, record.args()),
                log::Level::Trace => eprintln!("Trace (in {}:{}): {}", target, line, record.args()),
            }
        }
    }

    fn flush(&self) {}
}
