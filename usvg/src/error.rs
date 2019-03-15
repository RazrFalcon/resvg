// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::error;
use std::fmt;

use svgdom;

/// List of all errors.
#[derive(Debug)]
pub enum Error {
    /// Only `svg` and `svgz` suffixes are supported.
    InvalidFileSuffix,

    /// Failed to open the provided file.
    FileOpenFailed,

    /// Only UTF-8 content are supported.
    NotAnUtf8Str,

    /// Compressed SVG must use the GZip algorithm.
    MalformedGZip,

    /// SVG doesn't have a valid size.
    ///
    /// Occurs when width and/or height are <= 0.
    ///
    /// Also occurs if width, height and viewBox are not set.
    /// This is against the SVG spec, but an automatic size detection is not supported yet.
    InvalidSize,

    /// Failed to parse an SVG data.
    ParsingFailed(svgdom::ParserError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::InvalidFileSuffix => {
                write!(f, "invalid file suffix")
            }
            Error::FileOpenFailed => {
                write!(f, "failed to open the provided file")
            }
            Error::NotAnUtf8Str => {
                write!(f, "provided data has not an UTF-8 encoding")
            }
            Error::MalformedGZip => {
                write!(f, "provided data has a malformed GZip content")
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

impl error::Error for Error {
    fn description(&self) -> &str {
        "an SVG simplification error"
    }
}
