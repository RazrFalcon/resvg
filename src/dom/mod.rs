// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom;

use math::{
    Size,
    Rect,
};


mod attribute;
mod dump;
mod element;

pub use self::element::*;
pub use self::attribute::*;


/// Container for a preprocessed SVG.
///
/// Unlike svgdom's `Document` this one is immutable for a backend code
/// and contains only supported, resolved elements and attributes.
pub struct Document {
    /// Image size.
    ///
    /// Size of an image that should be created to fit the SVG.
    pub size: Size,
    /// SVG viewbox.
    ///
    /// Specifies which part of the SVG image should be rendered.
    pub view_box: Rect,
    /// Image DPI.
    ///
    /// Has the same value as `Options::dpi`. Used for text rendering.
    pub dpi: f64,
    /// List of all referenced elements.
    pub defs: Vec<RefElement>,
    /// List of all elements.
    ///
    /// Contains list of all elements that should be rendered.
    pub elements: Vec<Element>,
}

impl Document {
    /// Returns referenced element by id.
    pub fn get_defs<'a>(&'a self, id: usize) -> &'a RefElement {
        &self.defs[id]
    }

    /// Converts the document to `svgdom::Document`.
    ///
    /// Used to save document to file for debug purposes.
    pub fn to_svgdom(&self) -> svgdom::Document {
        dump::conv_doc(self)
    }
}
