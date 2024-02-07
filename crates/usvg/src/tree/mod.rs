// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

pub mod filter;
mod geom;
mod text;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

pub use strict_num::{self, ApproxEqUlps, NonZeroPositiveF32, NormalizedF32, PositiveF32};
pub use svgtypes::{Align, AspectRatio};

pub use tiny_skia_path;

pub use self::geom::*;
pub use self::text::*;
use crate::PostProcessingSteps;

/// An alias to `NormalizedF32`.
pub type Opacity = NormalizedF32;

/// A non-zero `f32`.
///
/// Just like `f32` but immutable and guarantee to never be zero.
#[derive(Clone, Copy, Debug)]
pub struct NonZeroF32(f32);

impl NonZeroF32 {
    /// Creates a new `NonZeroF32` value.
    #[inline]
    pub fn new(n: f32) -> Option<Self> {
        if n.approx_eq_ulps(&0.0, 4) {
            None
        } else {
            Some(NonZeroF32(n))
        }
    }

    /// Returns an underlying value.
    #[inline]
    pub fn get(&self) -> f32 {
        self.0
    }
}

/// An element units.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Units {
    UserSpaceOnUse,
    ObjectBoundingBox,
}

// `Units` cannot have a default value, because it changes depending on an element.

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

impl Default for Visibility {
    fn default() -> Self {
        Self::Visible
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

impl ShapeRendering {
    /// Checks if anti-aliasing should be enabled.
    pub fn use_shape_antialiasing(self) -> bool {
        match self {
            ShapeRendering::OptimizeSpeed => false,
            ShapeRendering::CrispEdges => false,
            ShapeRendering::GeometricPrecision => true,
        }
    }
}

impl Default for ShapeRendering {
    fn default() -> Self {
        Self::GeometricPrecision
    }
}

// TODO: remove?
impl std::str::FromStr for ShapeRendering {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "optimizeSpeed" => Ok(ShapeRendering::OptimizeSpeed),
            "crispEdges" => Ok(ShapeRendering::CrispEdges),
            "geometricPrecision" => Ok(ShapeRendering::GeometricPrecision),
            _ => Err("invalid"),
        }
    }
}

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

impl Default for TextRendering {
    fn default() -> Self {
        Self::OptimizeLegibility
    }
}

impl std::str::FromStr for TextRendering {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "optimizeSpeed" => Ok(TextRendering::OptimizeSpeed),
            "optimizeLegibility" => Ok(TextRendering::OptimizeLegibility),
            "geometricPrecision" => Ok(TextRendering::GeometricPrecision),
            _ => Err("invalid"),
        }
    }
}

/// An image rendering method.
///
/// `image-rendering` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ImageRendering {
    OptimizeQuality,
    OptimizeSpeed,
}

impl Default for ImageRendering {
    fn default() -> Self {
        Self::OptimizeQuality
    }
}

impl std::str::FromStr for ImageRendering {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "optimizeQuality" => Ok(ImageRendering::OptimizeQuality),
            "optimizeSpeed" => Ok(ImageRendering::OptimizeSpeed),
            _ => Err("invalid"),
        }
    }
}

/// A blending mode property.
///
/// `mix-blend-mode` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

impl Default for BlendMode {
    fn default() -> Self {
        Self::Normal
    }
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

impl Default for SpreadMethod {
    fn default() -> Self {
        Self::Pad
    }
}

/// A generic gradient.
#[derive(Clone, Debug)]
pub struct BaseGradient {
    pub(crate) id: String,
    pub(crate) units: Units,
    pub(crate) transform: Transform,
    pub(crate) spread_method: SpreadMethod,
    pub(crate) stops: Vec<Stop>,
}

impl BaseGradient {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Used only during SVG writing. `resvg` doesn't rely on this property.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Coordinate system units.
    ///
    /// `gradientUnits` in SVG.
    pub fn units(&self) -> Units {
        self.units
    }

    /// Gradient transform.
    ///
    /// `gradientTransform` in SVG.
    pub fn transform(&self) -> Transform {
        self.transform
    }

    /// Gradient spreading method.
    ///
    /// `spreadMethod` in SVG.
    pub fn spread_method(&self) -> SpreadMethod {
        self.spread_method
    }

    /// A list of `stop` elements.
    pub fn stops(&self) -> &[Stop] {
        &self.stops
    }
}

/// A linear gradient.
///
/// `linearGradient` element in SVG.
#[derive(Clone, Debug)]
pub struct LinearGradient {
    pub(crate) base: BaseGradient,
    pub(crate) x1: f32,
    pub(crate) y1: f32,
    pub(crate) x2: f32,
    pub(crate) y2: f32,
}

impl LinearGradient {
    /// `x1` coordinate.
    pub fn x1(&self) -> f32 {
        self.x1
    }

    /// `y1` coordinate.
    pub fn y1(&self) -> f32 {
        self.y1
    }

    /// `x2` coordinate.
    pub fn x2(&self) -> f32 {
        self.x2
    }

    /// `y2` coordinate.
    pub fn y2(&self) -> f32 {
        self.y2
    }
}

impl std::ops::Deref for LinearGradient {
    type Target = BaseGradient;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

/// A radial gradient.
///
/// `radialGradient` element in SVG.
#[derive(Clone, Debug)]
pub struct RadialGradient {
    pub(crate) base: BaseGradient,
    pub(crate) cx: f32,
    pub(crate) cy: f32,
    pub(crate) r: PositiveF32,
    pub(crate) fx: f32,
    pub(crate) fy: f32,
}

impl RadialGradient {
    /// `cx` coordinate.
    pub fn cx(&self) -> f32 {
        self.cx
    }

    /// `cy` coordinate.
    pub fn cy(&self) -> f32 {
        self.cy
    }

    /// Gradient radius.
    pub fn r(&self) -> PositiveF32 {
        self.r
    }

    /// `fx` coordinate.
    pub fn fx(&self) -> f32 {
        self.fx
    }

    /// `fy` coordinate.
    pub fn fy(&self) -> f32 {
        self.fy
    }
}

impl std::ops::Deref for RadialGradient {
    type Target = BaseGradient;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

/// An alias to `NormalizedF32`.
pub type StopOffset = NormalizedF32;

/// Gradient's stop element.
///
/// `stop` element in SVG.
#[derive(Clone, Copy, Debug)]
pub struct Stop {
    pub(crate) offset: StopOffset,
    pub(crate) color: Color,
    pub(crate) opacity: Opacity,
}

impl Stop {
    /// Gradient stop offset.
    ///
    /// `offset` in SVG.
    pub fn offset(&self) -> StopOffset {
        self.offset
    }

    /// Gradient stop color.
    ///
    /// `stop-color` in SVG.
    pub fn color(&self) -> Color {
        self.color
    }

    /// Gradient stop opacity.
    ///
    /// `stop-opacity` in SVG.
    pub fn opacity(&self) -> Opacity {
        self.opacity
    }
}

/// A pattern element.
///
/// `pattern` element in SVG.
#[derive(Clone, Debug)]
pub struct Pattern {
    pub(crate) id: String,
    pub(crate) units: Units,
    pub(crate) content_units: Units,
    pub(crate) transform: Transform,
    pub(crate) rect: NonZeroRect,
    pub(crate) view_box: Option<ViewBox>,
    pub(crate) root: Group,
}

impl Pattern {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Used only during SVG writing. `resvg` doesn't rely on this property.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Coordinate system units.
    ///
    /// `patternUnits` in SVG.
    pub fn units(&self) -> Units {
        self.units
    }

    // TODO: should not be accessible when `viewBox` is present.
    /// Content coordinate system units.
    ///
    /// `patternContentUnits` in SVG.
    pub fn content_units(&self) -> Units {
        self.content_units
    }

    /// Pattern transform.
    ///
    /// `patternTransform` in SVG.
    pub fn transform(&self) -> Transform {
        self.transform
    }

    /// Pattern rectangle.
    ///
    /// `x`, `y`, `width` and `height` in SVG.
    pub fn rect(&self) -> NonZeroRect {
        self.rect
    }

    /// Pattern viewbox.
    pub fn view_box(&self) -> Option<ViewBox> {
        self.view_box
    }

    /// Pattern children.
    pub fn root(&self) -> &Group {
        &self.root
    }
}

/// An alias to `NonZeroPositiveF32`.
pub type StrokeWidth = NonZeroPositiveF32;

/// A `stroke-miterlimit` value.
///
/// Just like `f32` but immutable and guarantee to be >=1.0.
#[derive(Clone, Copy, Debug)]
pub struct StrokeMiterlimit(f32);

impl StrokeMiterlimit {
    /// Creates a new `StrokeMiterlimit` value.
    #[inline]
    pub fn new(n: f32) -> Self {
        debug_assert!(n.is_finite());
        debug_assert!(n >= 1.0);

        let n = if !(n >= 1.0) { 1.0 } else { n };

        StrokeMiterlimit(n)
    }

    /// Returns an underlying value.
    #[inline]
    pub fn get(&self) -> f32 {
        self.0
    }
}

impl Default for StrokeMiterlimit {
    #[inline]
    fn default() -> Self {
        StrokeMiterlimit::new(4.0)
    }
}

impl From<f32> for StrokeMiterlimit {
    #[inline]
    fn from(n: f32) -> Self {
        Self::new(n)
    }
}

impl PartialEq for StrokeMiterlimit {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0.approx_eq_ulps(&other.0, 4)
    }
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

impl Default for LineCap {
    fn default() -> Self {
        Self::Butt
    }
}

/// A line join.
///
/// `stroke-linejoin` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LineJoin {
    Miter,
    MiterClip,
    Round,
    Bevel,
}

impl Default for LineJoin {
    fn default() -> Self {
        Self::Miter
    }
}

/// A stroke style.
#[derive(Clone, Debug)]
pub struct Stroke {
    pub(crate) paint: Paint,
    pub(crate) dasharray: Option<Vec<f32>>,
    pub(crate) dashoffset: f32,
    pub(crate) miterlimit: StrokeMiterlimit,
    pub(crate) opacity: Opacity,
    pub(crate) width: StrokeWidth,
    pub(crate) linecap: LineCap,
    pub(crate) linejoin: LineJoin,
}

impl Stroke {
    /// Stroke paint.
    pub fn paint(&self) -> &Paint {
        &self.paint
    }

    /// Stroke dash array.
    pub fn dasharray(&self) -> Option<&[f32]> {
        self.dasharray.as_deref()
    }

    /// Stroke dash offset.
    pub fn dashoffset(&self) -> f32 {
        self.dashoffset
    }

    /// Stroke miter limit.
    pub fn miterlimit(&self) -> StrokeMiterlimit {
        self.miterlimit
    }

    /// Stroke opacity.
    pub fn opacity(&self) -> Opacity {
        self.opacity
    }

    /// Stroke width.
    pub fn width(&self) -> StrokeWidth {
        self.width
    }

    /// Stroke linecap.
    pub fn linecap(&self) -> LineCap {
        self.linecap
    }

    /// Stroke linejoin.
    pub fn linejoin(&self) -> LineJoin {
        self.linejoin
    }

    /// Converts into a `tiny_skia_path::Stroke` type.
    pub fn to_tiny_skia(&self) -> tiny_skia_path::Stroke {
        let mut stroke = tiny_skia_path::Stroke {
            width: self.width.get(),
            miter_limit: self.miterlimit.get(),
            line_cap: match self.linecap {
                LineCap::Butt => tiny_skia_path::LineCap::Butt,
                LineCap::Round => tiny_skia_path::LineCap::Round,
                LineCap::Square => tiny_skia_path::LineCap::Square,
            },
            line_join: match self.linejoin {
                LineJoin::Miter => tiny_skia_path::LineJoin::Miter,
                LineJoin::MiterClip => tiny_skia_path::LineJoin::MiterClip,
                LineJoin::Round => tiny_skia_path::LineJoin::Round,
                LineJoin::Bevel => tiny_skia_path::LineJoin::Bevel,
            },
            // According to the spec, dash should not be accounted during
            // bbox calculation.
            dash: None,
        };

        if let Some(ref list) = self.dasharray {
            stroke.dash = tiny_skia_path::StrokeDash::new(list.clone(), self.dashoffset);
        }

        stroke
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

impl Default for FillRule {
    fn default() -> Self {
        Self::NonZero
    }
}

/// A fill style.
#[derive(Clone, Debug)]
pub struct Fill {
    pub(crate) paint: Paint,
    pub(crate) opacity: Opacity,
    pub(crate) rule: FillRule,
}

impl Fill {
    /// Fill paint.
    pub fn paint(&self) -> &Paint {
        &self.paint
    }

    /// Fill opacity.
    pub fn opacity(&self) -> Opacity {
        self.opacity
    }

    /// Fill rule.
    pub fn rule(&self) -> FillRule {
        self.rule
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

/// A paint style.
///
/// `paint` value type in the SVG.
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub enum Paint {
    Color(Color),
    LinearGradient(Rc<LinearGradient>),
    RadialGradient(Rc<RadialGradient>),
    Pattern(Rc<RefCell<Pattern>>),
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
            Self::Pattern(ref patt) => Some(patt.borrow().units),
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

/// A clip-path element.
///
/// `clipPath` element in SVG.
#[derive(Clone, Debug)]
pub struct ClipPath {
    pub(crate) id: String,
    pub(crate) units: Units,
    pub(crate) transform: Transform,
    pub(crate) clip_path: Option<SharedClipPath>,
    pub(crate) root: Group,
}

impl ClipPath {
    pub(crate) fn empty() -> Self {
        ClipPath {
            id: String::new(),
            units: Units::UserSpaceOnUse,
            transform: Transform::default(),
            clip_path: None,
            root: Group::empty(),
        }
    }

    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Used only during SVG writing. `resvg` doesn't rely on this property.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Coordinate system units.
    ///
    /// `clipPathUnits` in SVG.
    pub fn units(&self) -> Units {
        self.units
    }

    /// Clip path transform.
    ///
    /// `transform` in SVG.
    pub fn transform(&self) -> Transform {
        self.transform
    }

    /// Additional clip path.
    ///
    /// `clip-path` in SVG.
    pub fn clip_path(&self) -> Option<SharedClipPath> {
        self.clip_path.clone()
    }

    /// Clip path children.
    pub fn root(&self) -> &Group {
        &self.root
    }
}

/// An alias for a shared `ClipPath`.
pub type SharedClipPath = Rc<RefCell<ClipPath>>;

/// A mask type.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MaskType {
    /// Indicates that the luminance values of the mask should be used.
    Luminance,
    /// Indicates that the alpha values of the mask should be used.
    Alpha,
}

impl Default for MaskType {
    fn default() -> Self {
        Self::Luminance
    }
}

/// A mask element.
///
/// `mask` element in SVG.
#[derive(Clone, Debug)]
pub struct Mask {
    pub(crate) id: String,
    pub(crate) units: Units,
    pub(crate) content_units: Units,
    pub(crate) rect: NonZeroRect,
    pub(crate) kind: MaskType,
    pub(crate) mask: Option<SharedMask>,
    pub(crate) root: Group,
}

impl Mask {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Used only during SVG writing. `resvg` doesn't rely on this property.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Coordinate system units.
    ///
    /// `maskUnits` in SVG.
    pub fn units(&self) -> Units {
        self.units
    }

    /// Content coordinate system units.
    ///
    /// `maskContentUnits` in SVG.
    pub fn content_units(&self) -> Units {
        self.content_units
    }

    /// Mask rectangle.
    ///
    /// `x`, `y`, `width` and `height` in SVG.
    pub fn rect(&self) -> NonZeroRect {
        self.rect
    }

    /// Mask type.
    ///
    /// `mask-type` in SVG.
    pub fn kind(&self) -> MaskType {
        self.kind
    }

    /// Additional mask.
    ///
    /// `mask` in SVG.
    pub fn mask(&self) -> Option<SharedMask> {
        self.mask.clone()
    }

    /// Mask children.
    pub fn root(&self) -> &Group {
        &self.root
    }
}

/// An alias for a shared `Mask`.
pub type SharedMask = Rc<RefCell<Mask>>;

/// Node's kind.
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub enum Node {
    Group(Box<Group>),
    Path(Box<Path>),
    Image(Box<Image>),
    Text(Box<Text>),
}

impl Node {
    /// Returns node's ID.
    pub fn id(&self) -> &str {
        match self {
            Node::Group(ref e) => e.id.as_str(),
            Node::Path(ref e) => e.id.as_str(),
            Node::Image(ref e) => e.id.as_str(),
            Node::Text(ref e) => e.id.as_str(),
        }
    }

    /// Returns node's absolute transform.
    ///
    /// If a current node doesn't support transformation - a default
    /// transform will be returned.
    ///
    /// This method is cheap since absolute transforms are already resolved.
    pub fn abs_transform(&self) -> Transform {
        match self {
            Node::Group(ref group) => group.abs_transform,
            Node::Path(ref path) => path.abs_transform,
            Node::Image(ref image) => image.abs_transform,
            Node::Text(ref text) => text.abs_transform,
        }
    }

    /// Returns node's bounding box in object coordinates, if any.
    ///
    /// This method is cheap since bounding boxes are already calculated.
    pub fn bounding_box(&self) -> Option<Rect> {
        match self {
            Node::Group(ref group) => group.bounding_box,
            Node::Path(ref path) => path.bounding_box,
            Node::Image(ref image) => image.bounding_box.map(|r| r.to_rect()),
            Node::Text(ref text) => text.bounding_box.map(|r| r.to_rect()),
        }
    }

    /// Returns node's bounding box in canvas coordinates, if any.
    ///
    /// This method is cheap since bounding boxes are already calculated.
    pub fn abs_bounding_box(&self) -> Option<Rect> {
        match self {
            Node::Group(ref group) => group.abs_bounding_box,
            Node::Path(ref path) => path.abs_bounding_box,
            Node::Image(ref image) => image.abs_bounding_box().map(|r| r.to_rect()),
            Node::Text(ref text) => text.abs_bounding_box.map(|r| r.to_rect()),
        }
    }

    /// Returns node's bounding box, including stroke, in object coordinates, if any.
    ///
    /// This method is cheap since bounding boxes are already calculated.
    pub fn stroke_bounding_box(&self) -> Option<NonZeroRect> {
        match self {
            Node::Group(ref group) => group.stroke_bounding_box,
            Node::Path(ref path) => path.stroke_bounding_box,
            // Image cannot be stroked.
            Node::Image(ref image) => image.bounding_box,
            Node::Text(ref text) => text.stroke_bounding_box,
        }
    }

    /// Returns node's bounding box, including stroke, in canvas coordinates, if any.
    ///
    /// This method is cheap since bounding boxes are already calculated.
    pub fn abs_stroke_bounding_box(&self) -> Option<NonZeroRect> {
        match self {
            Node::Group(ref group) => group.abs_stroke_bounding_box,
            Node::Path(ref path) => path.abs_stroke_bounding_box,
            // Image cannot be stroked.
            Node::Image(ref image) => image.abs_bounding_box(),
            Node::Text(ref text) => text.abs_stroke_bounding_box,
        }
    }

    /// Calls a closure for each subroot this `Node` has.
    ///
    /// The [`Tree::root`](Tree::root) field contain only render-able SVG elements.
    /// But some elements, specifically clip paths, masks, patterns and feImage
    /// can store their own SVG subtrees.
    /// And while one can access them manually, it's pretty verbose.
    /// This methods allows looping over _all_ SVG elements present in the `Tree`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// fn all_nodes(parent: &usvg::Group) {
    ///     for node in parent.children() {
    ///         // do stuff...
    ///
    ///         if let usvg::Node::Group(ref g) = node {
    ///             all_nodes(g);
    ///         }
    ///
    ///         // handle subroots as well
    ///         node.subroots(|subroot| all_nodes(subroot));
    ///     }
    /// }
    /// ```
    pub fn subroots<F: FnMut(&Group)>(&self, mut f: F) {
        match self {
            Node::Group(ref group) => group.subroots(&mut f),
            Node::Path(ref path) => path.subroots(&mut f),
            Node::Image(ref image) => image.subroots(&mut f),
            Node::Text(ref text) => text.subroots(&mut f),
        }
    }

    /// Calls a closure for each subroot this `Node` has.
    ///
    /// A mutable version of `subroots()`.
    pub fn subroots_mut<F: FnMut(&mut Group)>(&mut self, mut f: F) {
        match self {
            Node::Group(ref mut group) => group.subroots_mut(&mut f),
            Node::Path(ref mut path) => path.subroots_mut(&mut f),
            Node::Image(ref mut image) => image.subroots_mut(&mut f),
            Node::Text(ref mut text) => text.subroots_mut(&mut f),
        }
    }
}

/// A group container.
///
/// The preprocessor will remove all groups that don't impact rendering.
/// Those that left is just an indicator that a new canvas should be created.
///
/// `g` element in SVG.
#[derive(Clone, Debug)]
pub struct Group {
    pub(crate) id: String,
    pub(crate) transform: Transform,
    pub(crate) abs_transform: Transform,
    pub(crate) opacity: Opacity,
    pub(crate) blend_mode: BlendMode,
    pub(crate) isolate: bool,
    pub(crate) clip_path: Option<SharedClipPath>,
    pub(crate) mask: Option<SharedMask>,
    pub(crate) filters: Vec<filter::SharedFilter>,
    pub(crate) bounding_box: Option<Rect>,
    pub(crate) abs_bounding_box: Option<Rect>,
    pub(crate) stroke_bounding_box: Option<NonZeroRect>,
    pub(crate) abs_stroke_bounding_box: Option<NonZeroRect>,
    pub(crate) layer_bounding_box: Option<NonZeroRect>,
    pub(crate) children: Vec<Node>,
}

impl Group {
    pub(crate) fn empty() -> Self {
        Group {
            id: String::new(),
            transform: Transform::default(),
            abs_transform: Transform::default(),
            opacity: Opacity::ONE,
            blend_mode: BlendMode::Normal,
            isolate: false,
            clip_path: None,
            mask: None,
            filters: Vec::new(),
            bounding_box: None,
            abs_bounding_box: None,
            stroke_bounding_box: None,
            abs_stroke_bounding_box: None,
            layer_bounding_box: None,
            children: Vec::new(),
        }
    }

    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Isn't automatically generated.
    /// Can be empty.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Element's transform.
    ///
    /// This is a relative transform. The one that is set via the `transform` attribute in SVG.
    pub fn transform(&self) -> Transform {
        self.transform
    }

    /// Element's absolute transform.
    ///
    /// Contains all ancestors transforms.
    ///
    /// Will be set after calling `usvg::Tree::postprocess`.
    ///
    /// Note that subroots, like clipPaths, masks and patterns, have their own root transform,
    /// which isn't affected by the node that references this subroot.
    pub fn abs_transform(&self) -> Transform {
        self.abs_transform
    }

    /// Group opacity.
    ///
    /// After the group is rendered we should combine
    /// it with a parent group using the specified opacity.
    pub fn opacity(&self) -> Opacity {
        self.opacity
    }

    /// Group blend mode.
    ///
    /// `mix-blend-mode` in SVG.
    pub fn blend_mode(&self) -> BlendMode {
        self.blend_mode
    }

    /// Group isolation.
    ///
    /// `isolation` in SVG.
    pub fn isolate(&self) -> bool {
        self.isolate
    }

    /// Element's clip path.
    pub fn clip_path(&self) -> Option<SharedClipPath> {
        self.clip_path.clone()
    }

    /// Element's mask.
    pub fn mask(&self) -> Option<SharedMask> {
        self.mask.clone()
    }

    /// Element's filters.
    pub fn filters(&self) -> &[filter::SharedFilter] {
        &self.filters
    }

    /// Element's object bounding box.
    ///
    /// `objectBoundingBox` in SVG terms. Meaning it doesn't affected by parent transforms.
    ///
    /// Can be set to `None` in case of an empty group.
    ///
    /// Will be set after calling `usvg::Tree::postprocess`.
    pub fn bounding_box(&self) -> Option<Rect> {
        self.bounding_box
    }

    /// Element's bounding box in canvas coordinates.
    ///
    /// `userSpaceOnUse` in SVG terms.
    ///
    /// Will be set after calling `usvg::Tree::postprocess`.
    pub fn abs_bounding_box(&self) -> Option<Rect> {
        self.abs_bounding_box
    }

    /// Element's object bounding box including stroke.
    ///
    /// Similar to `bounding_box`, but includes stroke.
    pub fn stroke_bounding_box(&self) -> Option<NonZeroRect> {
        self.stroke_bounding_box
    }

    /// Element's bounding box including stroke in user coordinates.
    ///
    /// Similar to `abs_bounding_box`, but includes stroke.
    pub fn abs_stroke_bounding_box(&self) -> Option<NonZeroRect> {
        self.abs_stroke_bounding_box
    }

    /// Element's "layer" bounding box in object units.
    ///
    /// Conceptually, this is `stroke_bounding_box` expanded and/or clipped
    /// by `filters_bounding_box`, but also including all the children.
    /// This is the bounding box `resvg` will later use to allocate layers/pixmaps
    /// during isolated groups rendering.
    ///
    /// Only groups have it, because only groups can have filters.
    /// For other nodes layer bounding box is the same as stroke bounding box.
    ///
    /// Unlike other bounding boxes, cannot have zero size.
    ///
    /// Will be set after calling `usvg::Tree::postprocess`.
    pub fn layer_bounding_box(&self) -> Option<NonZeroRect> {
        self.layer_bounding_box
    }

    /// Group's children.
    pub fn children(&self) -> &[Node] {
        &self.children
    }

    /// Checks if this group should be isolated during rendering.
    pub fn should_isolate(&self) -> bool {
        self.isolate
            || self.opacity != Opacity::ONE
            || self.clip_path.is_some()
            || self.mask.is_some()
            || !self.filters.is_empty()
            || self.blend_mode != BlendMode::Normal // TODO: probably not needed?
    }

    /// Returns `true` if the group has any children.
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    /// Calculates a node's filter bounding box.
    ///
    /// Filters with `objectBoundingBox` and missing or zero `bounding_box` would be ignored.
    ///
    /// Note that a filter region can act like a clipping rectangle,
    /// therefore this function can produce a bounding box smaller than `bounding_box`.
    ///
    /// Returns `None` when then group has no filters.
    ///
    /// This function is very fast, that's why we do not store this bbox as a `Group` field.
    pub fn filters_bounding_box(&self) -> Option<NonZeroRect> {
        let mut full_region = BBox::default();

        for filter in &self.filters {
            let mut region = filter.borrow().rect;

            if filter.borrow().units == Units::ObjectBoundingBox {
                let object_bbox = self.bounding_box.and_then(|bbox| bbox.to_non_zero_rect());
                if let Some(object_bbox) = object_bbox {
                    region = region.bbox_transform(object_bbox);
                } else {
                    // Skip filters with `objectBoundingBox` on nodes without a bbox.
                    continue;
                }
            }

            full_region = full_region.expand(BBox::from(region));
        }

        full_region.to_non_zero_rect()
    }

    fn subroots(&self, f: &mut dyn FnMut(&Group)) {
        if let Some(ref clip) = self.clip_path {
            f(&clip.borrow().root);

            if let Some(ref sub_clip) = clip.borrow().clip_path {
                f(&sub_clip.borrow().root);
            }
        }

        if let Some(ref mask) = self.mask {
            f(&mask.borrow().root);

            if let Some(ref sub_mask) = mask.borrow().mask {
                f(&sub_mask.borrow().root);
            }
        }

        for filter in &self.filters {
            for primitive in &filter.borrow().primitives {
                if let filter::Kind::Image(ref image) = primitive.kind {
                    if let filter::ImageKind::Use(ref use_node) = image.data {
                        f(use_node);
                    }
                }
            }
        }
    }

    fn subroots_mut(&mut self, f: &mut dyn FnMut(&mut Group)) {
        if let Some(ref clip) = self.clip_path {
            f(&mut clip.borrow_mut().root);

            if let Some(ref sub_clip) = clip.borrow().clip_path {
                f(&mut sub_clip.borrow_mut().root);
            }
        }

        if let Some(ref mask) = self.mask {
            f(&mut mask.borrow_mut().root);

            if let Some(ref sub_mask) = mask.borrow_mut().mask {
                f(&mut sub_mask.borrow_mut().root);
            }
        }

        for filter in &mut self.filters {
            for primitive in &mut filter.borrow_mut().primitives {
                if let filter::Kind::Image(ref mut image) = primitive.kind {
                    if let filter::ImageKind::Use(ref mut use_node) = image.data {
                        f(use_node);
                    }
                }
            }
        }
    }
}

/// Representation of the [`paint-order`] property.
///
/// `usvg` will handle `markers` automatically,
/// therefore we provide only `fill` and `stroke` variants.
///
/// [`paint-order`]: https://www.w3.org/TR/SVG2/painting.html#PaintOrder
#[derive(Clone, Copy, PartialEq, Debug)]
#[allow(missing_docs)]
pub enum PaintOrder {
    FillAndStroke,
    StrokeAndFill,
}

impl Default for PaintOrder {
    fn default() -> Self {
        Self::FillAndStroke
    }
}

/// A path element.
#[derive(Clone, Debug)]
pub struct Path {
    pub(crate) id: String,
    pub(crate) visibility: Visibility,
    pub(crate) fill: Option<Fill>,
    pub(crate) stroke: Option<Stroke>,
    pub(crate) paint_order: PaintOrder,
    pub(crate) rendering_mode: ShapeRendering,
    pub(crate) data: Rc<tiny_skia_path::Path>,
    pub(crate) abs_transform: Transform,
    pub(crate) bounding_box: Option<Rect>,
    pub(crate) abs_bounding_box: Option<Rect>,
    pub(crate) stroke_bounding_box: Option<NonZeroRect>,
    pub(crate) abs_stroke_bounding_box: Option<NonZeroRect>,
}

impl Path {
    /// Creates a new `Path` with default values.
    pub(crate) fn with_path(data: Rc<tiny_skia_path::Path>) -> Self {
        Path {
            id: String::new(),
            visibility: Visibility::Visible,
            fill: None,
            stroke: None,
            paint_order: PaintOrder::default(),
            rendering_mode: ShapeRendering::default(),
            data,
            abs_transform: Transform::default(),
            bounding_box: None,
            abs_bounding_box: None,
            stroke_bounding_box: None,
            abs_stroke_bounding_box: None,
        }
    }

    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Isn't automatically generated.
    /// Can be empty.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Element visibility.
    pub fn visibility(&self) -> Visibility {
        self.visibility
    }

    /// Fill style.
    pub fn fill(&self) -> Option<&Fill> {
        self.fill.as_ref()
    }

    /// Stroke style.
    pub fn stroke(&self) -> Option<&Stroke> {
        self.stroke.as_ref()
    }

    /// Fill and stroke paint order.
    ///
    /// Since markers will be replaced with regular nodes automatically,
    /// `usvg` doesn't provide the `markers` order type. It's was already done.
    ///
    /// `paint-order` in SVG.
    pub fn paint_order(&self) -> PaintOrder {
        self.paint_order
    }

    /// Rendering mode.
    ///
    /// `shape-rendering` in SVG.
    pub fn rendering_mode(&self) -> ShapeRendering {
        self.rendering_mode
    }

    // TODO: find a better name
    /// Segments list.
    ///
    /// All segments are in absolute coordinates.
    pub fn data(&self) -> &tiny_skia_path::Path {
        self.data.as_ref()
    }

    /// Element's absolute transform.
    ///
    /// Contains all ancestors transforms.
    ///
    /// Will be set after calling `usvg::Tree::postprocess`.
    ///
    /// Note that this is not the relative transform present in SVG.
    /// The SVG one would be set only on groups.
    pub fn abs_transform(&self) -> Transform {
        self.abs_transform
    }

    /// Element's object bounding box.
    ///
    /// `objectBoundingBox` in SVG terms. Meaning it doesn't affected by parent transforms.
    ///
    /// Will be set after calling `usvg::Tree::postprocess`.
    pub fn bounding_box(&self) -> Option<Rect> {
        self.bounding_box
    }

    /// Element's bounding box in canvas coordinates.
    ///
    /// `userSpaceOnUse` in SVG terms.
    ///
    /// Will be set after calling `usvg::Tree::postprocess`.
    pub fn abs_bounding_box(&self) -> Option<Rect> {
        self.abs_bounding_box
    }

    /// Element's object bounding box including stroke.
    ///
    /// Similar to `bounding_box`, but includes stroke.
    ///
    /// Will have the same value as `bounding_box` when path has no stroke.
    pub fn stroke_bounding_box(&self) -> Option<NonZeroRect> {
        self.stroke_bounding_box
    }

    /// Element's bounding box including stroke in canvas coordinates.
    ///
    /// Similar to `abs_bounding_box`, but includes stroke.
    ///
    /// Will have the same value as `abs_bounding_box` when path has no stroke.
    pub fn abs_stroke_bounding_box(&self) -> Option<NonZeroRect> {
        self.abs_stroke_bounding_box
    }

    /// Calculates and sets path's stroke bounding box.
    ///
    /// This operation is expensive.
    pub(crate) fn calculate_stroke_bounding_box(&self) -> Option<NonZeroRect> {
        Self::calculate_stroke_bounding_box_inner(self.stroke.as_ref(), &self.data)
    }

    fn calculate_stroke_bounding_box_inner(
        stroke: Option<&Stroke>,
        path: &tiny_skia_path::Path,
    ) -> Option<NonZeroRect> {
        let mut stroke = stroke?.to_tiny_skia();
        // According to the spec, dash should not be accounted during bbox calculation.
        stroke.dash = None;

        // TODO: avoid for round and bevel caps

        // Expensive, but there is not much we can do about it.
        if let Some(stroked_path) = path.stroke(&stroke, 1.0) {
            // A stroked path cannot have zero width or height,
            // therefore we use `NonZeroRect` here.
            return stroked_path
                .compute_tight_bounds()
                .and_then(|r| r.to_non_zero_rect());
        }

        None
    }

    fn subroots(&self, f: &mut dyn FnMut(&Group)) {
        if let Some(Paint::Pattern(ref patt)) = self.fill.as_ref().map(|f| &f.paint) {
            f(&patt.borrow().root)
        }
        if let Some(Paint::Pattern(ref patt)) = self.stroke.as_ref().map(|f| &f.paint) {
            f(&patt.borrow().root)
        }
    }

    fn subroots_mut(&mut self, f: &mut dyn FnMut(&mut Group)) {
        if let Some(Paint::Pattern(ref mut patt)) = self.fill.as_mut().map(|f| &mut f.paint) {
            f(&mut patt.borrow_mut().root)
        }
        if let Some(Paint::Pattern(ref mut patt)) = self.stroke.as_mut().map(|f| &mut f.paint) {
            f(&mut patt.borrow_mut().root)
        }
    }
}

/// An embedded image kind.
#[derive(Clone)]
pub enum ImageKind {
    /// A reference to raw JPEG data. Should be decoded by the caller.
    JPEG(Arc<Vec<u8>>),
    /// A reference to raw PNG data. Should be decoded by the caller.
    PNG(Arc<Vec<u8>>),
    /// A reference to raw GIF data. Should be decoded by the caller.
    GIF(Arc<Vec<u8>>),
    /// A preprocessed SVG tree. Can be rendered as is.
    SVG(Tree),
}

impl std::fmt::Debug for ImageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ImageKind::JPEG(_) => f.write_str("ImageKind::JPEG(..)"),
            ImageKind::PNG(_) => f.write_str("ImageKind::PNG(..)"),
            ImageKind::GIF(_) => f.write_str("ImageKind::GIF(..)"),
            ImageKind::SVG(_) => f.write_str("ImageKind::SVG(..)"),
        }
    }
}

/// A raster image element.
///
/// `image` element in SVG.
#[derive(Clone, Debug)]
pub struct Image {
    pub(crate) id: String,
    pub(crate) visibility: Visibility,
    pub(crate) view_box: ViewBox,
    pub(crate) rendering_mode: ImageRendering,
    pub(crate) kind: ImageKind,
    pub(crate) abs_transform: Transform,
    pub(crate) bounding_box: Option<NonZeroRect>,
}

impl Image {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Isn't automatically generated.
    /// Can be empty.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Element visibility.
    pub fn visibility(&self) -> Visibility {
        self.visibility
    }

    /// An image rectangle in which it should be fit.
    ///
    /// Combination of the `x`, `y`, `width`, `height` and `preserveAspectRatio`
    /// attributes.
    pub fn view_box(&self) -> ViewBox {
        self.view_box
    }

    /// Rendering mode.
    ///
    /// `image-rendering` in SVG.
    pub fn rendering_mode(&self) -> ImageRendering {
        self.rendering_mode
    }

    /// Image data.
    pub fn kind(&self) -> &ImageKind {
        &self.kind
    }

    /// Element's absolute transform.
    ///
    /// Contains all ancestors transforms.
    ///
    /// Will be set after calling `usvg::Tree::postprocess`.
    ///
    /// Note that this is not the relative transform present in SVG.
    /// The SVG one would be set only on groups.
    pub fn abs_transform(&self) -> Transform {
        self.abs_transform
    }

    /// Element's object bounding box.
    ///
    /// `objectBoundingBox` in SVG terms. Meaning it doesn't affected by parent transforms.
    ///
    /// Will be set after calling `usvg::Tree::postprocess`.
    pub fn bounding_box(&self) -> Option<NonZeroRect> {
        self.bounding_box
    }

    fn abs_bounding_box(&self) -> Option<NonZeroRect> {
        self.bounding_box
            .and_then(|r| r.transform(self.abs_transform))
    }

    fn subroots(&self, f: &mut dyn FnMut(&Group)) {
        if let ImageKind::SVG(ref tree) = self.kind {
            f(&tree.root)
        }
    }

    fn subroots_mut(&mut self, f: &mut dyn FnMut(&mut Group)) {
        if let ImageKind::SVG(ref mut tree) = self.kind {
            f(&mut tree.root)
        }
    }
}

/// A nodes tree container.
#[allow(missing_debug_implementations)]
#[derive(Clone, Debug)]
pub struct Tree {
    pub(crate) size: Size,
    pub(crate) view_box: ViewBox,
    pub(crate) root: Group,
}

impl Tree {
    /// Image size.
    ///
    /// Size of an image that should be created to fit the SVG.
    ///
    /// `width` and `height` in SVG.
    pub fn size(&self) -> Size {
        self.size
    }

    /// SVG viewbox.
    ///
    /// Specifies which part of the SVG image should be rendered.
    ///
    /// `viewBox` and `preserveAspectRatio` in SVG.
    pub fn view_box(&self) -> ViewBox {
        self.view_box
    }

    /// The root element of the SVG tree.
    pub fn root(&self) -> &Group {
        &self.root
    }

    /// Returns a renderable node by ID.
    ///
    /// If an empty ID is provided, than this method will always return `None`.
    pub fn node_by_id(&self, id: &str) -> Option<&Node> {
        if id.is_empty() {
            return None;
        }

        node_by_id(&self.root, id)
    }

    /// Checks if the current tree has any text nodes.
    pub fn has_text_nodes(&self) -> bool {
        has_text_nodes(&self.root)
    }

    /// Calls a closure for each [`Paint`] in the tree.
    ///
    /// Doesn't guarantee to have unique paint servers. A caller must deduplicate them manually.
    pub fn paint_servers<F: FnMut(&Paint)>(&self, mut f: F) {
        loop_over_paint_servers(&self.root, &mut f)
    }

    /// Calls a closure for each [`ClipPath`] in the tree.
    ///
    /// Doesn't guarantee to have unique clip paths. A caller must deduplicate them manually.
    pub fn clip_paths<F: FnMut(SharedClipPath)>(&self, mut f: F) {
        loop_over_clip_paths(&self.root, &mut f)
    }

    /// Calls a closure for each [`Mask`] in the tree.
    ///
    /// Doesn't guarantee to have unique masks. A caller must deduplicate them manually.
    pub fn masks<F: FnMut(SharedMask)>(&self, mut f: F) {
        loop_over_masks(&self.root, &mut f)
    }

    /// Calls a closure for each [`Filter`](filter::Filter) in the tree.
    ///
    /// Doesn't guarantee to have unique filters. A caller must deduplicate them manually.
    pub fn filters<F: FnMut(filter::SharedFilter)>(&self, mut f: F) {
        loop_over_filters(&self.root, &mut f)
    }

    /// Calculates absolute transforms for all nodes in the tree.
    ///
    /// A low-level method. Prefer `usvg::Tree::postprocess` instead.
    pub(crate) fn calculate_abs_transforms(&mut self) {
        self.root.calculate_abs_transforms(Transform::identity());
    }

    /// Calculates bounding boxes for all nodes in the tree.
    ///
    /// A low-level method. Prefer `usvg::Tree::postprocess` instead.
    pub(crate) fn calculate_bounding_boxes(&mut self) {
        self.root.calculate_bounding_boxes();
    }

    /// Postprocesses the `usvg::Tree`.
    ///
    /// Must be called after parsing a `usvg::Tree`.
    ///
    /// `steps` contains a list of _additional_ post-processing steps.
    /// This methods performs some operations even when `steps` is `PostProcessingSteps::default()`.
    ///
    /// `fontdb` is needed only for [`PostProcessingSteps::convert_text_into_paths`].
    /// Otherwise you can pass just `fontdb::Database::new()`.
    pub fn postprocess(
        &mut self,
        steps: PostProcessingSteps,
        #[cfg(feature = "text")] fontdb: &fontdb::Database,
    ) {
        self.calculate_abs_transforms();

        if steps.convert_text_into_paths {
            #[cfg(feature = "text")]
            {
                crate::text_to_paths::convert_text(&mut self.root, &fontdb);
            }
        }

        self.calculate_bounding_boxes();
    }
}

fn node_by_id<'a>(parent: &'a Group, id: &str) -> Option<&'a Node> {
    for child in &parent.children {
        if child.id() == id {
            return Some(child);
        }

        if let Node::Group(ref g) = child {
            if let Some(n) = node_by_id(g, id) {
                return Some(n);
            }
        }
    }

    None
}

fn has_text_nodes(root: &Group) -> bool {
    for node in &root.children {
        if let Node::Text(_) = node {
            return true;
        }

        let mut has_text = false;

        if let Node::Image(ref image) = node {
            if let ImageKind::SVG(ref tree) = image.kind {
                if has_text_nodes(&tree.root) {
                    has_text = true;
                }
            }
        }

        node.subroots(|subroot| has_text |= has_text_nodes(subroot));

        if has_text {
            return true;
        }
    }

    true
}

fn loop_over_paint_servers(parent: &Group, f: &mut dyn FnMut(&Paint)) {
    fn push(paint: Option<&Paint>, f: &mut dyn FnMut(&Paint)) {
        if let Some(paint) = paint {
            f(paint);
        }
    }

    for node in &parent.children {
        match node {
            Node::Group(ref group) => loop_over_paint_servers(group, f),
            Node::Path(ref path) => {
                push(path.fill.as_ref().map(|f| &f.paint), f);
                push(path.stroke.as_ref().map(|f| &f.paint), f);
            }
            Node::Image(_) => {}
            Node::Text(ref text) => {
                // A flattened text should be ignored, otherwise we would have duplicates.
                if text.flattened.is_none() {
                    for chunk in &text.chunks {
                        for span in &chunk.spans {
                            push(span.fill.as_ref().map(|f| &f.paint), f);
                            push(span.stroke.as_ref().map(|f| &f.paint), f);

                            if let Some(ref underline) = span.decoration.underline {
                                push(underline.fill.as_ref().map(|f| &f.paint), f);
                                push(underline.stroke.as_ref().map(|f| &f.paint), f);
                            }

                            if let Some(ref overline) = span.decoration.overline {
                                push(overline.fill.as_ref().map(|f| &f.paint), f);
                                push(overline.stroke.as_ref().map(|f| &f.paint), f);
                            }

                            if let Some(ref line_through) = span.decoration.line_through {
                                push(line_through.fill.as_ref().map(|f| &f.paint), f);
                                push(line_through.stroke.as_ref().map(|f| &f.paint), f);
                            }
                        }
                    }
                }
            }
        }

        node.subroots(|subroot| loop_over_paint_servers(subroot, f));
    }
}

fn loop_over_clip_paths(parent: &Group, f: &mut dyn FnMut(SharedClipPath)) {
    for node in &parent.children {
        if let Node::Group(ref g) = node {
            if let Some(ref clip) = g.clip_path {
                f(clip.clone());

                if let Some(ref sub_clip) = clip.borrow().clip_path {
                    f(sub_clip.clone());
                }
            }
        }

        node.subroots(|subroot| loop_over_clip_paths(subroot, f));

        if let Node::Group(ref g) = node {
            loop_over_clip_paths(g, f);
        }
    }
}

fn loop_over_masks(parent: &Group, f: &mut dyn FnMut(SharedMask)) {
    for node in &parent.children {
        if let Node::Group(ref g) = node {
            if let Some(ref mask) = g.mask {
                f(mask.clone());

                if let Some(ref sub_mask) = mask.borrow().mask {
                    f(sub_mask.clone());
                }
            }

            loop_over_masks(g, f);
        }

        node.subroots(|subroot| loop_over_masks(subroot, f));

        if let Node::Group(ref g) = node {
            loop_over_masks(g, f);
        }
    }
}

fn loop_over_filters(parent: &Group, f: &mut dyn FnMut(filter::SharedFilter)) {
    for node in &parent.children {
        if let Node::Group(ref g) = node {
            for filter in &g.filters {
                f(filter.clone());
            }
        }

        node.subroots(|subroot| loop_over_filters(subroot, f));

        if let Node::Group(ref g) = node {
            loop_over_filters(g, f);
        }
    }
}

impl Group {
    /// Calculates absolute transforms for all children of this group.
    ///
    /// A low-level method. Prefer `usvg::Tree::postprocess` instead.
    pub(crate) fn calculate_abs_transforms(&mut self, transform: Transform) {
        for node in &mut self.children {
            match node {
                Node::Group(ref mut group) => {
                    let abs_ts = transform.pre_concat(group.transform);
                    group.abs_transform = abs_ts;
                    group.calculate_abs_transforms(abs_ts);
                }
                Node::Path(ref mut path) => path.abs_transform = transform,
                Node::Image(ref mut image) => image.abs_transform = transform,
                Node::Text(ref mut text) => text.abs_transform = transform,
            }

            // Yes, subroots are not affected by the node's transform.
            node.subroots_mut(|root| root.calculate_abs_transforms(Transform::identity()));
        }
    }

    /// Calculates bounding boxes for all children of this group.
    ///
    /// A low-level method. Prefer `usvg::Tree::postprocess` instead.
    pub(crate) fn calculate_bounding_boxes(&mut self) {
        for node in &mut self.children {
            match node {
                Node::Path(ref mut path) => {
                    path.bounding_box = path.data.compute_tight_bounds();
                    path.stroke_bounding_box = path.calculate_stroke_bounding_box();

                    if path.abs_transform.has_skew() {
                        // TODO: avoid re-alloc
                        let path2 = path.data.as_ref().clone();
                        if let Some(path2) = path2.transform(path.abs_transform) {
                            path.abs_bounding_box = path2.compute_tight_bounds();
                            path.abs_stroke_bounding_box =
                                Path::calculate_stroke_bounding_box_inner(
                                    path.stroke.as_ref(),
                                    &path2,
                                );
                        }
                    } else {
                        // A transform without a skew can be performed just on a bbox.
                        path.abs_bounding_box = path
                            .bounding_box
                            .and_then(|r| r.transform(path.abs_transform));

                        path.abs_stroke_bounding_box = path
                            .stroke_bounding_box
                            .and_then(|r| r.transform(path.abs_transform));
                    }

                    if path.stroke_bounding_box.is_none() {
                        path.stroke_bounding_box =
                            path.bounding_box.and_then(|r| r.to_non_zero_rect());
                    }
                }
                // TODO: should we account for `preserveAspectRatio`?
                Node::Image(ref mut image) => image.bounding_box = Some(image.view_box.rect),
                // Have to be handled separately to prevent multiple mutable reference to the tree.
                Node::Group(ref mut group) => {
                    group.calculate_bounding_boxes();
                }
                // Will be set only during text-to-path conversion.
                Node::Text(_) => {}
            }

            // Yes, subroots are not affected by the node's transform.
            node.subroots_mut(|root| root.calculate_bounding_boxes());
        }

        let mut bbox = BBox::default();
        let mut abs_bbox = BBox::default();
        let mut stroke_bbox = BBox::default();
        let mut abs_stroke_bbox = BBox::default();
        let mut layer_bbox = BBox::default();
        for child in &self.children {
            if let Some(mut c_bbox) = child.bounding_box() {
                if let Node::Group(ref group) = child {
                    if let Some(r) = c_bbox.transform(group.transform) {
                        c_bbox = r;
                    }
                }

                bbox = bbox.expand(c_bbox);
            }

            if let Some(c_bbox) = child.abs_bounding_box() {
                abs_bbox = abs_bbox.expand(c_bbox);
            }

            if let Some(mut c_bbox) = child.stroke_bounding_box() {
                if let Node::Group(ref group) = child {
                    if let Some(r) = c_bbox.transform(group.transform) {
                        c_bbox = r;
                    }
                }

                stroke_bbox = stroke_bbox.expand(c_bbox);
            }

            if let Some(c_bbox) = child.abs_stroke_bounding_box() {
                abs_stroke_bbox = abs_stroke_bbox.expand(c_bbox);
            }

            if let Node::Group(ref group) = child {
                if let Some(r) = group.layer_bounding_box {
                    if let Some(r) = r.transform(group.transform) {
                        layer_bbox = layer_bbox.expand(r);
                    }
                }
            } else if let Some(c_bbox) = child.stroke_bounding_box() {
                // Not a group - no need to transform.
                layer_bbox = layer_bbox.expand(c_bbox);
            }
        }

        self.bounding_box = bbox.to_rect();
        self.abs_bounding_box = abs_bbox.to_rect();
        self.stroke_bounding_box = stroke_bbox.to_non_zero_rect();
        self.abs_stroke_bounding_box = abs_stroke_bbox.to_non_zero_rect();

        // Filter bbox has a higher priority than layers bbox.
        if let Some(filter_bbox) = self.filters_bounding_box() {
            self.layer_bounding_box = Some(filter_bbox);
        } else {
            self.layer_bounding_box = layer_bbox.to_non_zero_rect();
        }
    }
}
