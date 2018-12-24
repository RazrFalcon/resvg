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


/// A fill rule.
///
/// `fill-rule` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}


/// An element units.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Units {
    UserSpaceOnUse,
    ObjectBoundingBox,
}


/// A marker units.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MarkerUnits {
    StrokeWidth,
    UserSpaceOnUse,
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


/// A text decoration style.
///
/// Defines the style of the line that should be rendered.
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct TextDecorationStyle {
    pub fill: Option<Fill>,
    pub stroke: Option<Stroke>,
}

impl Default for TextDecorationStyle {
    fn default() -> Self {
        TextDecorationStyle {
            fill: None,
            stroke: None,
        }
    }
}


/// A text decoration.
#[derive(Clone, Debug)]
pub struct TextDecoration {
    /// Draw underline using specified style.
    ///
    /// Should be drawn before/under text.
    pub underline: Option<TextDecorationStyle>,

    /// Draw overline using specified style.
    ///
    /// Should be drawn before/under text.
    pub overline: Option<TextDecorationStyle>,

    /// Draw line-through using specified style.
    ///
    /// Should be drawn after/over text.
    pub line_through: Option<TextDecorationStyle>,
}

impl Default for TextDecoration {
    fn default() -> Self {
        TextDecoration {
            underline: None,
            overline: None,
            line_through: None,
        }
    }
}


/// A text anchor.
///
/// `text-anchor` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TextAnchor {
    Start,
    Middle,
    End,
}


/// A font style.
///
/// `font-style` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}


/// A font variant.
///
/// `font-variant` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FontVariant {
    Normal,
    SmallCaps,
}


/// A font weight.
///
/// `font-weight` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FontWeight {
    W100,
    W200,
    W300,
    W400,
    W500,
    W600,
    W700,
    W800,
    W900,
}


/// A font stretch.
///
/// `font-stretch` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FontStretch {
    Normal,
    Wider,
    Narrower,
    UltraCondensed,
    ExtraCondensed,
    Condensed,
    SemiCondensed,
    SemiExpanded,
    Expanded,
    ExtraExpanded,
    UltraExpanded,
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
    pub dashoffset: f32,
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


/// A font description.
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct Font {
    /// Font family.
    ///
    /// Currently, is exactly the same as in the `font-family` attribute.
    /// So it can look like `Verdana, 'Times New Roman', sans-serif`.
    pub family: String,
    pub size: FontSize,
    pub style: FontStyle,
    pub variant: FontVariant,
    pub weight: FontWeight,
    pub stretch: FontStretch,

    /// Letter spacing.
    ///
    /// None == `normal`
    pub letter_spacing: Option<f64>,

    /// Word spacing.
    ///
    /// None == `normal`
    pub word_spacing: Option<f64>,
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


/// A baseline shift value.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug)]
pub enum BaselineShift {
    Baseline,
    Subscript,
    Superscript,
    Percent(f64),
    Number(f64),
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
