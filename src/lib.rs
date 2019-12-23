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

#![doc(html_root_url = "https://docs.rs/resvg/0.8.0")]

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Unwraps `Option` and invokes `return` on `None`.
macro_rules! try_opt {
    ($task:expr) => {
        match $task {
            Some(v) => v,
            None => return,
        }
    };
}

/// Unwraps `Option` and invokes `return $ret` on `None`.
macro_rules! try_opt_or {
    ($task:expr, $ret:expr) => {
        match $task {
            Some(v) => v,
            None => return $ret,
        }
    };
}

/// Unwraps `Option` and invokes `return` on `None` with a warning.
#[allow(unused_macros)]
macro_rules! try_opt_warn {
    ($task:expr, $msg:expr) => {
        match $task {
            Some(v) => v,
            None => {
                log::warn!($msg);
                return;
            }
        }
    };
    ($task:expr, $fmt:expr, $($arg:tt)*) => {
        match $task {
            Some(v) => v,
            None => {
                log::warn!($fmt, $($arg)*);
                return;
            }
        }
    };
}

/// Unwraps `Option` and invokes `return $ret` on `None` with a warning.
#[allow(unused_macros)]
macro_rules! try_opt_warn_or {
    ($task:expr, $ret:expr, $msg:expr) => {
        match $task {
            Some(v) => v,
            None => {
                log::warn!($msg);
                return $ret;
            }
        }
    };
    ($task:expr, $ret:expr, $fmt:expr, $($arg:tt)*) => {
        match $task {
            Some(v) => v,
            None => {
                log::warn!($fmt, $($arg)*);
                return $ret;
            }
        }
    };
}

#[cfg(feature = "cairo-backend")]
pub use cairo;

#[cfg(feature = "qt-backend")]
pub use resvg_qt as qt;

#[cfg(feature = "skia-backend")]
pub use resvg_skia as skia;

#[cfg(feature = "raqote-backend")]
pub use raqote;

pub use usvg::{self, Error};


#[cfg(feature = "cairo-backend")]
pub mod backend_cairo;

#[cfg(feature = "qt-backend")]
pub mod backend_qt;

#[cfg(feature = "skia-backend")]
pub mod backend_skia;

#[cfg(feature = "raqote-backend")]
pub mod backend_raqote;

pub mod utils;
mod filter;
mod geom;
mod image;
mod layers;
mod options;

/// Commonly used types and traits.
pub mod prelude {
    pub use usvg::{self, prelude::*};
    pub use crate::{geom::*, options::*, utils, OutputImage, Render};
}

pub use crate::geom::*;
pub use crate::options::*;


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
    ) -> Option<Box<dyn OutputImage>>;

    /// Renders SVG node to image.
    ///
    /// Returns `None` if an image allocation failed.
    fn render_node_to_image(
        &self,
        node: &usvg::Node,
        opt: &Options,
    ) -> Option<Box<dyn OutputImage>>;
}

/// A generic interface for output image.
pub trait OutputImage {
    /// Saves rendered image to the selected path.
    fn save_png(
        &mut self,
        path: &std::path::Path,
    ) -> bool;

    /// Converts an image's internal data into a `Vec<u8>`.
    fn make_vec(&mut self) -> Vec<u8>;
}


/// Returns a default backend.
///
/// - If both backends are enabled - cairo backend will be returned.
/// - If no backends are enabled - will panic.
/// - Otherwise will return a corresponding backend.
#[allow(unreachable_code)]
pub fn default_backend() -> Box<dyn Render> {
    #[cfg(feature = "cairo-backend")]
    {
        return Box::new(backend_cairo::Backend);
    }

    #[cfg(feature = "qt-backend")]
    {
        return Box::new(backend_qt::Backend);
    }

    #[cfg(feature = "skia-backend")]
    {
        return Box::new(backend_skia::Backend);
    }

    #[cfg(feature = "raqote-backend")]
    {
        return Box::new(backend_raqote::Backend);
    }

    unreachable!("at least one backend must be enabled")
}

pub(crate) fn use_shape_antialiasing(
    mode: usvg::ShapeRendering,
) -> bool {
    match mode {
        usvg::ShapeRendering::OptimizeSpeed         => false,
        usvg::ShapeRendering::CrispEdges            => false,
        usvg::ShapeRendering::GeometricPrecision    => true,
    }
}

/// Converts an image to an alpha mask.
pub(crate) fn image_to_mask(
    data: &mut [rgb::alt::BGRA8],
    img_size: ScreenSize,
) {
    let width = img_size.width();
    let height = img_size.height();

    let coeff_r = 0.2125 / 255.0;
    let coeff_g = 0.7154 / 255.0;
    let coeff_b = 0.0721 / 255.0;

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            let ref mut pixel = data[idx];

            let r = pixel.r as f64;
            let g = pixel.g as f64;
            let b = pixel.b as f64;

            let luma = r * coeff_r + g * coeff_g + b * coeff_b;

            pixel.r = 0;
            pixel.g = 0;
            pixel.b = 0;
            pixel.a = f64_bound(0.0, luma * 255.0, 255.0) as u8;
        }
    }
}

pub(crate) trait ConvTransform<T> {
    fn to_native(&self) -> T;
    fn from_native(_: &T) -> Self;
}


#[derive(PartialEq)]
pub(crate) enum RenderState {
    /// A default value. Doesn't indicate anything.
    Ok,
    /// Indicates that the current rendering task should stop after reaching the specified node.
    RenderUntil(usvg::Node),
    /// Indicates that `usvg::FilterInput::BackgroundImage` rendering task was finished.
    BackgroundFinished,
}


/// Returns the node starting from which the filter background should be rendered.
pub(crate) fn filter_background_start_node(
    parent: &usvg::Node,
    filter: &usvg::Filter,
) -> Option<usvg::Node> {
    fn has_enable_background(node: &usvg::Node) -> bool {
        if let usvg::NodeKind::Group(ref g) = *node.borrow() {
            g.enable_background.is_some()
        } else {
            false
        }
    }

    if !filter.children.iter().any(|c| c.kind.has_input(&usvg::FilterInput::BackgroundImage)) &&
       !filter.children.iter().any(|c| c.kind.has_input(&usvg::FilterInput::BackgroundAlpha)) {
        return None;
    }

    // We should have an ancestor with `enable-background=new`.
    parent.ancestors().find(|node| has_enable_background(node))
}
