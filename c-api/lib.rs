// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! C bindings.

#![allow(non_camel_case_types)]
#![warn(missing_docs)]
#![warn(missing_copy_implementations)]

use std::ffi::CStr;
use std::os::raw::c_char;
use std::slice;

use resvg::tiny_skia;
use resvg::usvg::{self, NodeExt};
#[cfg(feature = "text")]
use resvg::usvg_text_layout::{fontdb, TreeTextToPath};

/// @brief List of possible errors.
#[repr(C)]
#[derive(Copy, Clone)]
pub enum resvg_error {
    /// Everything is ok.
    OK = 0,
    /// Only UTF-8 content are supported.
    NOT_AN_UTF8_STR,
    /// Failed to open the provided file.
    FILE_OPEN_FAILED,
    /// Compressed SVG must use the GZip algorithm.
    MALFORMED_GZIP,
    /// We do not allow SVG with more than 1_000_000 elements for security reasons.
    ELEMENTS_LIMIT_REACHED,
    /// SVG doesn't have a valid size.
    ///
    /// Occurs when width and/or height are <= 0.
    ///
    /// Also occurs if width, height and viewBox are not set.
    INVALID_SIZE,
    /// Failed to parse an SVG data.
    PARSING_FAILED,
}

/// @brief A path bbox representation.
///
/// Width *or* height are guarantee to be > 0.
#[repr(C)]
#[allow(missing_docs)]
#[derive(Copy, Clone)]
pub struct resvg_path_bbox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// @brief A rectangle representation.
///
/// Width *and* height are guarantee to be > 0.
#[repr(C)]
#[allow(missing_docs)]
#[derive(Copy, Clone)]
pub struct resvg_rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// @brief A size representation.
///
/// Width and height are guarantee to be > 0.
#[repr(C)]
#[allow(missing_docs)]
#[derive(Copy, Clone)]
pub struct resvg_size {
    pub width: f64,
    pub height: f64,
}

/// @brief A 2D transform representation.
#[repr(C)]
#[allow(missing_docs)]
#[derive(Copy, Clone)]
pub struct resvg_transform {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl resvg_transform {
    #[inline]
    fn to_tiny_skia(&self) -> tiny_skia::Transform {
        tiny_skia::Transform::from_row(
            self.a as f32,
            self.b as f32,
            self.c as f32,
            self.d as f32,
            self.e as f32,
            self.f as f32,
        )
    }
}

/// @brief A "fit to" type.
///
/// All types produce proportional scaling.
#[repr(C)]
#[derive(Copy, Clone)]
pub enum resvg_fit_to_type {
    /// Use an original image size.
    ORIGINAL,
    /// Fit an image to a specified width.
    WIDTH,
    /// Fit an image to a specified height.
    HEIGHT,
    /// Zoom an image using scaling factor.
    ZOOM,
}

/// @brief A "fit to" property.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct resvg_fit_to {
    /// A fit type.
    kind: resvg_fit_to_type,
    /// @brief Fit to value
    ///
    /// Not used by RESVG_FIT_TO_ORIGINAL.
    /// Must be >= 1 for RESVG_FIT_TO_WIDTH and RESVG_FIT_TO_HEIGHT.
    /// Must be > 0 for RESVG_FIT_TO_ZOOM.
    value: f32,
}

impl resvg_fit_to {
    #[inline]
    fn to_usvg(&self) -> usvg::FitTo {
        match self.kind {
            resvg_fit_to_type::ORIGINAL => usvg::FitTo::Original,
            resvg_fit_to_type::WIDTH => {
                assert!(self.value >= 1.0);
                usvg::FitTo::Width(self.value as u32)
            }
            resvg_fit_to_type::HEIGHT => {
                assert!(self.value >= 1.0);
                usvg::FitTo::Height(self.value as u32)
            }
            resvg_fit_to_type::ZOOM => usvg::FitTo::Zoom(self.value),
        }
    }
}

/// @brief Creates an identity transform.
#[no_mangle]
pub extern "C" fn resvg_transform_identity() -> resvg_transform {
    resvg_transform {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    }
}

/// @brief Initializes the library log.
///
/// Use it if you want to see any warnings.
///
/// Must be called only once.
///
/// All warnings will be printed to the `stderr`.
#[no_mangle]
pub extern "C" fn resvg_init_log() {
    if let Ok(()) = log::set_logger(&LOGGER) {
        log::set_max_level(log::LevelFilter::Warn);
    }
}

/// @brief An SVG to #resvg_render_tree conversion options.
///
/// Also, contains a fonts database used during text to path conversion.
/// The database is empty by default.
pub struct resvg_options {
    options: usvg::Options,
    #[cfg(feature = "text")]
    fontdb: fontdb::Database,
}

/// @brief Creates a new #resvg_options object.
///
/// Should be destroyed via #resvg_options_destroy.
#[no_mangle]
pub extern "C" fn resvg_options_create() -> *mut resvg_options {
    Box::into_raw(Box::new(resvg_options {
        options: usvg::Options::default(),
        #[cfg(feature = "text")]
        fontdb: fontdb::Database::new(),
    }))
}

#[inline]
fn cast_opt(opt: *mut resvg_options) -> &'static mut usvg::Options {
    unsafe {
        assert!(!opt.is_null());
        &mut (*opt).options
    }
}

#[cfg(feature = "text")]
#[inline]
fn cast_fontdb(opt: *mut resvg_options) -> &'static mut fontdb::Database {
    unsafe {
        assert!(!opt.is_null());
        &mut (*opt).fontdb
    }
}

/// @brief Sets a directory that will be used during relative paths resolving.
///
/// Expected to be the same as the directory that contains the SVG file,
/// but can be set to any.
///
/// Must be UTF-8. Can be set to NULL.
///
/// Default: NULL
#[no_mangle]
pub extern "C" fn resvg_options_set_resources_dir(opt: *mut resvg_options, path: *const c_char) {
    if path.is_null() {
        cast_opt(opt).resources_dir = None;
    } else {
        cast_opt(opt).resources_dir = Some(cstr_to_str(path).unwrap().into());
    }
}

/// @brief Sets the target DPI.
///
/// Impact units conversion.
///
/// Default: 96
#[no_mangle]
pub extern "C" fn resvg_options_set_dpi(opt: *mut resvg_options, dpi: f64) {
    cast_opt(opt).dpi = dpi;
}

/// @brief Sets the default font family.
///
/// Will be used when no `font-family` attribute is set in the SVG.
///
/// Must be UTF-8. NULL is not allowed.
///
/// Default: Times New Roman
#[no_mangle]
pub extern "C" fn resvg_options_set_font_family(opt: *mut resvg_options, family: *const c_char) {
    cast_opt(opt).font_family = cstr_to_str(family).unwrap().to_string();
}

/// @brief Sets the default font size.
///
/// Will be used when no `font-size` attribute is set in the SVG.
///
/// Default: 12
#[no_mangle]
pub extern "C" fn resvg_options_set_font_size(opt: *mut resvg_options, size: f64) {
    cast_opt(opt).font_size = size;
}

/// @brief Sets the `serif` font family.
///
/// Must be UTF-8. NULL is not allowed.
///
/// Has no effect when the `text` feature is not enabled.
///
/// Default: Times New Roman
#[no_mangle]
#[allow(unused_variables)]
pub extern "C" fn resvg_options_set_serif_family(opt: *mut resvg_options, family: *const c_char) {
    #[cfg(feature = "text")]
    {
        cast_fontdb(opt).set_serif_family(cstr_to_str(family).unwrap().to_string());
    }
}

/// @brief Sets the `sans-serif` font family.
///
/// Must be UTF-8. NULL is not allowed.
///
/// Has no effect when the `text` feature is not enabled.
///
/// Default: Arial
#[no_mangle]
#[allow(unused_variables)]
pub extern "C" fn resvg_options_set_sans_serif_family(
    opt: *mut resvg_options,
    family: *const c_char,
) {
    #[cfg(feature = "text")]
    {
        cast_fontdb(opt).set_sans_serif_family(cstr_to_str(family).unwrap().to_string());
    }
}

/// @brief Sets the `cursive` font family.
///
/// Must be UTF-8. NULL is not allowed.
///
/// Has no effect when the `text` feature is not enabled.
///
/// Default: Comic Sans MS
#[no_mangle]
#[allow(unused_variables)]
pub extern "C" fn resvg_options_set_cursive_family(opt: *mut resvg_options, family: *const c_char) {
    #[cfg(feature = "text")]
    {
        cast_fontdb(opt).set_cursive_family(cstr_to_str(family).unwrap().to_string());
    }
}

/// @brief Sets the `fantasy` font family.
///
/// Must be UTF-8. NULL is not allowed.
///
/// Has no effect when the `text` feature is not enabled.
///
/// Default: Papyrus on macOS, Impact on other OS'es
#[no_mangle]
#[allow(unused_variables)]
pub extern "C" fn resvg_options_set_fantasy_family(opt: *mut resvg_options, family: *const c_char) {
    #[cfg(feature = "text")]
    {
        cast_fontdb(opt).set_fantasy_family(cstr_to_str(family).unwrap().to_string());
    }
}

/// @brief Sets the `monospace` font family.
///
/// Must be UTF-8. NULL is not allowed.
///
/// Has no effect when the `text` feature is not enabled.
///
/// Default: Courier New
#[no_mangle]
#[allow(unused_variables)]
pub extern "C" fn resvg_options_set_monospace_family(
    opt: *mut resvg_options,
    family: *const c_char,
) {
    #[cfg(feature = "text")]
    {
        cast_fontdb(opt).set_monospace_family(cstr_to_str(family).unwrap().to_string());
    }
}

/// @brief Sets a comma-separated list of languages.
///
/// Will be used to resolve a `systemLanguage` conditional attribute.
///
/// Example: en,en-US.
///
/// Must be UTF-8. Can be NULL.
///
/// Default: en
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

/// @brief A shape rendering method.
#[repr(C)]
#[allow(missing_docs)]
#[derive(Copy, Clone)]
pub enum resvg_shape_rendering {
    OPTIMIZE_SPEED,
    CRISP_EDGES,
    GEOMETRIC_PRECISION,
}

/// @brief Sets the default shape rendering method.
///
/// Will be used when an SVG element's `shape-rendering` property is set to `auto`.
///
/// Default: `RESVG_SHAPE_RENDERING_GEOMETRIC_PRECISION`
#[no_mangle]
pub extern "C" fn resvg_options_set_shape_rendering_mode(
    opt: *mut resvg_options,
    mode: resvg_shape_rendering,
) {
    cast_opt(opt).shape_rendering = match mode as i32 {
        0 => usvg::ShapeRendering::OptimizeSpeed,
        1 => usvg::ShapeRendering::CrispEdges,
        2 => usvg::ShapeRendering::GeometricPrecision,
        _ => return,
    }
}

/// @brief A text rendering method.
#[repr(C)]
#[allow(missing_docs)]
#[derive(Copy, Clone)]
pub enum resvg_text_rendering {
    OPTIMIZE_SPEED,
    OPTIMIZE_LEGIBILITY,
    GEOMETRIC_PRECISION,
}

/// @brief Sets the default text rendering method.
///
/// Will be used when an SVG element's `text-rendering` property is set to `auto`.
///
/// Default: `RESVG_TEXT_RENDERING_OPTIMIZE_LEGIBILITY`
#[no_mangle]
pub extern "C" fn resvg_options_set_text_rendering_mode(
    opt: *mut resvg_options,
    mode: resvg_text_rendering,
) {
    cast_opt(opt).text_rendering = match mode as i32 {
        0 => usvg::TextRendering::OptimizeSpeed,
        1 => usvg::TextRendering::OptimizeLegibility,
        2 => usvg::TextRendering::GeometricPrecision,
        _ => return,
    }
}

/// @brief A image rendering method.
#[repr(C)]
#[allow(missing_docs)]
#[derive(Copy, Clone)]
pub enum resvg_image_rendering {
    OPTIMIZE_QUALITY,
    OPTIMIZE_SPEED,
}

/// @brief Sets the default image rendering method.
///
/// Will be used when an SVG element's `image-rendering` property is set to `auto`.
///
/// Default: `RESVG_IMAGE_RENDERING_OPTIMIZE_QUALITY`
#[no_mangle]
pub extern "C" fn resvg_options_set_image_rendering_mode(
    opt: *mut resvg_options,
    mode: resvg_image_rendering,
) {
    cast_opt(opt).image_rendering = match mode as i32 {
        0 => usvg::ImageRendering::OptimizeQuality,
        1 => usvg::ImageRendering::OptimizeSpeed,
        _ => return,
    }
}

/// @brief Loads a font data into the internal fonts database.
///
/// Prints a warning into the log when the data is not a valid TrueType font.
///
/// Has no effect when the `text` feature is not enabled.
#[no_mangle]
#[allow(unused_variables)]
pub extern "C" fn resvg_options_load_font_data(
    opt: *mut resvg_options,
    data: *const c_char,
    len: usize,
) {
    #[cfg(feature = "text")]
    {
        let data = unsafe { slice::from_raw_parts(data as *const u8, len) };

        let opt = unsafe {
            assert!(!opt.is_null());
            &mut *opt
        };

        opt.fontdb.load_font_data(data.to_vec())
    }
}

/// @brief Loads a font file into the internal fonts database.
///
/// Prints a warning into the log when the data is not a valid TrueType font.
///
/// Has no effect when the `text` feature is not enabled.
///
/// @return #resvg_error with RESVG_OK, RESVG_ERROR_NOT_AN_UTF8_STR or RESVG_ERROR_FILE_OPEN_FAILED
#[no_mangle]
#[allow(unused_variables)]
pub extern "C" fn resvg_options_load_font_file(
    opt: *mut resvg_options,
    file_path: *const c_char,
) -> i32 {
    #[cfg(feature = "text")]
    {
        let file_path = match cstr_to_str(file_path) {
            Some(v) => v,
            None => return resvg_error::NOT_AN_UTF8_STR as i32,
        };

        let opt = unsafe {
            assert!(!opt.is_null());
            &mut *opt
        };

        if opt.fontdb.load_font_file(file_path).is_ok() {
            resvg_error::OK as i32
        } else {
            resvg_error::FILE_OPEN_FAILED as i32
        }
    }

    #[cfg(not(feature = "text"))]
    {
        resvg_error::OK as i32
    }
}

/// @brief Loads system fonts into the internal fonts database.
///
/// This method is very IO intensive.
///
/// This method should be executed only once per #resvg_options.
///
/// The system scanning is not perfect, so some fonts may be omitted.
/// Please send a bug report in this case.
///
/// Prints warnings into the log.
///
/// Has no effect when the `text` feature is not enabled.
#[no_mangle]
#[allow(unused_variables)]
pub extern "C" fn resvg_options_load_system_fonts(opt: *mut resvg_options) {
    #[cfg(feature = "text")]
    {
        let opt = unsafe {
            assert!(!opt.is_null());
            &mut *opt
        };

        opt.fontdb.load_system_fonts();
    }
}

/// @brief Destroys the #resvg_options.
#[no_mangle]
pub extern "C" fn resvg_options_destroy(opt: *mut resvg_options) {
    unsafe {
        assert!(!opt.is_null());
        Box::from_raw(opt)
    };
}

/// @brief An opaque pointer to the rendering tree.
pub struct resvg_render_tree(pub usvg::Tree);

/// @brief Creates #resvg_render_tree from file.
///
/// .svg and .svgz files are supported.
///
/// See #resvg_is_image_empty for details.
///
/// @param file_path UTF-8 file path.
/// @param opt Rendering options. Must not be NULL.
/// @param tree Parsed render tree. Should be destroyed via #resvg_tree_destroy.
/// @return #resvg_error
#[no_mangle]
pub extern "C" fn resvg_parse_tree_from_file(
    file_path: *const c_char,
    opt: *const resvg_options,
    tree: *mut *mut resvg_render_tree,
) -> i32 {
    let file_path = match cstr_to_str(file_path) {
        Some(v) => v,
        None => return resvg_error::NOT_AN_UTF8_STR as i32,
    };

    let raw_opt = unsafe {
        assert!(!opt.is_null());
        &*opt
    };

    let file_data = match std::fs::read(file_path) {
        Ok(tree) => tree,
        Err(_) => return resvg_error::FILE_OPEN_FAILED as i32,
    };

    #[allow(unused_mut)]
    let mut utree = match usvg::Tree::from_data(&file_data, &raw_opt.options) {
        Ok(tree) => tree,
        Err(e) => return convert_error(e) as i32,
    };

    #[cfg(feature = "text")]
    {
        utree.convert_text(&raw_opt.fontdb);
    }

    let tree_box = Box::new(resvg_render_tree(utree));
    unsafe {
        *tree = Box::into_raw(tree_box);
    }

    resvg_error::OK as i32
}

/// @brief Creates #resvg_render_tree from data.
///
/// See #resvg_is_image_empty for details.
///
/// @param data SVG data. Can contain SVG string or gzip compressed data. Must not be NULL.
/// @param len Data length.
/// @param opt Rendering options. Must not be NULL.
/// @param tree Parsed render tree. Should be destroyed via #resvg_tree_destroy.
/// @return #resvg_error
#[no_mangle]
pub extern "C" fn resvg_parse_tree_from_data(
    data: *const c_char,
    len: usize,
    opt: *const resvg_options,
    tree: *mut *mut resvg_render_tree,
) -> i32 {
    let data = unsafe { slice::from_raw_parts(data as *const u8, len) };

    let raw_opt = unsafe {
        assert!(!opt.is_null());
        &*opt
    };

    #[allow(unused_mut)]
    let mut utree = match usvg::Tree::from_data(data, &raw_opt.options) {
        Ok(tree) => tree,
        Err(e) => return convert_error(e) as i32,
    };

    #[cfg(feature = "text")]
    {
        utree.convert_text(&raw_opt.fontdb);
    }

    let tree_box = Box::new(resvg_render_tree(utree));
    unsafe {
        *tree = Box::into_raw(tree_box);
    }

    resvg_error::OK as i32
}

/// @brief Checks that tree has any nodes.
///
/// @param tree Render tree.
/// @return Returns `true` if tree has no nodes.
#[no_mangle]
pub extern "C" fn resvg_is_image_empty(tree: *const resvg_render_tree) -> bool {
    let tree = unsafe {
        assert!(!tree.is_null());
        &*tree
    };

    !tree.0.root.has_children()
}

/// @brief Returns an image size.
///
/// The size of a canvas that required to render this SVG.
///
/// The `width` and `height` attributes in SVG.
///
/// @param tree Render tree.
/// @return Image size.
#[no_mangle]
pub extern "C" fn resvg_get_image_size(tree: *const resvg_render_tree) -> resvg_size {
    let tree = unsafe {
        assert!(!tree.is_null());
        &*tree
    };

    let size = tree.0.size;

    resvg_size {
        width: size.width(),
        height: size.height(),
    }
}

/// @brief Returns an image viewbox.
///
/// The `viewBox` attribute in SVG.
///
/// @param tree Render tree.
/// @return Image viewbox.
#[no_mangle]
pub extern "C" fn resvg_get_image_viewbox(tree: *const resvg_render_tree) -> resvg_rect {
    let tree = unsafe {
        assert!(!tree.is_null());
        &*tree
    };

    let r = tree.0.view_box.rect;

    resvg_rect {
        x: r.x(),
        y: r.y(),
        width: r.width(),
        height: r.height(),
    }
}

/// @brief Returns an image bounding box.
///
/// Can be smaller or bigger than a `viewbox`.
///
/// @param tree Render tree.
/// @param bbox Image's bounding box.
/// @return `false` if an image has no elements.
#[no_mangle]
pub extern "C" fn resvg_get_image_bbox(
    tree: *const resvg_render_tree,
    bbox: *mut resvg_rect,
) -> bool {
    let tree = unsafe {
        assert!(!tree.is_null());
        &*tree
    };

    if let Some(r) = tree.0.root.calculate_bbox().and_then(|r| r.to_rect()) {
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

/// @brief Returns `true` if a renderable node with such an ID exists.
///
/// @param tree Render tree.
/// @param id Node's ID. UTF-8 string. Must not be NULL.
/// @return `true` if a node exists.
/// @return `false` if a node doesn't exist or ID isn't a UTF-8 string.
/// @return `false` if a node exists, but not renderable.
#[no_mangle]
pub extern "C" fn resvg_node_exists(tree: *const resvg_render_tree, id: *const c_char) -> bool {
    let id = match cstr_to_str(id) {
        Some(v) => v,
        None => {
            log::warn!("Provided ID is no an UTF-8 string.");
            return false;
        }
    };

    let tree = unsafe {
        assert!(!tree.is_null());
        &*tree
    };

    tree.0.node_by_id(id).is_some()
}

/// @brief Returns node's transform by ID.
///
/// @param tree Render tree.
/// @param id Node's ID. UTF-8 string. Must not be NULL.
/// @param transform Node's transform.
/// @return `true` if a node exists.
/// @return `false` if a node doesn't exist or ID isn't a UTF-8 string.
/// @return `false` if a node exists, but not renderable.
#[no_mangle]
pub extern "C" fn resvg_get_node_transform(
    tree: *const resvg_render_tree,
    id: *const c_char,
    transform: *mut resvg_transform,
) -> bool {
    let id = match cstr_to_str(id) {
        Some(v) => v,
        None => {
            log::warn!("Provided ID is no an UTF-8 string.");
            return false;
        }
    };

    let tree = unsafe {
        assert!(!tree.is_null());
        &*tree
    };

    if let Some(node) = tree.0.node_by_id(id) {
        let abs_ts = node.abs_transform();

        unsafe {
            *transform = resvg_transform {
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

/// @brief Returns node's bounding box by ID.
///
/// @param tree Render tree.
/// @param id Node's ID. Must not be NULL.
/// @param bbox Node's bounding box.
/// @return `false` if a node with such an ID does not exist
/// @return `false` if ID isn't a UTF-8 string.
/// @return `false` if ID is an empty string
#[no_mangle]
pub extern "C" fn resvg_get_node_bbox(
    tree: *const resvg_render_tree,
    id: *const c_char,
    bbox: *mut resvg_path_bbox,
) -> bool {
    let id = match cstr_to_str(id) {
        Some(v) => v,
        None => {
            log::warn!("Provided ID is no an UTF-8 string.");
            return false;
        }
    };

    if id.is_empty() {
        log::warn!("Node ID must not be empty.");
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
                    *bbox = resvg_path_bbox {
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
            log::warn!("No node with '{}' ID is in the tree.", id);
            false
        }
    }
}

/// @brief Destroys the #resvg_render_tree.
#[no_mangle]
pub extern "C" fn resvg_tree_destroy(tree: *mut resvg_render_tree) {
    unsafe {
        assert!(!tree.is_null());
        Box::from_raw(tree)
    };
}

fn cstr_to_str(text: *const c_char) -> Option<&'static str> {
    let text = unsafe {
        assert!(!text.is_null());
        CStr::from_ptr(text)
    };

    text.to_str().ok()
}

fn convert_error(e: usvg::Error) -> resvg_error {
    match e {
        usvg::Error::NotAnUtf8Str => resvg_error::NOT_AN_UTF8_STR,
        usvg::Error::MalformedGZip => resvg_error::MALFORMED_GZIP,
        usvg::Error::ElementsLimitReached => resvg_error::ELEMENTS_LIMIT_REACHED,
        usvg::Error::InvalidSize => resvg_error::INVALID_SIZE,
        usvg::Error::ParsingFailed(_) => resvg_error::PARSING_FAILED,
    }
}

/// @brief Renders the #resvg_render_tree onto the pixmap.
///
/// @param tree A render tree.
/// @param fit_to Specifies into which region SVG should be fit.
/// @param transform A root SVG transform. Can be used to position SVG inside the `pixmap`.
/// @param width Pixmap width.
/// @param height Pixmap height.
/// @param pixmap Pixmap data. Should have width*height*4 size and contain
///               premultiplied RGBA8888 pixels.
#[no_mangle]
pub extern "C" fn resvg_render(
    tree: *const resvg_render_tree,
    fit_to: resvg_fit_to,
    transform: resvg_transform,
    width: u32,
    height: u32,
    pixmap: *mut c_char,
) {
    let tree = unsafe {
        assert!(!tree.is_null());
        &*tree
    };

    let pixmap_len = width as usize * height as usize * tiny_skia::BYTES_PER_PIXEL;
    let pixmap: &mut [u8] =
        unsafe { std::slice::from_raw_parts_mut(pixmap as *mut u8, pixmap_len) };
    let pixmap = tiny_skia::PixmapMut::from_bytes(pixmap, width, height).unwrap();

    resvg::render(&tree.0, fit_to.to_usvg(), transform.to_tiny_skia(), pixmap).unwrap()
}

/// @brief Renders a Node by ID onto the image.
///
/// @param tree A render tree.
/// @param id Node's ID. Must not be NULL.
/// @param fit_to Specifies into which region the image should be fit.
/// @param transform A root SVG transform. Can be used to position SVG inside the `pixmap`.
/// @param width Pixmap width.
/// @param height Pixmap height.
/// @param pixmap Pixmap data. Should have width*height*4 size and contain
///               premultiplied RGBA8888 pixels.
/// @return `false` when `id` is not a non-empty UTF-8 string.
/// @return `false` when the selected `id` is not present.
/// @return `false` when an element has a zero bbox.
#[no_mangle]
pub extern "C" fn resvg_render_node(
    tree: *const resvg_render_tree,
    id: *const c_char,
    fit_to: resvg_fit_to,
    transform: resvg_transform,
    width: u32,
    height: u32,
    pixmap: *mut c_char,
) -> bool {
    let tree = unsafe {
        assert!(!tree.is_null());
        &*tree
    };

    let id = match cstr_to_str(id) {
        Some(v) => v,
        None => return false,
    };

    if id.is_empty() {
        log::warn!("Node with an empty ID cannot be rendered.");
        return false;
    }

    if let Some(node) = tree.0.node_by_id(id) {
        let pixmap_len = width as usize * height as usize * tiny_skia::BYTES_PER_PIXEL;
        let pixmap: &mut [u8] =
            unsafe { std::slice::from_raw_parts_mut(pixmap as *mut u8, pixmap_len) };
        let pixmap = tiny_skia::PixmapMut::from_bytes(pixmap, width, height).unwrap();

        resvg::render_node(
            &tree.0,
            &node,
            fit_to.to_usvg(),
            transform.to_tiny_skia(),
            pixmap,
        )
        .is_some()
    } else {
        log::warn!("A node with '{}' ID wasn't found.", id);
        false
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
