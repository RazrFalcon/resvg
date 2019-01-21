// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt;
use std::path::PathBuf;

// external
pub use svgdom::{
    Align,
    AspectRatio,
    Color,
    FuzzyEq,
    FuzzyZero,
    NumberList,
    Transform,
};

// self
use geom::*;
pub use super::numbers::*;


/// A line cap.
///
/// `stroke-linecap` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LineCap {
    Butt,
    Round,
    Square,
}

impl ToString for LineCap {
    fn to_string(&self) -> String {
        match self {
            LineCap::Butt   => "butt",
            LineCap::Round  => "round",
            LineCap::Square => "square",
        }.to_string()
    }
}


/// A line join.
///
/// `stroke-linejoin` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}

impl ToString for LineJoin {
    fn to_string(&self) -> String {
        match self {
            LineJoin::Miter => "miter",
            LineJoin::Round => "round",
            LineJoin::Bevel => "bevel",
        }.to_string()
    }
}


/// A fill rule.
///
/// `fill-rule` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

impl ToString for FillRule {
    fn to_string(&self) -> String {
        match self {
            FillRule::NonZero => "nonzero",
            FillRule::EvenOdd => "evenodd",
        }.to_string()
    }
}


/// An element units.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Units {
    UserSpaceOnUse,
    ObjectBoundingBox,
}

impl ToString for Units {
    fn to_string(&self) -> String {
        match self {
            Units::UserSpaceOnUse       => "userSpaceOnUse",
            Units::ObjectBoundingBox    => "objectBoundingBox",
        }.to_string()
    }
}


/// A marker units.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MarkerUnits {
    StrokeWidth,
    UserSpaceOnUse,
}

impl ToString for MarkerUnits {
    fn to_string(&self) -> String {
        match self {
            MarkerUnits::UserSpaceOnUse => "userSpaceOnUse",
            MarkerUnits::StrokeWidth    => "strokeWidth",
        }.to_string()
    }
}


/// A marker orientation.
#[derive(Clone, Copy, Debug)]
pub enum MarkerOrientation {
    /// Requires an automatic rotation.
    Auto,

    /// A rotation angle in degrees.
    Angle(f64),
}


/// A spread method.
///
/// `spreadMethod` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SpreadMethod {
    Pad,
    Reflect,
    Repeat,
}

impl ToString for SpreadMethod {
    fn to_string(&self) -> String {
        match self {
            SpreadMethod::Pad       => "pad",
            SpreadMethod::Reflect   => "reflect",
            SpreadMethod::Repeat    => "repeat",
        }.to_string()
    }
}


/// A visibility property.
///
/// `visibility` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Visibility {
    Visible,
    Hidden,
    Collapse,
}

impl ToString for Visibility {
    fn to_string(&self) -> String {
        match self {
            Visibility::Visible     => "visible",
            Visibility::Hidden      => "hidden",
            Visibility::Collapse    => "collapse",
        }.to_string()
    }
}


/// An overflow property.
///
/// `overflow` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Overflow {
    Visible,
    Hidden,
    Scroll,
    Auto,
}

impl ToString for Overflow {
    fn to_string(&self) -> String {
        match self {
            Overflow::Visible   => "visible",
            Overflow::Hidden    => "hidden",
            Overflow::Scroll    => "scroll",
            Overflow::Auto      => "auto",
        }.to_string()
    }
}


/// A paint style.
///
/// `paint` value type in the SVG.
#[allow(missing_docs)]
#[derive(Clone)]
pub enum Paint {
    /// Paint with a color.
    Color(Color),

    /// Paint using a referenced element.
    Link(String),
}

impl fmt::Debug for Paint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Paint::Color(c) => write!(f, "Color({})", c),
            Paint::Link(_) => write!(f, "Link"),
        }
    }
}


/// A fill style.
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct Fill {
    pub paint: Paint,
    pub opacity: Opacity,
    pub rule: FillRule,
}

impl Default for Fill {
    fn default() -> Self {
        Fill {
            paint: Paint::Color(Color::black()),
            opacity: 1.0.into(),
            rule: FillRule::NonZero,
        }
    }
}


/// A stroke style.
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct Stroke {
    pub paint: Paint,
    pub dasharray: Option<NumberList>,
    pub dashoffset: f32, // f32 and not f64 to reduce the struct size.
    pub miterlimit: StrokeMiterlimit,
    pub opacity: Opacity,
    pub width: StrokeWidth,
    pub linecap: LineCap,
    pub linejoin: LineJoin,
}

impl Default for Stroke {
    fn default() -> Self {
        Stroke {
            paint: Paint::Color(Color::black()),
            dasharray: None,
            dashoffset: 0.0,
            miterlimit: StrokeMiterlimit::default(),
            opacity: 1.0.into(),
            width: StrokeWidth::default(),
            linecap: LineCap::Butt,
            linejoin: LineJoin::Miter,
        }
    }
}


/// View box.
#[derive(Clone, Copy, Debug)]
pub struct ViewBox {
    /// Value of the `viewBox` attribute.
    pub rect: Rect,

    /// Value of the `preserveAspectRatio` attribute.
    pub aspect: AspectRatio,
}


/// A path absolute segment.
///
/// Unlike the SVG spec, can contain only `M`, `L`, `C` and `Z` segments.
/// All other segments will be converted into this one.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug)]
pub enum PathSegment {
    MoveTo {
        x: f64,
        y: f64,
    },
    LineTo {
        x: f64,
        y: f64,
    },
    CurveTo {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x: f64,
        y: f64,
    },
    ClosePath,
}


/// Identifies input for a filter primitive.
#[allow(missing_docs)]
#[derive(Clone, PartialEq, Debug)]
pub enum FilterInput {
    SourceGraphic,
    SourceAlpha,
    BackgroundImage,
    BackgroundAlpha,
    FillPaint,
    StrokePaint,
    Reference(String),
}

impl ToString for FilterInput {
    fn to_string(&self) -> String {
        match self {
            FilterInput::SourceGraphic      => "SourceGraphic",
            FilterInput::SourceAlpha        => "SourceAlpha",
            FilterInput::BackgroundImage    => "BackgroundImage",
            FilterInput::BackgroundAlpha    => "BackgroundAlpha",
            FilterInput::FillPaint          => "FillPaint",
            FilterInput::StrokePaint        => "StrokePaint",
            FilterInput::Reference(ref s)   => s,
        }.to_string()
    }
}


/// A color interpolation mode.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ColorInterpolation {
    SRGB,
    LinearRGB,
}

impl ToString for ColorInterpolation {
    fn to_string(&self) -> String {
        match self {
            ColorInterpolation::SRGB        => "sRGB",
            ColorInterpolation::LinearRGB   => "linearRGB",
        }.to_string()
    }
}


/// A raster image container.
#[derive(Clone, Debug)]
pub enum ImageData {
    /// Path to a PNG, JPEG or SVG(Z) image.
    ///
    /// Preprocessor will check that the file exist, but because it can be removed later,
    /// so there is no guarantee that this path is valid.
    ///
    /// The path may be relative.
    Path(PathBuf),

    /// Image raw data.
    ///
    /// It's not a decoded image data, but the data that was decoded from base64.
    /// So you still need a PNG, JPEG and SVG(Z) decoding libraries.
    Raw(Vec<u8>),
}


/// An image codec.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ImageFormat {
    PNG,
    JPEG,
    SVG,
}


/// An images blending mode.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FeBlendMode {
    Normal,
    Multiply,
    Screen,
    Darken,
    Lighten,
}

impl ToString for FeBlendMode {
    fn to_string(&self) -> String {
        match self {
            FeBlendMode::Normal     => "normal",
            FeBlendMode::Multiply   => "multiply",
            FeBlendMode::Screen     => "screen",
            FeBlendMode::Darken     => "darken",
            FeBlendMode::Lighten    => "lighten",
        }.to_string()
    }
}


/// An images compositing operation.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FeCompositeOperator {
    Over,
    In,
    Out,
    Atop,
    Xor,
    Arithmetic,
}

impl ToString for FeCompositeOperator {
    fn to_string(&self) -> String {
        match self {
            FeCompositeOperator::Over       => "over",
            FeCompositeOperator::In         => "in",
            FeCompositeOperator::Out        => "out",
            FeCompositeOperator::Atop       => "atop",
            FeCompositeOperator::Xor        => "xor",
            FeCompositeOperator::Arithmetic => "arithmetic",
        }.to_string()
    }
}


/// Kind of the `feImage` data.
#[derive(Clone, Debug)]
pub enum FeImageKind {
    /// Empty image.
    ///
    /// Unlike the `image` element, `feImage` can be without the `href` attribute.
    /// In this case the filter primitive is an empty canvas.
    /// And we can't remove it, because its `result` can be used.
    None,

    /// An image data.
    Image(ImageData, ImageFormat),

    /// A reference to an SVG object.
    ///
    /// `feImage` can reference any SVG object, just like `use` element.
    /// But we can't resolve `use` in this case.
    ///
    /// Not supported yet.
    Use(String),
}


/// A path marker properties.
#[derive(Clone, Debug)]
pub struct PathMarker {
    /// Start marker.
    ///
    /// `marker-start` in SVG.
    pub start: Option<String>,

    /// Middle marker
    ///
    /// `marker-mid` in SVG.
    pub mid: Option<String>,

    /// End marker
    ///
    /// `marker-end` in SVG.
    pub end: Option<String>,

    /// Marker stroke.
    ///
    /// This value contains a copy of the `stroke-width` value.
    /// `usvg` will set `Path::stroke` to `None` if a path doesn't have a stroke,
    /// but marker rendering still relies on the `stroke-width` value, even when `stroke=none`.
    /// So we have to store it separately.
    pub stroke: Option<StrokeWidth>,
}

impl Default for PathMarker {
    fn default() -> Self {
        PathMarker {
            start: None,
            mid: None,
            end: None,
            stroke: None,
        }
    }
}
