// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::rc::Rc;

use crate::svgtree::{self, AId};
use crate::{
    converter, paint_server, FuzzyEq, LinearGradient, Opacity, Pattern, RadialGradient, Units,
};
use strict_num::NonZeroPositiveF64;

macro_rules! wrap {
    ($name:ident) => {
        impl From<f64> for $name {
            #[inline]
            fn from(n: f64) -> Self {
                $name::new(n)
            }
        }

        impl PartialEq for $name {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                self.0.fuzzy_eq(&other.0)
            }
        }
    };
}

/// An alias to `NonZeroPositiveF64`.
pub type StrokeWidth = NonZeroPositiveF64;

/// A `stroke-miterlimit` value.
///
/// Just like `f64` but immutable and guarantee to be >=1.0.
#[derive(Clone, Copy, Debug)]
pub struct StrokeMiterlimit(f64);

impl StrokeMiterlimit {
    /// Creates a new `StrokeMiterlimit` value.
    #[inline]
    pub fn new(n: f64) -> Self {
        debug_assert!(n.is_finite());
        debug_assert!(n >= 1.0);

        let n = if !(n >= 1.0) { 1.0 } else { n };

        StrokeMiterlimit(n)
    }

    /// Returns an underlying value.
    #[inline]
    pub fn get(&self) -> f64 {
        self.0
    }
}

impl Default for StrokeMiterlimit {
    #[inline]
    fn default() -> Self {
        StrokeMiterlimit::new(4.0)
    }
}

wrap!(StrokeMiterlimit);

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
            opacity: Opacity::ONE,
            width: StrokeWidth::new(1.0).unwrap(),
            linecap: LineCap::default(),
            linejoin: LineJoin::default(),
        }
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

impl_enum_default!(FillRule, NonZero);

impl_enum_from_str!(FillRule,
    "nonzero" => FillRule::NonZero,
    "evenodd" => FillRule::EvenOdd
);

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
            opacity: Opacity::ONE,
            rule: FillRule::default(),
        }
    }
}

/// A 8-bit RGB color.
#[derive(Clone, Copy, PartialEq, Debug)]
#[allow(missing_docs)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Color {
    /// Constructs a new `Color` from RGB values.
    #[inline]
    pub fn new_rgb(red: u8, green: u8, blue: u8) -> Color {
        Color { red, green, blue }
    }

    /// Constructs a new `Color` set to black.
    #[inline]
    pub fn black() -> Color {
        Color::new_rgb(0, 0, 0)
    }

    /// Constructs a new `Color` set to white.
    #[inline]
    pub fn white() -> Color {
        Color::new_rgb(255, 255, 255)
    }
}

pub(crate) trait SvgColorExt {
    fn split_alpha(self) -> (Color, Opacity);
}

impl SvgColorExt for svgtypes::Color {
    fn split_alpha(self) -> (Color, Opacity) {
        (
            Color::new_rgb(self.red, self.green, self.blue),
            Opacity::new_u8(self.alpha),
        )
    }
}

/// A paint style.
///
/// `paint` value type in the SVG.
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub enum Paint {
    Color(Color),
    LinearGradient(Rc<LinearGradient>),
    RadialGradient(Rc<RadialGradient>),
    Pattern(Rc<Pattern>),
}

impl Paint {
    /// Returns paint server units.
    ///
    /// Returns `None` for `Color`.
    #[inline]
    pub fn units(&self) -> Option<Units> {
        match self {
            Self::Color(_) => None,
            Self::LinearGradient(ref lg) => Some(lg.units),
            Self::RadialGradient(ref rg) => Some(rg.units),
            Self::Pattern(ref patt) => Some(patt.units),
        }
    }
}

impl PartialEq for Paint {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Color(lc), Self::Color(rc)) => lc == rc,
            (Self::LinearGradient(ref lg1), Self::LinearGradient(ref lg2)) => Rc::ptr_eq(lg1, lg2),
            (Self::RadialGradient(ref rg1), Self::RadialGradient(ref rg2)) => Rc::ptr_eq(rg1, rg2),
            (Self::Pattern(ref p1), Self::Pattern(ref p2)) => Rc::ptr_eq(p1, p2),
            _ => false,
        }
    }
}

pub(crate) fn resolve_fill(
    node: svgtree::Node,
    has_bbox: bool,
    state: &converter::State,
    cache: &mut converter::Cache,
) -> Option<Fill> {
    if state.parent_clip_path.is_some() {
        // A `clipPath` child can be filled only with a black color.
        return Some(Fill {
            paint: Paint::Color(Color::black()),
            opacity: Opacity::ONE,
            rule: node.find_attribute(AId::ClipRule).unwrap_or_default(),
        });
    }

    let mut sub_opacity = Opacity::ONE;
    let paint = if let Some(n) = node.find_node_with_attribute(AId::Fill) {
        convert_paint(n, AId::Fill, has_bbox, state, &mut sub_opacity, cache)?
    } else {
        Paint::Color(Color::black())
    };

    Some(Fill {
        paint,
        opacity: sub_opacity
            * node
                .find_attribute(AId::FillOpacity)
                .unwrap_or(Opacity::ONE),
        rule: node.find_attribute(AId::FillRule).unwrap_or_default(),
    })
}

pub(crate) fn resolve_stroke(
    node: svgtree::Node,
    has_bbox: bool,
    state: &converter::State,
    cache: &mut converter::Cache,
) -> Option<Stroke> {
    if state.parent_clip_path.is_some() {
        // A `clipPath` child cannot be stroked.
        return None;
    }

    let mut sub_opacity = Opacity::ONE;
    let paint = if let Some(n) = node.find_node_with_attribute(AId::Stroke) {
        convert_paint(n, AId::Stroke, has_bbox, state, &mut sub_opacity, cache)?
    } else {
        return None;
    };

    let width = node.resolve_valid_length(AId::StrokeWidth, state, 1.0)?;

    // Must be bigger than 1.
    let miterlimit = node.find_attribute(AId::StrokeMiterlimit).unwrap_or(4.0);
    let miterlimit = if miterlimit < 1.0 { 1.0 } else { miterlimit };
    let miterlimit = StrokeMiterlimit::new(miterlimit);

    let stroke = Stroke {
        paint,
        dasharray: conv_dasharray(node, state),
        dashoffset: node.resolve_length(AId::StrokeDashoffset, state, 0.0) as f32,
        miterlimit,
        opacity: sub_opacity
            * node
                .find_attribute(AId::StrokeOpacity)
                .unwrap_or(Opacity::ONE),
        width,
        linecap: node.find_attribute(AId::StrokeLinecap).unwrap_or_default(),
        linejoin: node.find_attribute(AId::StrokeLinejoin).unwrap_or_default(),
    };

    Some(stroke)
}

fn convert_paint(
    node: svgtree::Node,
    aid: AId,
    has_bbox: bool,
    state: &converter::State,
    opacity: &mut Opacity,
    cache: &mut converter::Cache,
) -> Option<Paint> {
    match node.attribute::<&svgtree::AttributeValue>(aid)? {
        svgtree::AttributeValue::CurrentColor => {
            let svg_color: svgtypes::Color = node
                .find_attribute(AId::Color)
                .unwrap_or_else(svgtypes::Color::black);
            let (color, alpha) = svg_color.split_alpha();
            *opacity = alpha;
            Some(Paint::Color(color))
        }
        svgtree::AttributeValue::Color(svg_color) => {
            let (color, alpha) = svg_color.split_alpha();
            *opacity = alpha;
            Some(Paint::Color(color))
        }
        svgtree::AttributeValue::Paint(func_iri, fallback) => {
            if let Some(link) = node.document().element_by_id(func_iri) {
                let tag_name = link.tag_name().unwrap();
                if tag_name.is_paint_server() {
                    match paint_server::convert(link, state, cache) {
                        Some(paint_server::ServerOrColor::Server(paint)) => {
                            // We can use a paint server node with ObjectBoundingBox units
                            // for painting only when the shape itself has a bbox.
                            //
                            // See SVG spec 7.11 for details.
                            if !has_bbox && paint.units() == Some(Units::ObjectBoundingBox) {
                                from_fallback(node, *fallback, opacity)
                            } else {
                                Some(paint)
                            }
                        }
                        Some(paint_server::ServerOrColor::Color { color, opacity: so }) => {
                            *opacity = so;
                            Some(Paint::Color(color))
                        }
                        None => from_fallback(node, *fallback, opacity),
                    }
                } else {
                    log::warn!("'{}' cannot be used to {} a shape.", tag_name, aid);
                    None
                }
            } else {
                from_fallback(node, *fallback, opacity)
            }
        }
        _ => None,
    }
}

fn from_fallback(
    node: svgtree::Node,
    fallback: Option<svgtypes::PaintFallback>,
    opacity: &mut Opacity,
) -> Option<Paint> {
    match fallback? {
        svgtypes::PaintFallback::None => None,
        svgtypes::PaintFallback::CurrentColor => {
            let svg_color: svgtypes::Color = node
                .find_attribute(AId::Color)
                .unwrap_or_else(svgtypes::Color::black);
            let (color, alpha) = svg_color.split_alpha();
            *opacity = alpha;
            Some(Paint::Color(color))
        }
        svgtypes::PaintFallback::Color(svg_color) => {
            let (color, alpha) = svg_color.split_alpha();
            *opacity = alpha;
            Some(Paint::Color(color))
        }
    }
}

// Prepare the 'stroke-dasharray' according to:
// https://www.w3.org/TR/SVG11/painting.html#StrokeDasharrayProperty
fn conv_dasharray(node: svgtree::Node, state: &converter::State) -> Option<Vec<f64>> {
    let node = node.find_node_with_attribute(AId::StrokeDasharray)?;
    let list = super::units::convert_list(node, AId::StrokeDasharray, state)?;

    // `A negative value is an error`
    if list.iter().any(|n| n.is_sign_negative()) {
        return None;
    }

    // `If the sum of the values is zero, then the stroke is rendered
    // as if a value of none were specified.`
    {
        // no Iter::sum(), because of f64

        let mut sum = 0.0f64;
        for n in list.iter() {
            sum += *n;
        }

        if sum.fuzzy_eq(&0.0) {
            return None;
        }
    }

    // `If an odd number of values is provided, then the list of values
    // is repeated to yield an even number of values.`
    if list.len() % 2 != 0 {
        let mut tmp_list = list.clone();
        tmp_list.extend_from_slice(&list);
        return Some(tmp_list);
    }

    Some(list)
}
