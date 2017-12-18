// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/*!
*libresvg* is an [SVG](https://en.wikipedia.org/wiki/Scalable_Vector_Graphics) rendering library.

*libresvg* can be used to render SVG files based on a
[static](http://www.w3.org/TR/SVG11/feature#SVG-static)
[SVG Full 1.1](https://www.w3.org/TR/SVG/Overview.html) subset.
In simple terms: no animations and scripting.

It can be used as a simple SVG to PNG converted.
And as an embeddable library to paint SVG on an application native canvas.
*/

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub extern crate svgdom;
extern crate base64;
extern crate libflate;
#[macro_use] pub extern crate log;
#[macro_use] extern crate error_chain;

#[cfg(feature = "cairo-backend")] pub extern crate cairo;
#[cfg(feature = "cairo-backend")] extern crate pango;
#[cfg(feature = "cairo-backend")] extern crate pangocairo;
#[cfg(feature = "cairo-backend")] extern crate image;

#[cfg(feature = "qt-backend")] pub extern crate resvg_qt as qt;


#[cfg(feature = "cairo-backend")] pub mod render_cairo;
#[cfg(feature = "qt-backend")] pub mod render_qt;

mod math;
mod convert;
mod dom;
mod error;
mod options;
mod preproc;
mod render_utils;
mod traits;


use std::fs;
use std::path::{
    Path,
};
use std::io::{
    Read,
};

pub use svgdom::{
    ChainedErrorExt,
};

pub use error::{
    Error,
    ErrorKind,
    Result,
};
pub use options::{
    FitTo,
    Options,
};
pub use dom::{
    Document,
};
pub use math::{
    Rect,
};

/// Shorthand names for modules.
mod short {
    pub use svgdom::types::LengthUnit as Unit;
    pub use svgdom::ElementId as EId;
    pub use svgdom::AttributeId as AId;
    pub use svgdom::AttributeValue as AValue;
}

use preproc::{
    DEFAULT_FONT_FAMILY,
    DEFAULT_FONT_SIZE,
};


/// Creates `Document` from SVG data.
pub fn parse_doc_from_data(text: &str, opt: &Options) -> Result<dom::Document> {
    let mut doc = parse_svg(text)?;
    prepare_doc(&mut doc, opt)?;
    let re_doc = convert_doc(&doc, opt)?;

    Ok(re_doc)
}

/// Creates `Document` from file.
///
/// `.svg` and `.svgz` files are supported.
pub fn parse_doc_from_file<P: AsRef<Path>>(path: P, opt: &Options) -> Result<dom::Document> {
    let text = load_file(path.as_ref())?;
    let mut doc = parse_svg(&text)?;
    prepare_doc(&mut doc, opt)?;
    let re_doc = convert_doc(&doc, opt)?;

    Ok(re_doc)
}

fn load_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let length = file.metadata()?.len() as usize;

    // 'unwrap' is safe because we already checked the extension via 'clap'.
    let ext = Path::new(path).extension().unwrap().to_str().unwrap().to_lowercase();

    let s = if ext == "svgz" {
        let mut decoder = libflate::gzip::Decoder::new(&file)?;
        let mut decoded = Vec::new();
        decoder.read_to_end(&mut decoded)?;

        String::from_utf8(decoded)?
    } else {
        let mut s = String::with_capacity(length + 1);
        file.read_to_string(&mut s)?;
        s
    };

    Ok(s)
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

fn prepare_doc(doc: &mut svgdom::Document, opt: &Options) -> Result<()> {
    preproc::prepare_doc(doc, opt)
}

fn convert_doc(doc: &svgdom::Document, opt: &Options) -> Result<dom::Document> {
    convert::convert_doc(doc, opt)
}
