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

// For error-chain.
#![recursion_limit="128"]

extern crate svgdom;
extern crate base64;
extern crate libflate;
extern crate euclid;
#[macro_use] extern crate log;
#[macro_use] extern crate error_chain;

#[cfg(feature = "cairo-backend")] pub extern crate cairo;
#[cfg(feature = "cairo-backend")] extern crate pango;
#[cfg(feature = "cairo-backend")] extern crate pangocairo;
#[cfg(feature = "cairo-backend")] extern crate image;

#[cfg(feature = "qt-backend")] pub extern crate resvg_qt as qt;


#[cfg(feature = "cairo-backend")] pub mod render_cairo;
#[cfg(feature = "qt-backend")] pub mod render_qt;

pub mod tree;
mod math;
mod convert;
mod error;
mod options;
mod preproc;
mod render_utils;
mod traits;


use std::path::{
    Path,
};

pub use svgdom::{
    ChainedErrorExt,
};

pub use error::{
    Error,
    ErrorKind,
    Result,
};
pub use options::*;
pub use math::*;
use preproc::{
    DEFAULT_FONT_FAMILY,
    DEFAULT_FONT_SIZE,
};
/// Shorthand names for modules.
mod short {
    pub use svgdom::{
        LengthUnit as Unit,
        ElementId as EId,
        AttributeId as AId,
        AttributeValue as AValue,
    };
}


/// A list of supported backends.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Backend {
    /// [Cairo](https://cairographics.org/) backend.
    #[cfg(feature = "cairo-backend")]
    Cairo,
    /// [Qt](https://www.qt.io/) backend.
    #[cfg(feature = "qt-backend")]
    Qt,
}


/// Creates `RenderTree` from SVG data.
pub fn parse_doc_from_data(
    text: &str,
    opt: &Options,
) -> Result<tree::RenderTree> {
    let mut doc = parse_svg(text)?;
    preproc::prepare_doc(&mut doc, opt)?;
    let rtree = convert::convert_doc(&doc, opt)?;

    Ok(rtree)
}

/// Creates `RenderTree` from file.
///
/// `.svg` and `.svgz` files are supported.
pub fn parse_doc_from_file<P: AsRef<Path>>(
    path: P,
    opt: &Options,
) -> Result<tree::RenderTree> {
    let text = load_file(path.as_ref())?;
    let mut doc = parse_svg(&text)?;
    preproc::prepare_doc(&mut doc, opt)?;
    let rtree = convert::convert_doc(&doc, opt)?;

    Ok(rtree)
}

fn load_file(path: &Path) -> Result<String> {
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

fn parse_svg(text: &str) -> Result<svgdom::Document> {
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
