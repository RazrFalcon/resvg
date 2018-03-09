// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::path::PathBuf;

// self
use math::{
    Size,
    Rect,
};
use super::attribute::*;
use super::NodeId;


#[allow(missing_docs)]
pub enum NodeKind {
    Svg(Svg),
    Defs,
    LinearGradient(LinearGradient),
    RadialGradient(RadialGradient),
    Stop(Stop),
    ClipPath(ClipPath),
    Pattern(Pattern),
    Path(Path),
    Text(Text),
    TextChunk(TextChunk),
    TSpan(TSpan),
    Image(Image),
    Group(Group),
}

impl NodeKind {
    /// Returns node's ID.
    ///
    /// If a current node doesn't support ID - an empty string
    /// will be returned.
    pub fn id(&self) -> &str {
        match *self {
            NodeKind::Svg(_) => "",
            NodeKind::Defs => "",
            NodeKind::LinearGradient(ref e) => e.id.as_str(),
            NodeKind::RadialGradient(ref e) => e.id.as_str(),
            NodeKind::Stop(_) => "",
            NodeKind::ClipPath(ref e) => e.id.as_str(),
            NodeKind::Pattern(ref e) => e.id.as_str(),
            NodeKind::Path(ref e) => e.id.as_str(),
            NodeKind::Text(ref e) => e.id.as_str(),
            NodeKind::TextChunk(_) => "",
            NodeKind::TSpan(_) => "",
            NodeKind::Image(ref e) => e.id.as_str(),
            NodeKind::Group(ref e) => e.id.as_str(),
        }
    }

    /// Returns node's transform.
    ///
    /// If a current node doesn't support transformation - a default
    /// transform will be returned.
    pub fn transform(&self) -> Transform {
        match *self {
            NodeKind::Svg(_) => Transform::default(),
            NodeKind::Defs => Transform::default(),
            NodeKind::LinearGradient(ref e) => e.d.transform,
            NodeKind::RadialGradient(ref e) => e.d.transform,
            NodeKind::Stop(_) => Transform::default(),
            NodeKind::ClipPath(ref e) => e.transform,
            NodeKind::Pattern(ref e) => e.transform,
            NodeKind::Path(ref e) => e.transform,
            NodeKind::Text(ref e) => e.transform,
            NodeKind::TextChunk(_) => Transform::default(),
            NodeKind::TSpan(_) => Transform::default(),
            NodeKind::Image(ref e) => e.transform,
            NodeKind::Group(ref e) => e.transform,
        }
    }
}


/// An SVG root element.
#[derive(Clone, Copy, Debug)]
pub struct Svg {
    /// Image size.
    ///
    /// Size of an image that should be created to fit the SVG.
    pub size: Size,
    /// SVG viewbox.
    ///
    /// Specifies which part of the SVG image should be rendered.
    pub view_box: Rect,
}


/// A path element.
#[derive(Clone)]
pub struct Path {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Isn't automatically generated.
    /// Can be empty.
    pub id: String,
    /// Element transform.
    pub transform: Transform,
    /// Fill style.
    pub fill: Option<Fill>,
    /// Stroke style.
    pub stroke: Option<Stroke>,
    /// Segments list.
    ///
    /// All segments are in absolute coordinates.
    pub segments: Vec<PathSegment>,
}


/// A text element.
///
/// `text` element in the SVG.
pub struct Text {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Isn't automatically generated.
    /// Can be empty.
    pub id: String,
    /// Element transform.
    pub transform: Transform,
}


/// A text chunk.
///
/// Contains position and anchor of the next
/// [text chunk](https://www.w3.org/TR/SVG11/text.html#TextChunk).
///
/// Doesn't represented in the SVG directly. Usually, it's a first `tspan` or text node
/// and any `tspan` that defines either `x` or `y` coordinate and/or have `text-anchor`.
#[derive(Clone, Copy)]
pub struct TextChunk {
    /// An absolute position on the X-axis.
    pub x: f64,
    /// An absolute position on the Y-axis.
    pub y: f64,
    /// A text anchor/align.
    pub anchor: TextAnchor,
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
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Isn't automatically generated.
    /// Can be empty.
    pub id: String,
    /// Element transform.
    pub transform: Transform,
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
#[derive(Clone, Copy, PartialEq)]
pub enum ImageDataKind {
    PNG,
    JPEG,
}


/// A group container.
///
/// The preprocessor will remove all groups that don't impact rendering.
/// Those that left is just an indicator that a new canvas should be created.
///
/// Currently, it's needed only for `opacity`.
///
/// `g` element in the SVG.
pub struct Group {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Isn't automatically generated.
    /// Can be empty.
    pub id: String,
    /// Element transform.
    pub transform: Transform,
    /// Group opacity.
    ///
    /// After the group is rendered we should combine
    /// it with a parent group using the specified opacity.
    pub opacity: Option<f64>,
    /// Element clip path.
    pub clip_path: Option<NodeId>,
}


/// A generic gradient.
#[derive(Clone, Copy)]
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
}


/// A linear gradient.
///
/// `linearGradient` element in the SVG.
#[allow(missing_docs)]
pub struct LinearGradient {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Can't be empty.
    pub id: String,
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    /// Base gradient data.
    pub d: BaseGradient,
}


/// A radial gradient.
///
/// `radialGradient` element in the SVG.
#[allow(missing_docs)]
pub struct RadialGradient {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Can't be empty.
    pub id: String,
    pub cx: f64,
    pub cy: f64,
    pub r: f64,
    pub fx: f64,
    pub fy: f64,
    /// Base gradient data.
    pub d: BaseGradient,
}


/// Gradient's stop element.
///
/// `stop` element in the SVG.
#[derive(Clone, Copy)]
#[allow(missing_docs)]
pub struct Stop {
    pub offset: f64,
    pub color: Color,
    pub opacity: f64,
}


/// A clip-path element.
///
/// `clipPath` element in the SVG.
pub struct ClipPath {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Can't be empty.
    pub id: String,
    /// Coordinate system units.
    ///
    /// `clipPathUnits` in the SVG.
    pub units: Units,
    /// Clip path transform.
    ///
    /// `transform` in the SVG.
    pub transform: Transform,
}


/// A pattern element.
///
/// `pattern` element in the SVG.
pub struct Pattern {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Can't be empty.
    pub id: String,
    /// Coordinate system units.
    ///
    /// `patternUnits` in the SVG.
    pub units: Units,
    /// Content coordinate system units.
    ///
    /// `patternContentUnits` in the SVG.
    pub content_units: Units,
    /// Pattern transform.
    ///
    /// `patternTransform` in the SVG.
    pub transform: Transform,
    /// Pattern rectangle.
    ///
    /// `x`, `y`, `width` and `height` in the SVG.
    pub rect: Rect,
    /// Pattern viewbox.
    pub view_box: Option<Rect>,
}
