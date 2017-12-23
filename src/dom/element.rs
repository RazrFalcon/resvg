// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt;
use std::path::PathBuf;

use svgdom::types::{
    Transform,
    Color,
};

use math::{
    Rect,
};

use super::attribute::*;


/// An element.
///
/// Represents an object, that should be rendered.
pub struct Element {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Isn't automatically generated.
    /// Can be empty.
    ///
    /// Currently, used only for SVG dump purposes.
    pub id: String,
    /// Element transform.
    pub transform: Transform,
    /// Element kind.
    pub kind: ElementKind,
}

/// An element kind.
#[allow(missing_docs)]
pub enum ElementKind {
    Path(Path),
    Text(Text),
    Image(Image),
    Group(Group),
}

/// An element that can be referenced.
pub struct RefElement {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Can't be empty.
    ///
    /// Currently, used only for SVG dump purposes.
    ///
    /// It isn't used for referencing itself, because we use indexes for that.
    pub id: String,
    /// Element kind.
    pub kind: RefElementKind,
}

/// A referenced element kind.
#[allow(missing_docs)]
pub enum RefElementKind {
    LinearGradient(LinearGradient),
    RadialGradient(RadialGradient),
    ClipPath(ClipPath),
}

/// A path element.
pub struct Path {
    /// Fill style.
    pub fill: Option<Fill>,
    /// Stroke style.
    pub stroke: Option<Stroke>,
    /// Segments list.
    ///
    /// All segments are in absolute coordinates.
    pub d: Vec<PathSegment>,
}

/// A text element.
///
/// `text` element in the SVG.
pub struct Text {
    /// List of [text chunks](https://www.w3.org/TR/SVG11/text.html#TextChunk).
    pub children: Vec<TextChunk>,
}

/// A text chunk.
///
/// Contains position and anchor of the next
/// [text chunk](https://www.w3.org/TR/SVG11/text.html#TextChunk).
///
/// Doesn't represented in the SVG directly. Usually, it's a first `tspan` or text node
/// and any `tspan` that defines either `x` or `y` coordinate and/or have `text-anchor`.
pub struct TextChunk {
    /// An absolute position on the X-axis.
    pub x: f64,
    /// An absolute position on the Y-axis.
    pub y: f64,
    /// A text anchor/align.
    pub anchor: TextAnchor,
    /// List of `Tspan`.
    pub children: Vec<TSpan>
}

// TODO: dx, dy
/// A text span.
///
/// `tspan` element in the SVG.
#[derive(Clone)]
pub struct TSpan {
    /// Fill style.
    pub fill: Option<Fill>,
    /// Stroke style.
    pub stroke: Option<Stroke>,
    /// Font description.
    pub font: Font,
    /// Text decoration.
    ///
    /// Unlike `text-decoration` attribute from the SVG, this one has all styles resolved.
    /// Basically, by the SVG `text-decoration` attribute can be defined on `tspan` element
    /// and on any parent element. And all definitions should be combined.
    /// The one that was defined by `tspan` uses the `tspan` style itself.
    /// The one that was defined by any parent node uses the `text` element style.
    /// So it's pretty hard to resolve.
    ///
    /// This property has all this stuff resolved.
    pub decoration: TextDecoration,
    /// An actual text line.
    ///
    /// SVG doesn't support multiline text, so this property doesn't have a new line inside of it.
    /// All the spaces are already trimmed or preserved, depending on the `xml:space` attribute.
    /// All characters references are already resolved, so there is no `&gt;` or `&#x50;`.
    /// So this text should be rendered as is, without any postprocessing.
    pub text: String,
}

/// A raster image element.
///
/// `image` element in the SVG.
pub struct Image {
    /// An image rectangle in which it should be fit.
    pub rect: Rect,
    /// Image data.
    pub data: ImageData,
}

/// A raster image container.
pub enum ImageData {
    /// Path to the image.
    ///
    /// Preprocessor checks that file exists, but because it can be removed later,
    /// there is no guarantee that this path is valid.
    Path(PathBuf),
    /// An image raw data.
    ///
    /// It's not a decoded image data, but the data that was decoded from base64.
    /// So you still need a PNG and a JPEG decoding library.
    Raw(Vec<u8>, ImageDataKind),
}

/// An image codec.
#[allow(missing_docs)]
#[derive(Copy,Clone,PartialEq)]
pub enum ImageDataKind {
    PNG,
    JPEG,
}

// TODO: no need for a separate vector
/// A group container.
///
/// The preprocessor will remove all groups that don't impact rendering.
/// Those that left is just an indicator that a new canvas should be created.
///
/// Currently, it's needed only for `opacity`.
///
/// `g` element in the SVG.
pub struct Group {
    /// Group opacity.
    ///
    /// After the group is rendered we should combine
    /// it with a parent group using the specified opacity.
    pub opacity: Option<f64>,
    /// Element clip path.
    ///
    /// The value is an index from the `Document::defs` list.
    /// Use it via `Document::get_defs()` method.
    pub clip_path: Option<usize>,
    /// List of children elements.
    pub children: Vec<Element>,
}

/// A generic gradient.
pub struct BaseGradient {
    /// Coordinate system units.
    ///
    /// `gradientUnits` in the SVG.
    pub units: Units,
    /// Gradient transform.
    ///
    /// `gradientTransform` in the SVG.
    pub transform: Transform,
    /// Gradient spreading method.
    ///
    /// `spreadMethod` in the SVG.
    pub spread_method: SpreadMethod,
    /// List of stop elements.
    ///
    /// Will always have more than two items.
    pub stops: Vec<Stop>,
}

/// A linear gradient.
///
/// `linearGradient` element in the SVG.
#[allow(missing_docs)]
pub struct LinearGradient {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    /// Base gradient data.
    pub d: BaseGradient,
}

impl fmt::Debug for LinearGradient {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
            "LinearGradient(\n  \
             x1: {} y1: {}\n  \
             x2: {} y2: {}\n  \
             units: {:?}\n  \
             transform: {}\n  \
             spread: {:?}\n",
            self.x1, self.y1, self.x2, self.y2,
            self.d.units, self.d.transform, self.d.spread_method
        )?;

        for stop in &self.d.stops {
            write!(f, "    {:?}\n", stop)?;
        }

        write!(f, ")")
    }
}

/// A radial gradient.
///
/// `radialGradient` element in the SVG.
#[allow(missing_docs)]
pub struct RadialGradient {
    pub cx: f64,
    pub cy: f64,
    pub r: f64,
    pub fx: f64,
    pub fy: f64,
    /// Base gradient data.
    pub d: BaseGradient,
}

impl fmt::Debug for RadialGradient {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
            "RadialGradient(\n  \
             cx: {} cy: {}\n  \
             fx: {} fy: {}\n  \
             r: {}\n  \
             units: {:?}\n  \
             transform: {}\n  \
             spread: {:?}\n",
            self.cx, self.cy, self.fx, self.fy, self.r,
            self.d.units, self.d.transform, self.d.spread_method
        )?;

        for stop in &self.d.stops {
            write!(f, "    {:?}\n", stop)?;
        }

        write!(f, ")")
    }
}

/// Gradient's stop element.
///
/// `stop` element in the SVG.
#[allow(missing_docs)]
pub struct Stop {
    pub offset: f64,
    pub color: Color,
    pub opacity: f64,
}

impl fmt::Debug for Stop {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Stop(offset: {:?}, color: {}, opacity: {:?})",
               self.offset, self.color, self.opacity)
    }
}

/// A clip-path element.
///
/// `clipPath` element in the SVG.
pub struct ClipPath {
    /// Coordinate system units.
    ///
    /// `clipPathUnits` in the SVG.
    pub units: Units,
    /// Clip path transform.
    ///
    /// `transform` in the SVG.
    pub transform: Transform,
    /// List of children elements.
    ///
    /// Contains only `Path` and `Text` elements.
    pub children: Vec<Element>,
}
