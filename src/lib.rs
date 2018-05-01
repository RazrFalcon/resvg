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

#![doc(html_root_url = "https://docs.rs/resvg/0.2.0")]

#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate lyon_geom;
#[macro_use] pub extern crate usvg;
#[macro_use] extern crate failure;
#[macro_use] extern crate log;

#[cfg(feature = "cairo-backend")] pub extern crate cairo;
#[cfg(feature = "cairo-backend")] extern crate pango;
#[cfg(feature = "cairo-backend")] extern crate pangocairo;
#[cfg(feature = "cairo-backend")] extern crate image as piston_image;

#[cfg(feature = "qt-backend")] pub extern crate resvg_qt as qt;


pub use usvg::svgdom;
use lyon_geom::euclid;


#[cfg(feature = "cairo-backend")] pub mod render_cairo;
#[cfg(feature = "qt-backend")] pub mod render_qt;

pub mod utils;
pub mod geom;
mod error;
mod layers;
mod options;
mod traits;


use std::path::{
    Path,
};

pub use error::*;
pub use options::*;
pub use geom::*;

/// Shorthand names for modules.
mod short {
    pub use svgdom::{
        LengthUnit as Unit,
        ElementId as EId,
        AttributeId as AId,
        AttributeValue as AValue,
    };
}

pub use usvg::tree;


/// A generic interface for image rendering.
///
/// Instead of using backend implementation directly, you can
/// use this trait to write backend-independent code.
pub trait Render {
    /// Renders SVG to image.
    fn render_to_image(
        &self,
        tree: &tree::Tree,
        opt: &Options,
    ) -> Result<Box<OutputImage>>;

    /// Renders SVG node to image.
    fn render_node_to_image(
        &self,
        node: &tree::Node,
        opt: &Options,
    ) -> Result<Box<OutputImage>>;

    /// Calculates node's absolute bounding box.
    ///
    /// Note: this method can be pretty expensive.
    fn calc_node_bbox(
        &self,
        node: &tree::Node,
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
) -> tree::Tree {
    usvg::parse_tree_from_data(text, &opt.usvg)
}

/// Creates `RenderTree` from `svgdom::Document`.
pub fn parse_rtree_from_dom(
    doc: svgdom::Document,
    opt: &Options,
) -> tree::Tree {
    usvg::parse_tree_from_dom(doc, &opt.usvg)
}

/// Creates `RenderTree` from `svgdom::Document`.
pub fn parse_rtree_from_file<P: AsRef<Path>>(
    path: P,
    opt: &Options,
) -> Result<tree::Tree> {
    Ok(usvg::parse_tree_from_file(path, &opt.usvg)?)
}
