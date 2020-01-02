// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt;
use std::path::PathBuf;

pub use svgtypes::{
    Align,
    AspectRatio,
    Color,
    FuzzyEq,
    FuzzyZero,
    Transform,
};

use crate::geom::*;
pub use super::numbers::*;


macro_rules! impl_from_str {
    ($name:ident) => {
        impl std::str::FromStr for $name {
            type Err = &'static str;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                crate::svgtree::EnumFromStr::enum_from_str(s).ok_or("invalid value")
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

impl_enum_default!(LineCap, Butt);

impl_enum_from_str!(LineCap,
    "butt"      => LineCap::Butt,
    "round"     => LineCap::Round,
    "square"    => LineCap::Square
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

impl_enum_default!(LineJoin, Miter);

impl_enum_from_str!(LineJoin,
    "miter" => LineJoin::Miter,
    "round" => LineJoin::Round,
    "bevel" => LineJoin::Bevel
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

impl_enum_default!(FillRule, NonZero);

impl_enum_from_str!(FillRule,
    "nonzero" => FillRule::NonZero,
    "evenodd" => FillRule::EvenOdd
);


/// An element units.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Units {
    UserSpaceOnUse,
    ObjectBoundingBox,
}

// `Units` cannot have a default value, because it changes depending on an element.

impl_enum_from_str!(Units,
    "userSpaceOnUse"    => Units::UserSpaceOnUse,
    "objectBoundingBox" => Units::ObjectBoundingBox
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

impl_enum_default!(SpreadMethod, Pad);

impl_enum_from_str!(SpreadMethod,
    "pad"       => SpreadMethod::Pad,
    "reflect"   => SpreadMethod::Reflect,
    "repeat"    => SpreadMethod::Repeat
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

impl_enum_default!(Visibility, Visible);

impl_enum_from_str!(Visibility,
    "visible"   => Visibility::Visible,
    "hidden"    => Visibility::Hidden,
    "collapse"  => Visibility::Collapse
);


/// A paint style.
///
/// `paint` value type in the SVG.
#[allow(missing_docs)]
#[derive(Clone)]
pub enum Paint {
    /// Paint with a color.
    Color(Color),

    /// Paint using a paint server.
    Link(String),
}

impl fmt::Debug for Paint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Paint::Color(c) => write!(f, "Color({})", c),
            Paint::Link(ref id)  => write!(f, "Link({})", id),
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

impl Fill {
    /// Creates a `Fill` from `Paint`.
    ///
    /// `opacity` and `rule` will be set to default values.
    pub fn from_paint(paint: Paint) -> Self {
        Fill {
            paint,
            ..Fill::default()
        }
    }
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


/// View box.
#[derive(Clone, Copy, Debug)]
pub struct ViewBox {
    /// Value of the `viewBox` attribute.
    pub rect: Rect,

    /// Value of the `preserveAspectRatio` attribute.
    pub aspect: AspectRatio,
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


/// A color interpolation mode.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ColorInterpolation {
    SRGB,
    LinearRGB,
}

impl_enum_default!(ColorInterpolation, LinearRGB);

impl_enum_from_str!(ColorInterpolation,
    "sRGB"      => ColorInterpolation::SRGB,
    "linearRGB" => ColorInterpolation::LinearRGB
);


/// A color channel.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ColorChannel {
    R,
    G,
    B,
    A,
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
        k1: f64,
        k2: f64,
        k3: f64,
        k4: f64,
    },
}


/// A morphology operation.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FeMorphologyOperator {
    Erode,
    Dilate,
}


/// Kind of the `feImage` data.
#[derive(Clone, Debug)]
pub enum FeImageKind {
    /// An image data.
    Image(ImageData, ImageFormat),

    /// A reference to an SVG object.
    ///
    /// `feImage` can reference any SVG object, just like `use` element.
    Use(String),
}


/// An edges processing mode.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FeEdgeMode {
    None,
    Duplicate,
    Wrap,
}


/// A turbulence kind for the `feTurbulence` filter.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FeTurbulenceKind {
    FractalNoise,
    Turbulence,
}


/// A convolve matrix representation.
///
/// Used primarily by `FeConvolveMatrix`.
#[derive(Clone, Debug)]
pub struct ConvolveMatrix {
    x: u32,
    y: u32,
    columns: u32,
    rows: u32,
    data: Vec<f64>,
}

impl ConvolveMatrix {
    /// Creates a new `ConvolveMatrix`.
    ///
    /// Returns `None` when:
    ///
    /// - `columns` * `rows` != `data.len()`
    /// - `target_x` >= `columns`
    /// - `target_y` >= `rows`
    pub fn new(target_x: u32, target_y: u32, columns: u32, rows: u32, data: Vec<f64>) -> Option<Self> {
        if (columns * rows) as usize != data.len()
           || target_x >= columns
           || target_y >= rows
        {
            return None;
        }

        Some(ConvolveMatrix {
            x: target_x,
            y: target_y,
            columns,
            rows,
            data,
        })
    }

    /// Returns a matrix's X target.
    ///
    /// `targetX` in the SVG.
    #[inline]
    pub fn target_x(&self) -> u32 {
        self.x
    }

    /// Returns a matrix's Y target.
    ///
    /// `targetY` in the SVG.
    #[inline]
    pub fn target_y(&self) -> u32 {
        self.y
    }

    /// Returns a number of columns in the matrix.
    ///
    /// Part of the `order` attribute in the SVG.
    #[inline]
    pub fn columns(&self) -> u32 {
        self.columns
    }

    /// Returns a number of rows in the matrix.
    ///
    /// Part of the `order` attribute in the SVG.
    #[inline]
    pub fn rows(&self) -> u32 {
        self.rows
    }

    /// Returns a matrix value at the specified position.
    ///
    /// # Panics
    ///
    /// - When position is out of bounds.
    #[inline]
    pub fn get(&self, x: u32, y: u32) -> f64 {
        self.data[(y * self.columns + x) as usize]
    }

    /// Returns a reference to an internal data.
    #[inline]
    pub fn data(&self) -> &[f64] {
        &self.data
    }
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

impl_enum_default!(ShapeRendering, GeometricPrecision);

impl_enum_from_str!(ShapeRendering,
    "optimizeSpeed"         => ShapeRendering::OptimizeSpeed,
    "crispEdges"            => ShapeRendering::CrispEdges,
    "geometricPrecision"    => ShapeRendering::GeometricPrecision
);

impl_from_str!(ShapeRendering);


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

impl_enum_default!(TextRendering, OptimizeLegibility);

impl_enum_from_str!(TextRendering,
    "optimizeSpeed"         => TextRendering::OptimizeSpeed,
    "optimizeLegibility"    => TextRendering::OptimizeLegibility,
    "geometricPrecision"    => TextRendering::GeometricPrecision
);

impl_from_str!(TextRendering);


/// An image rendering method.
///
/// `image-rendering` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ImageRendering {
    OptimizeQuality,
    OptimizeSpeed,
}

impl_enum_default!(ImageRendering, OptimizeQuality);

impl_enum_from_str!(ImageRendering,
    "optimizeQuality"   => ImageRendering::OptimizeQuality,
    "optimizeSpeed"     => ImageRendering::OptimizeSpeed
);

impl_from_str!(ImageRendering);


/// An `enable-background`.
///
/// Contains only the `new [ <x> <y> <width> <height> ]` value.
#[derive(Clone, Copy, Debug)]
#[allow(missing_docs)]
pub struct EnableBackground(pub Option<Rect>);
