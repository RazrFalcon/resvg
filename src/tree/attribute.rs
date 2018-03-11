// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt;

// external
pub use svgdom::{
    Align,
    AspectRatio,
    Color,
    FuzzyEq,
    NumberList,
    Transform,
};

// self
use super::NodeId;
use math::{
    Rect,
};


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
///
/// `*Units` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Units {
    UserSpaceOnUse,
    ObjectBoundingBox,
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
#[derive(Clone, Copy)]
pub enum Paint {
    /// Paint with a color.
    Color(Color),
    /// Paint using a referenced element.
    Link(NodeId),
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
#[derive(Clone, Copy, Debug)]
pub struct Fill {
    pub paint: Paint,
    pub opacity: f64,
    pub rule: FillRule,
}

impl Default for Fill {
    fn default() -> Self {
        Fill {
            paint: Paint::Color(Color::new(0, 0, 0)),
            opacity: 1.0,
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
    pub dashoffset: f64,
    pub miterlimit: f64,
    pub opacity: f64,
    pub width: f64,
    pub linecap: LineCap,
    pub linejoin: LineJoin,
}

impl Default for Stroke {
    fn default() -> Self {
        Stroke {
            paint: Paint::Color(Color::new(0, 0, 0)),
            dasharray: None,
            dashoffset: 0.0,
            miterlimit: 4.0,
            opacity: 1.0,
            width: 1.0,
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
    pub size: f64,
    pub style: FontStyle,
    pub variant: FontVariant,
    pub weight: FontWeight,
    pub stretch: FontStretch,
}

impl Default for Font {
    fn default() -> Self {
        Font {
            family: ::DEFAULT_FONT_FAMILY.to_owned(),
            size: ::DEFAULT_FONT_SIZE,
            style: FontStyle::Normal,
            variant: FontVariant::Normal,
            weight: FontWeight::W400,
            stretch: FontStretch::Normal,
        }
    }
}


/// View box.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ViewBox {
    /// Value of the `viewBox` attribute.
    pub rect: Rect,
    /// Value of the `preserveAspectRatio` attribute.
    pub aspect: AspectRatio,
}


/// A path absolute segment.
///
/// Unlike the SVG spec can contain only `M`, `L`, `C` and `Z` segments.
/// All other segments will be converted to this one.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
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
