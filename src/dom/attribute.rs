// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom::types::{
    Color,
    NumberList,
};


#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LineCap {
    Butt,
    Round,
    Square,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum GradientUnits {
    UserSpaceOnUse,
    ObjectBoundingBox,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SpreadMethod {
    Pad,
    Reflect,
    Repeat,
}

#[derive(Clone)]
pub struct TextDecorationStyle {
    pub fill: Option<Fill>,
    pub stroke: Option<Stroke>,
}

#[derive(Clone)]
pub struct TextDecoration {
    pub underline: Option<TextDecorationStyle>,
    pub overline: Option<TextDecorationStyle>,
    pub line_through: Option<TextDecorationStyle>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TextAnchor {
    Start,
    Middle,
    End,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FontVariant {
    Normal,
    SmallCaps,
}

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

#[derive(Copy,Clone)]
pub enum Paint {
    Color(Color),
    Link(usize),
}

#[derive(Copy,Clone)]
pub struct Fill {
    pub paint: Paint,
    pub opacity: f64,
    pub rule: FillRule,
}

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

#[derive(Clone)]
pub struct Font {
    pub family: String,
    pub size: f64,
    pub style: FontStyle,
    pub variant: FontVariant,
    pub weight: FontWeight,
    pub stretch: FontStretch,
}

#[derive(Copy,Clone,Debug,PartialEq)]
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
