// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

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
    /// This is against the SVG spec, but an automatic size detection is not supported yet.
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
