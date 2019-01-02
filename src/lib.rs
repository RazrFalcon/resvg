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

#![doc(html_root_url = "https://docs.rs/resvg/0.4.0")]

//#![forbid(unsafe_code)]
#![warn(missing_docs)]

#[macro_use] pub extern crate usvg;
#[macro_use] extern crate log;
extern crate unicode_segmentation;
extern crate rgb;

#[cfg(feature = "cairo-backend")] pub extern crate cairo;
#[cfg(feature = "cairo-backend")] extern crate pango;
#[cfg(feature = "cairo-backend")] extern crate pangocairo;
#[cfg(feature = "cairo-backend")] extern crate gdk_pixbuf;

#[cfg(feature = "qt-backend")] pub extern crate resvg_qt as qt;


pub use usvg::{
    svgdom,
    Error,
};

use usvg::lyon_geom;


#[cfg(feature = "cairo-backend")] pub mod backend_cairo;
#[cfg(feature = "qt-backend")] pub mod backend_qt;

pub mod utils;
mod backend_utils;
mod geom;
mod layers;
mod options;
mod traits;

/// Commonly used types and traits.
pub mod prelude {
    pub use usvg;
    pub use usvg::prelude::*;
    pub use geom::*;
    pub(crate) use traits::*;
    pub use utils;
    pub use Options;
    pub use Render;
    pub use OutputImage;
}


use std::path;

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


/// A generic interface for image rendering.
///
/// Instead of using backend implementation directly, you can
/// use this trait to write backend-independent code.
pub trait Render {
    /// Renders SVG to image.
    ///
    /// Returns `None` if an image allocation failed.
    fn render_to_image(
        &self,
        tree: &usvg::Tree,
        opt: &Options,
    ) -> Option<Box<OutputImage>>;

    /// Renders SVG node to image.
    ///
    /// Returns `None` if an image allocation failed.
    fn render_node_to_image(
        &self,
        node: &usvg::Node,
        opt: &Options,
    ) -> Option<Box<OutputImage>>;

    /// Calculates node's absolute bounding box.
    ///
    /// Note: this method can be pretty expensive.
    fn calc_node_bbox(
        &self,
        node: &usvg::Node,
        opt: &Options,
    ) -> Option<Rect>;
}

/// A generic interface for output image.
pub trait OutputImage {
    /// Saves rendered image to the selected path.
    fn save(&self, path: &path::Path) -> bool;
}


/// A global library handle.
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
/// Does nothing when only the `cairo` backend is enabled.
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
///
/// **Warning**: this method is not thread-safe.
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
        return Box::new(backend_cairo::Backend);
    }

    #[cfg(feature = "qt-backend")]
    {
        return Box::new(backend_qt::Backend);
    }

    unreachable!("at least one backend must be enabled")
}
