// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom::types::{
    Color,
    NumberList,
};


/// A line cap.
///
/// `stroke-linecap` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LineCap {
    Butt,
    Round,
    Square,
}

/// A line join.
///
/// `stroke-linejoin` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}

/// A fill rule.
///
/// `fill-rule` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

/// An element units.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Units {
    UserSpaceOnUse,
    ObjectBoundingBox,
}

/// A spread method.
///
/// `spreadMethod` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SpreadMethod {
    Pad,
    Reflect,
    Repeat,
}

/// A text decoration style.
///
/// Defines the style of the line that should be rendered.
#[allow(missing_docs)]
#[derive(Clone)]
pub struct TextDecorationStyle {
    pub fill: Option<Fill>,
    pub stroke: Option<Stroke>,
}

/// A text decoration.
#[derive(Clone)]
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
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TextAnchor {
    Start,
    Middle,
    End,
}

/// A font style.
///
/// `font-style` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

/// A font variant.
///
/// `font-variant` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FontVariant {
    Normal,
    SmallCaps,
}

/// A font weight.
///
/// `font-weight` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FontWeight {
    Normal,
    Bold,
    Bolder,
    Lighter,
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
#[derive(Debug, Copy, Clone, PartialEq)]
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
#[derive(Copy, Clone)]
pub enum Paint {
    /// Paint with a color.
    Color(Color),
    /// Paint using a referenced element.
    ///
    /// The value is an index from the `Document::defs` list.
    /// Use it via `Document::get_defs()` method.
    Link(usize),
}

/// A fill style.
#[allow(missing_docs)]
#[derive(Copy, Clone)]
pub struct Fill {
    pub paint: Paint,
    pub opacity: f64,
    pub rule: FillRule,
}

/// A stroke style.
#[allow(missing_docs)]
#[derive(Clone)]
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

/// A font description.
#[allow(missing_docs)]
#[derive(Clone)]
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

/// A path absolute segment.
///
/// Unlike the SVG spec can contain only `M`, `L`, `C` and `Z` segments.
/// All other segments will be converted to this one.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, PartialEq)]
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
