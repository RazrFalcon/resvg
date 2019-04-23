// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

// external
pub use svgdom::{
    Align,
    AspectRatio,
    Color,
    FuzzyEq,
    FuzzyZero,
    Transform,
};

// self
use geom::*;
pub use super::numbers::*;


macro_rules! enum_default {
    ($name:ident, $def_value:ident) => {
        impl Default for $name {
            fn default() -> Self {
                $name::$def_value
            }
        }
    };
}

macro_rules! enum_from_str {
    ($name:ident, $($string:pat => $result:expr),+) => {
        impl FromStr for $name {
            type Err = &'static str;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($string => Ok($result)),+,
                    _ => Err("invalid value"),
                }
            }
        }
    };
}

macro_rules! enum_to_string {
    ($name:ident, $($value:pat => $string:expr),+) => {
        impl ToString for $name {
            fn to_string(&self) -> String {
                match self {
                    $($value => $string),+,
                }.to_string()
            }
        }
    };
}


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

enum_default!(LineCap, Butt);

enum_from_str!(LineCap,
    "butt"      => LineCap::Butt,
    "round"     => LineCap::Round,
    "square"    => LineCap::Square
);

enum_to_string!(LineCap,
    LineCap::Butt   => "butt",
    LineCap::Round  => "round",
    LineCap::Square => "square"
);


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

enum_default!(LineJoin, Miter);

enum_from_str!(LineJoin,
    "miter" => LineJoin::Miter,
    "round" => LineJoin::Round,
    "bevel" => LineJoin::Bevel
);

enum_to_string!(LineJoin,
    LineJoin::Miter => "miter",
    LineJoin::Round => "round",
    LineJoin::Bevel => "bevel"
);


/// A fill rule.
///
/// `fill-rule` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

enum_default!(FillRule, NonZero);

enum_from_str!(FillRule,
    "nonzero" => FillRule::NonZero,
    "evenodd" => FillRule::EvenOdd
);

enum_to_string!(FillRule,
    FillRule::NonZero => "nonzero",
    FillRule::EvenOdd => "evenodd"
);


/// An element units.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Units {
    UserSpaceOnUse,
    ObjectBoundingBox,
}

enum_to_string!(Units,
    Units::UserSpaceOnUse       => "userSpaceOnUse",
    Units::ObjectBoundingBox    => "objectBoundingBox"
);


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

enum_default!(SpreadMethod, Pad);

enum_to_string!(SpreadMethod,
    SpreadMethod::Pad       => "pad",
    SpreadMethod::Reflect   => "reflect",
    SpreadMethod::Repeat    => "repeat"
);


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

enum_default!(Visibility, Visible);

enum_from_str!(Visibility,
    "visible"   => Visibility::Visible,
    "hidden"    => Visibility::Hidden,
    "collapse"  => Visibility::Collapse
);

enum_to_string!(Visibility,
    Visibility::Visible     => "visible",
    Visibility::Hidden      => "hidden",
    Visibility::Collapse    => "collapse"
);


/// A text decoration style.
///
/// Defines the style of the line that should be rendered.
#[allow(missing_docs)]
#[derive(Clone, Default, Debug)]
pub struct TextDecorationStyle {
    pub fill: Option<Fill>,
    pub stroke: Option<Stroke>,
}


/// A text decoration.
#[derive(Clone, Default, Debug)]
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

enum_default!(TextAnchor, Start);

enum_from_str!(TextAnchor,
    "start"     => TextAnchor::Start,
    "middle"    => TextAnchor::Middle,
    "end"       => TextAnchor::End
);

enum_to_string!(TextAnchor,
    TextAnchor::Start   => "start",
    TextAnchor::Middle  => "middle",
    TextAnchor::End     => "end"
);


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

enum_default!(FontStyle, Normal);

enum_from_str!(FontStyle,
    "normal"    => FontStyle::Normal,
    "italic"    => FontStyle::Italic,
    "oblique"   => FontStyle::Oblique
);

enum_to_string!(FontStyle,
    FontStyle::Normal   => "normal",
    FontStyle::Italic   => "italic",
    FontStyle::Oblique  => "oblique"
);


/// A font variant.
///
/// `font-variant` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FontVariant {
    Normal,
    SmallCaps,
}

enum_default!(FontVariant, Normal);

enum_from_str!(FontVariant,
    "normal"        => FontVariant::Normal,
    "small-caps"    => FontVariant::SmallCaps
);

enum_to_string!(FontVariant,
    FontVariant::Normal     => "normal",
    FontVariant::SmallCaps  => "small-caps"
);


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

enum_default!(FontWeight, W400);

enum_to_string!(FontWeight,
    FontWeight::W100 => "100",
    FontWeight::W200 => "200",
    FontWeight::W300 => "300",
    FontWeight::W400 => "400",
    FontWeight::W500 => "500",
    FontWeight::W600 => "600",
    FontWeight::W700 => "700",
    FontWeight::W800 => "800",
    FontWeight::W900 => "900"
);


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

enum_default!(FontStretch, Normal);

enum_from_str!(FontStretch,
    "normal"            => FontStretch::Normal,
    "wider"             => FontStretch::Wider,
    "narrower"          => FontStretch::Narrower,
    "ultra-condensed"   => FontStretch::UltraCondensed,
    "extra-condensed"   => FontStretch::ExtraCondensed,
    "condensed"         => FontStretch::Condensed,
    "semi-condensed"    => FontStretch::SemiCondensed,
    "semi-expanded"     => FontStretch::SemiExpanded,
    "expanded"          => FontStretch::Expanded,
    "extra-expanded"    => FontStretch::ExtraExpanded,
    "ultra-expanded"    => FontStretch::UltraExpanded
);

enum_to_string!(FontStretch,
    FontStretch::Normal         => "normal",
    FontStretch::Wider          => "wider",
    FontStretch::Narrower       => "narrower",
    FontStretch::UltraCondensed => "ultra-condensed",
    FontStretch::ExtraCondensed => "extra-condensed",
    FontStretch::Condensed      => "condensed",
    FontStretch::SemiCondensed  => "semi-condensed",
    FontStretch::SemiExpanded   => "semi-expanded",
    FontStretch::Expanded       => "expanded",
    FontStretch::ExtraExpanded  => "extra-expanded",
    FontStretch::UltraExpanded  => "ultra-expanded"
);


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
            Paint::Link(_)  => write!(f, "Link"),
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
            opacity: Opacity::default(),
            rule: FillRule::default(),
        }
    }
}


/// A stroke style.
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct Stroke {
    pub paint: Paint,
    pub dasharray: Option<Vec<f64>>,
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
            // The actual default color is `none`,
            // but to simplify the `Stroke` object creation we use `black`.
            paint: Paint::Color(Color::black()),
            dasharray: None,
            dashoffset: 0.0,
            miterlimit: StrokeMiterlimit::default(),
            opacity: Opacity::default(),
            width: StrokeWidth::default(),
            linecap: LineCap::default(),
            linejoin: LineJoin::default(),
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


/// A path's absolute segment.
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

enum_to_string!(FilterInput,
    FilterInput::SourceGraphic      => "SourceGraphic",
    FilterInput::SourceAlpha        => "SourceAlpha",
    FilterInput::BackgroundImage    => "BackgroundImage",
    FilterInput::BackgroundAlpha    => "BackgroundAlpha",
    FilterInput::FillPaint          => "FillPaint",
    FilterInput::StrokePaint        => "StrokePaint",
    FilterInput::Reference(ref s)   => s
);


/// A color interpolation mode.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ColorInterpolation {
    SRGB,
    LinearRGB,
}

enum_default!(ColorInterpolation, LinearRGB);

enum_from_str!(ColorInterpolation,
    "sRGB"      => ColorInterpolation::SRGB,
    "linearRGB" => ColorInterpolation::LinearRGB
);

enum_to_string!(ColorInterpolation,
    ColorInterpolation::SRGB        => "sRGB",
    ColorInterpolation::LinearRGB   => "linearRGB"
);


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

enum_to_string!(FeBlendMode,
    FeBlendMode::Normal     => "normal",
    FeBlendMode::Multiply   => "multiply",
    FeBlendMode::Screen     => "screen",
    FeBlendMode::Darken     => "darken",
    FeBlendMode::Lighten    => "lighten"
);


/// An images compositing operation.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FeCompositeOperator {
    Over,
    In,
    Out,
    Atop,
    Xor,
    Arithmetic {
        k1: CompositingCoefficient,
        k2: CompositingCoefficient,
        k3: CompositingCoefficient,
        k4: CompositingCoefficient,
    },
}

enum_to_string!(FeCompositeOperator,
    FeCompositeOperator::Over               => "over",
    FeCompositeOperator::In                 => "in",
    FeCompositeOperator::Out                => "out",
    FeCompositeOperator::Atop               => "atop",
    FeCompositeOperator::Xor                => "xor",
    FeCompositeOperator::Arithmetic { .. }  => "arithmetic"
);


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


/// A shape rendering method.
///
/// `shape-rendering` attribute in the SVG.
#[derive(Clone, Copy, PartialEq, Debug)]
#[allow(missing_docs)]
pub enum ShapeRendering {
    OptimizeSpeed,
    CrispEdges,
    GeometricPrecision,
}

enum_default!(ShapeRendering, GeometricPrecision);

enum_from_str!(ShapeRendering,
    "optimizeSpeed"         => ShapeRendering::OptimizeSpeed,
    "crispEdges"            => ShapeRendering::CrispEdges,
    "geometricPrecision"    => ShapeRendering::GeometricPrecision
);

enum_to_string!(ShapeRendering,
    ShapeRendering::OptimizeSpeed       => "optimizeSpeed",
    ShapeRendering::CrispEdges          => "crispEdges",
    ShapeRendering::GeometricPrecision  => "geometricPrecision"
);


/// A text rendering method.
///
/// `text-rendering` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TextRendering {
    OptimizeSpeed,
    OptimizeLegibility,
    GeometricPrecision,
}

enum_default!(TextRendering, OptimizeLegibility);

enum_from_str!(TextRendering,
    "optimizeSpeed"         => TextRendering::OptimizeSpeed,
    "optimizeLegibility"    => TextRendering::OptimizeLegibility,
    "geometricPrecision"    => TextRendering::GeometricPrecision
);

enum_to_string!(TextRendering,
    TextRendering::OptimizeSpeed       => "optimizeSpeed",
    TextRendering::OptimizeLegibility  => "optimizeLegibility",
    TextRendering::GeometricPrecision  => "geometricPrecision"
);


/// An image rendering method.
///
/// `image-rendering` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ImageRendering {
    OptimizeQuality,
    OptimizeSpeed,
}

enum_default!(ImageRendering, OptimizeQuality);

enum_from_str!(ImageRendering,
    "optimizeQuality"   => ImageRendering::OptimizeQuality,
    "optimizeSpeed"     => ImageRendering::OptimizeSpeed
);

enum_to_string!(ImageRendering,
    ImageRendering::OptimizeQuality => "optimizeQuality",
    ImageRendering::OptimizeSpeed   => "optimizeSpeed"
);
