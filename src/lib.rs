// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/*!
*resvg* is an [SVG](https://en.wikipedia.org/wiki/Scalable_Vector_Graphics) rendering library.

*resvg* can be used to render SVG files based on a
[static](http://www.w3.org/TR/SVG11/feature#SVG-static)
[SVG Full 1.1](https://www.w3.org/TR/SVG/Overview.html) subset.
In simple terms: no animations and scripting.

It can be used as a simple SVG to PNG converted.
And as an embeddable library to paint SVG on an application native canvas.
*/

#![forbid(unsafe_code)]
#![warn(missing_docs)]
//#![warn(missing_debug_implementations)]
//#![warn(missing_copy_implementations)]

// For error-chain.
#![recursion_limit="128"]

extern crate base64;
extern crate ego_tree;
extern crate euclid;
extern crate libflate;
extern crate lyon_geom;
extern crate svgdom;
#[macro_use] extern crate error_chain;
#[macro_use] extern crate log;

#[cfg(feature = "cairo-backend")] pub extern crate cairo;
#[cfg(feature = "cairo-backend")] extern crate pango;
#[cfg(feature = "cairo-backend")] extern crate pangocairo;
#[cfg(feature = "cairo-backend")] extern crate image;

#[cfg(feature = "qt-backend")] pub extern crate resvg_qt as qt;


macro_rules! try_opt {
    ($task:expr, $ret:expr) => {
        match $task {
            Some(v) => v,
            None => return $ret,
        }
    };
}

/// Task, return value, warning message.
macro_rules! try_opt_warn {
    ($task:expr, $ret:expr, $fmt:expr) => {
        match $task {
            Some(v) => v,
            None => {
                warn!($fmt);
                return $ret;
            }
        }
    };
    ($task:expr, $ret:expr, $fmt:expr, $($arg:tt)*) => {
        match $task {
            Some(v) => v,
            None => {
                warn!($fmt, $($arg)*);
                return $ret;
            }
        }
    };
}


#[cfg(feature = "cairo-backend")] pub mod render_cairo;
#[cfg(feature = "qt-backend")] pub mod render_qt;

pub mod tree;
pub mod utils;
mod convert;
mod error;
mod layers;
mod math;
mod options;
mod preproc;
mod traits;


use std::path::{
    Path,
};

pub use error::{
    Error,
    ErrorKind,
    Result,
};
// reexport traits
pub use tree::{
    NodeExt,
};
pub use options::*;
pub use math::*;

/// Shorthand names for modules.
mod short {
    pub use svgdom::{
        LengthUnit as Unit,
        ElementId as EId,
        AttributeId as AId,
        AttributeValue as AValue,
    };
}

use preproc::{
    DEFAULT_FONT_FAMILY,
    DEFAULT_FONT_SIZE,
};


/// A generic interface for image rendering.
///
/// Instead of using backend implementation directly, you can
/// use this trait to write backend-independent code.
pub trait Render {
    /// Renders SVG to image.
    fn render_to_image(
        &self,
        rtree: &tree::RenderTree,
        opt: &Options,
    ) -> Result<Box<OutputImage>>;

    /// Renders SVG node to image.
    fn render_node_to_image(
        &self,
        node: tree::NodeRef,
        opt: &Options,
    ) -> Result<Box<OutputImage>>;

    /// Calculates node's absolute bounding box.
    ///
    /// Note: this method can be pretty expensive.
    fn calc_node_bbox(
        &self,
        node: tree::NodeRef,
        opt: &Options,
    ) -> Option<Rect>;
}

/// A generic interface for output image.
pub trait OutputImage {
    /// Saves rendered image to the selected path.
    fn save(&self, path: &Path) -> bool;
}


/// Global library handle.
pub struct InitObject {
    #[cfg(feature = "qt-backend")]
    #[allow(dead_code)]
    handle: qt::GuiApp,
}

/// Creates a global library handle.
///
/// Must be invoked before any other `resvg` code.
///
/// Currently, handles `QGuiApplication` object which must be created
/// in order to draw text. If you don't plan to draw text - it's better to skip
/// the initialization.
///
/// Does nothing when only `cairo` backend is enabled.
///
/// Note: `QGuiApplication` initialization is pretty slow (up to 100ms).
///
/// # Example
///
/// ```
/// let _resvg = resvg::init();
///
/// // other code
/// ```
///
/// Also, take a look at `examples/minimal.rs`.
pub fn init() -> InitObject {
    InitObject {
        #[cfg(feature = "qt-backend")]
        handle: qt::GuiApp::new("resvg"),
    }
}

/// Returns default backend.
///
/// - If both backends are enabled - cairo backend will be returned.
/// - If no backends are enabled - will panic.
/// - Otherwise will return a corresponding backend.
#[allow(unreachable_code)]
pub fn default_backend() -> Box<Render> {
    #[cfg(feature = "cairo-backend")]
    {
        return Box::new(render_cairo::Backend);
    }

    #[cfg(feature = "qt-backend")]
    {
        return Box::new(render_qt::Backend);
    }

    unreachable!("at least one backend must be enabled")
}

/// Creates `RenderTree` from SVG data.
pub fn parse_rtree_from_data(
    text: &str,
    opt: &Options,
) -> Result<tree::RenderTree> {
    let doc = parse_dom(text)?;
    parse_rtree_from_dom(doc, opt)
}

/// Creates `RenderTree` from file.
///
/// `.svg` and `.svgz` files are supported.
pub fn parse_rtree_from_file<P: AsRef<Path>>(
    path: P,
    opt: &Options,
) -> Result<tree::RenderTree> {
    let text = load_file(path.as_ref())?;
    parse_rtree_from_data(&text, opt)
}

/// Creates `RenderTree` from `svgdom::Document`.
pub fn parse_rtree_from_dom(
    mut doc: svgdom::Document,
    opt: &Options,
) -> Result<tree::RenderTree> {
    preproc::prepare_doc(&mut doc, opt)?;
    let rtree = convert::convert_doc(&doc, opt)?;

    Ok(rtree)
}

/// Load an SVG file.
///
/// - `svg` files will be loaded as is.
/// - `svgz` files will be decompressed.
///
/// **Note**: this is a low-level API. Use `parse_rtree_from_*` instead.
pub fn load_file(path: &Path) -> Result<String> {
    use std::fs;
    use std::io::Read;

    let mut file = fs::File::open(path)?;
    let length = file.metadata()?.len() as usize;

    let ext = if let Some(ext) = Path::new(path).extension() {
        ext.to_str().map(|s| s.to_lowercase()).unwrap_or(String::new())
    } else {
        String::new()
    };

    match ext.as_str() {
        "svgz" => {
            let mut decoder = libflate::gzip::Decoder::new(&file)?;
            let mut decoded = Vec::new();
            decoder.read_to_end(&mut decoded)?;

            Ok(String::from_utf8(decoded)?)
        }
        "svg" => {
            let mut s = String::with_capacity(length + 1);
            file.read_to_string(&mut s)?;
            Ok(s)
        }
        _ => {
            Err(ErrorKind::InvalidFileExtension.into())
        }
    }
}

/// Parses `svgdom::Document` object from the string data.
///
/// **Note**: this is a low-level API. Use `parse_rtree_from_*` instead.
pub fn parse_dom(text: &str) -> Result<svgdom::Document> {
    let opt = svgdom::ParseOptions {
        parse_comments: false,
        parse_declarations: false,
        parse_unknown_elements: false,
        parse_unknown_attributes: false,
        parse_px_unit: false,
        skip_invalid_attributes: true,
        skip_invalid_css: true,
        skip_paint_fallback: true,
        .. svgdom::ParseOptions::default()
    };

    let doc = svgdom::Document::from_str_with_opt(&text, &opt)?;
    Ok(doc)
}

/// Preprocesses a provided `svgdom::Document`.
///
/// Prepares an input `svgdom::Document` for conversion via `convert_dom_to_rtree`.
///
/// **Note**: this is a low-level API. Use `parse_rtree_from_*` instead.
pub fn preprocess_dom(
    doc: &mut svgdom::Document,
    opt: &Options,
) -> Result<()> {
    preproc::prepare_doc(doc, opt)
}

/// Converts a provided `svgdom::Document` to `tree::RenderTree`.
///
/// **Note**: this is a low-level API. Use `parse_rtree_from_*` instead.
pub fn convert_dom_to_rtree(
    doc: &svgdom::Document,
    opt: &Options,
) -> Result<tree::RenderTree> {
    convert::convert_doc(doc, opt)
}
