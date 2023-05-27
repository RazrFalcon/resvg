// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/*!
`usvg-parser` is an [SVG] parser used by [usvg].

[SVG]: https://en.wikipedia.org/wiki/Scalable_Vector_Graphics
[usvg]: https://github.com/RazrFalcon/resvg/tree/master/crates/usvg
*/

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![warn(missing_copy_implementations)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::identity_op)]
#![allow(clippy::question_mark)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::upper_case_acronyms)]

mod clippath;
mod converter;
mod filter;
mod image;
mod marker;
mod mask;
mod options;
mod paint_server;
mod shapes;
mod style;
mod svgtree;
mod switch;
mod text;
mod units;
mod use_node;

pub use crate::options::*;
pub use image::ImageHrefResolver;
pub use roxmltree;
pub use svgtree::{AId, EId};

/// List of all errors.
#[derive(Debug)]
pub enum Error {
    /// Only UTF-8 content are supported.
    NotAnUtf8Str,

    /// Compressed SVG must use the GZip algorithm.
    MalformedGZip,

    /// We do not allow SVG with more than 1_000_000 elements for security reasons.
    ElementsLimitReached,

    /// SVG doesn't have a valid size.
    ///
    /// Occurs when width and/or height are <= 0.
    ///
    /// Also occurs if width, height and viewBox are not set.
    InvalidSize,

    /// Failed to parse an SVG data.
    ParsingFailed(roxmltree::Error),
}

impl From<roxmltree::Error> for Error {
    fn from(e: roxmltree::Error) -> Self {
        Error::ParsingFailed(e)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Error::NotAnUtf8Str => {
                write!(f, "provided data has not an UTF-8 encoding")
            }
            Error::MalformedGZip => {
                write!(f, "provided data has a malformed GZip content")
            }
            Error::ElementsLimitReached => {
                write!(f, "the maximum number of SVG elements has been reached")
            }
            Error::InvalidSize => {
                write!(f, "SVG has an invalid size")
            }
            Error::ParsingFailed(ref e) => {
                write!(f, "SVG data parsing failed cause {}", e)
            }
        }
    }
}

impl std::error::Error for Error {}

trait OptionLog {
    fn log_none<F: FnOnce()>(self, f: F) -> Self;
}

impl<T> OptionLog for Option<T> {
    #[inline]
    fn log_none<F: FnOnce()>(self, f: F) -> Self {
        self.or_else(|| {
            f();
            None
        })
    }
}

/// A trait to parse `usvg_tree::Tree` from various sources.
pub trait TreeParsing: Sized {
    /// Parses `Tree` from an SVG data.
    ///
    /// Can contain an SVG string or a gzip compressed data.
    fn from_data(data: &[u8], opt: &Options) -> Result<Self, Error>;

    /// Parses `Tree` from an SVG string.
    fn from_str(text: &str, opt: &Options) -> Result<Self, Error>;

    /// Parses `Tree` from `roxmltree::Document`.
    fn from_xmltree(doc: &roxmltree::Document, opt: &Options) -> Result<Self, Error>;
}

impl TreeParsing for usvg_tree::Tree {
    /// Parses `Tree` from an SVG data.
    ///
    /// Can contain an SVG string or a gzip compressed data.
    fn from_data(data: &[u8], opt: &Options) -> Result<Self, Error> {
        if data.starts_with(&[0x1f, 0x8b]) {
            let data = decompress_svgz(data)?;
            let text = std::str::from_utf8(&data).map_err(|_| Error::NotAnUtf8Str)?;
            Self::from_str(text, opt)
        } else {
            let text = std::str::from_utf8(data).map_err(|_| Error::NotAnUtf8Str)?;
            Self::from_str(text, opt)
        }
    }

    /// Parses `Tree` from an SVG string.
    fn from_str(text: &str, opt: &Options) -> Result<Self, Error> {
        let xml_opt = roxmltree::ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        };

        let doc =
            roxmltree::Document::parse_with_options(text, xml_opt).map_err(Error::ParsingFailed)?;

        Self::from_xmltree(&doc, opt)
    }

    /// Parses `Tree` from `roxmltree::Document`.
    fn from_xmltree(doc: &roxmltree::Document, opt: &Options) -> Result<Self, Error> {
        let doc = svgtree::Document::parse_tree(doc)?;
        crate::converter::convert_doc(&doc, opt)
    }
}

/// Decompresses an SVGZ file.
pub fn decompress_svgz(data: &[u8]) -> Result<Vec<u8>, Error> {
    use std::io::Read;

    let mut decoder = flate2::read::GzDecoder::new(data);
    let mut decoded = Vec::with_capacity(data.len() * 2);
    decoder
        .read_to_end(&mut decoded)
        .map_err(|_| Error::MalformedGZip)?;
    Ok(decoded)
}

#[inline]
pub(crate) fn f32_bound(min: f32, val: f32, max: f32) -> f32 {
    debug_assert!(min.is_finite());
    debug_assert!(val.is_finite());
    debug_assert!(max.is_finite());

    if val > max {
        max
    } else if val < min {
        min
    } else {
        val
    }
}
