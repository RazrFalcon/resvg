// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/*!
`usvg-tree` is an [SVG] tree representation used by [usvg].

[SVG]: https://en.wikipedia.org/wiki/Scalable_Vector_Graphics
[usvg]: https://github.com/RazrFalcon/resvg/tree/master/usvg
*/

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![warn(missing_copy_implementations)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::neg_cmp_op_on_partial_ord)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::derivable_impls)]

pub mod filter;
mod geom;
mod pathdata;
mod text;
pub mod utils;

use std::rc::Rc;
use std::sync::Arc;

pub use strict_num::{ApproxEq, ApproxEqUlps, NonZeroPositiveF64, NormalizedF64, PositiveF64};
pub use svgtypes::{Align, AspectRatio};

pub use crate::geom::*;
pub use crate::pathdata::*;
pub use crate::text::*;

/// An alias to `NormalizedF64`.
pub type Opacity = NormalizedF64;

/// A non-zero `f64`.
///
/// Just like `f64` but immutable and guarantee to never be zero.
#[derive(Clone, Copy, Debug)]
pub struct NonZeroF64(f64);

impl NonZeroF64 {
    /// Creates a new `NonZeroF64` value.
    #[inline]
    pub fn new(n: f64) -> Option<Self> {
        if n.is_fuzzy_zero() {
            None
        } else {
            Some(NonZeroF64(n))
        }
    }

    /// Returns an underlying value.
    #[inline]
    pub fn value(&self) -> f64 {
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
    /// Coordinate system units.
    ///
    /// `gradientUnits` in SVG.
    pub units: Units,

    /// Gradient transform.
    ///
    /// `gradientTransform` in SVG.
    pub transform: Transform,

    /// Gradient spreading method.
    ///
    /// `spreadMethod` in SVG.
    pub spread_method: SpreadMethod,

    /// A list of `stop` elements.
    pub stops: Vec<Stop>,
}

/// A linear gradient.
///
/// `linearGradient` element in SVG.
#[allow(missing_docs)]
#[derive(Clone, Debug)]
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
    pub base: BaseGradient,
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
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct RadialGradient {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Can't be empty.
    pub id: String,

    pub cx: f64,
    pub cy: f64,
    pub r: PositiveF64,
    pub fx: f64,
    pub fy: f64,

    /// Base gradient data.
    pub base: BaseGradient,
}

impl std::ops::Deref for RadialGradient {
    type Target = BaseGradient;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

/// An alias to `NormalizedF64`.
pub type StopOffset = NormalizedF64;

/// Gradient's stop element.
///
/// `stop` element in SVG.
#[derive(Clone, Copy, Debug)]
pub struct Stop {
    /// Gradient stop offset.
    ///
    /// `offset` in SVG.
    pub offset: StopOffset,

    /// Gradient stop color.
    ///
    /// `stop-color` in SVG.
    pub color: Color,

    /// Gradient stop opacity.
    ///
    /// `stop-opacity` in SVG.
    pub opacity: Opacity,
}

/// A pattern element.
///
/// `pattern` element in SVG.
#[derive(Clone, Debug)]
pub struct Pattern {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Can't be empty.
    pub id: String,

    /// Coordinate system units.
    ///
    /// `patternUnits` in SVG.
    pub units: Units,

    // TODO: should not be accessible when `viewBox` is present.
    /// Content coordinate system units.
    ///
    /// `patternContentUnits` in SVG.
    pub content_units: Units,

    /// Pattern transform.
    ///
    /// `patternTransform` in SVG.
    pub transform: Transform,

    /// Pattern rectangle.
    ///
    /// `x`, `y`, `width` and `height` in SVG.
    pub rect: Rect,

    /// Pattern viewbox.
    pub view_box: Option<ViewBox>,

    /// Pattern children.
    ///
    /// The root node is always `Group`.
    pub root: Node,
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

impl From<f64> for StrokeMiterlimit {
    #[inline]
    fn from(n: f64) -> Self {
        Self::new(n)
    }
}

impl PartialEq for StrokeMiterlimit {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0.fuzzy_eq(&other.0)
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
    Round,
    Bevel,
}

impl Default for LineJoin {
    fn default() -> Self {
        Self::Miter
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

impl Default for FillRule {
    fn default() -> Self {
        Self::NonZero
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

/// A clip-path element.
///
/// `clipPath` element in SVG.
#[derive(Clone, Debug)]
pub struct ClipPath {
    /// Element's ID.
    ///
    /// Taken from the SVG itself or generated by the parser.
    /// Used only during SVG writing. `resvg` doesn't rely on this property.
    pub id: String,

    /// Coordinate system units.
    ///
    /// `clipPathUnits` in SVG.
    pub units: Units,

    /// Clip path transform.
    ///
    /// `transform` in SVG.
    pub transform: Transform,

    /// Additional clip path.
    ///
    /// `clip-path` in SVG.
    pub clip_path: Option<Rc<Self>>,

    /// Clip path children.
    ///
    /// The root node is always `Group`.
    pub root: Node,
}

impl Default for ClipPath {
    fn default() -> Self {
        ClipPath {
            id: String::new(),
            units: Units::UserSpaceOnUse,
            transform: Transform::default(),
            clip_path: None,
            root: Node::new(NodeKind::Group(Group::default())),
        }
    }
}

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
    /// Element's ID.
    ///
    /// Taken from the SVG itself or generated by the parser.
    /// Used only during SVG writing. `resvg` doesn't rely on this property.
    pub id: String,

    /// Coordinate system units.
    ///
    /// `maskUnits` in SVG.
    pub units: Units,

    /// Content coordinate system units.
    ///
    /// `maskContentUnits` in SVG.
    pub content_units: Units,

    /// Mask rectangle.
    ///
    /// `x`, `y`, `width` and `height` in SVG.
    pub rect: Rect,

    /// Mask type.
    ///
    /// `mask-type` in SVG.
    pub kind: MaskType,

    /// Additional mask.
    ///
    /// `mask` in SVG.
    pub mask: Option<Rc<Self>>,

    /// Clip path children.
    ///
    /// The root node is always `Group`.
    pub root: Node,
}

/// Node's kind.
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub enum NodeKind {
    Group(Group),
    Path(Path),
    Image(Image),
    Text(Text),
}

impl NodeKind {
    /// Returns node's ID.
    pub fn id(&self) -> &str {
        match self {
            NodeKind::Group(ref e) => e.id.as_str(),
            NodeKind::Path(ref e) => e.id.as_str(),
            NodeKind::Image(ref e) => e.id.as_str(),
            NodeKind::Text(ref e) => e.id.as_str(),
        }
    }

    /// Returns node's transform.
    pub fn transform(&self) -> Transform {
        match self {
            NodeKind::Group(ref e) => e.transform,
            NodeKind::Path(ref e) => e.transform,
            NodeKind::Image(ref e) => e.transform,
            NodeKind::Text(ref e) => e.transform,
        }
    }
}

/// An `enable-background`.
///
/// Contains only the `new [ <x> <y> <width> <height> ]` value.
#[derive(Clone, Copy, Debug)]
#[allow(missing_docs)]
pub struct EnableBackground(pub Option<Rect>);

/// A group container.
///
/// The preprocessor will remove all groups that don't impact rendering.
/// Those that left is just an indicator that a new canvas should be created.
///
/// `g` element in SVG.
#[derive(Clone, Debug)]
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
    pub opacity: Opacity,

    /// Group blend mode.
    ///
    /// `mix-blend-mode` in SVG.
    pub blend_mode: BlendMode,

    /// Group isolation.
    ///
    /// `isolation` in SVG.
    pub isolate: bool,

    /// Element's clip path.
    pub clip_path: Option<Rc<ClipPath>>,

    /// Element's mask.
    pub mask: Option<Rc<Mask>>,

    /// Element's filters.
    pub filters: Vec<Rc<filter::Filter>>,

    /// Contains a fill color or paint server used by `FilterInput::FillPaint`.
    ///
    /// Will be set only when filter actually has a `FilterInput::FillPaint`.
    pub filter_fill: Option<Paint>,

    /// Contains a fill color or paint server used by `FilterInput::StrokePaint`.
    ///
    /// Will be set only when filter actually has a `FilterInput::StrokePaint`.
    pub filter_stroke: Option<Paint>,

    /// Indicates that this node can be accessed via `filter`.
    ///
    /// `None` indicates an `accumulate` value.
    pub enable_background: Option<EnableBackground>,
}

impl Default for Group {
    fn default() -> Self {
        Group {
            id: String::new(),
            transform: Transform::default(),
            opacity: Opacity::ONE,
            blend_mode: BlendMode::Normal,
            isolate: false,
            clip_path: None,
            mask: None,
            filters: Vec::new(),
            filter_fill: None,
            filter_stroke: None,
            enable_background: None,
        }
    }
}

impl Group {
    /// Checks if this group should be isolated during rendering.
    pub fn should_isolate(&self) -> bool {
        self.isolate
            || self.opacity != Opacity::ONE
            || self.clip_path.is_some()
            || self.mask.is_some()
            || !self.filters.is_empty()
            || self.blend_mode != BlendMode::Normal // TODO: probably not needed?
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
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Isn't automatically generated.
    /// Can be empty.
    pub id: String,

    /// Element transform.
    pub transform: Transform,

    /// Element visibility.
    pub visibility: Visibility,

    /// Fill style.
    pub fill: Option<Fill>,

    /// Stroke style.
    pub stroke: Option<Stroke>,

    /// Fill and stroke paint order.
    ///
    /// Since markers will be replaced with regular nodes automatically,
    /// `usvg` doesn't provide the `markers` order type. It's was already done.
    ///
    /// `paint-order` in SVG.
    pub paint_order: PaintOrder,

    /// Rendering mode.
    ///
    /// `shape-rendering` in SVG.
    pub rendering_mode: ShapeRendering,

    /// Contains a text bbox.
    ///
    /// Text bbox is different from path bbox. The later one contains a tight path bbox,
    /// while the text bbox is based on the actual font metrics and usually larger than tight bbox.
    ///
    /// Also, path bbox doesn't include leading and trailing whitespaces,
    /// because there is nothing to include. But text bbox does.
    ///
    /// As the name suggests, this property will be set only for paths
    /// that were converted from text.
    pub text_bbox: Option<Rect>,

    /// Segments list.
    ///
    /// All segments are in absolute coordinates.
    pub data: Rc<PathData>,
}

impl Default for Path {
    fn default() -> Self {
        Path {
            id: String::new(),
            transform: Transform::default(),
            visibility: Visibility::Visible,
            fill: None,
            stroke: None,
            paint_order: PaintOrder::default(),
            rendering_mode: ShapeRendering::default(),
            text_bbox: None,
            data: Rc::new(PathData::default()),
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
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Isn't automatically generated.
    /// Can be empty.
    pub id: String,

    /// Element transform.
    pub transform: Transform,

    /// Element visibility.
    pub visibility: Visibility,

    /// An image rectangle in which it should be fit.
    ///
    /// Combination of the `x`, `y`, `width`, `height` and `preserveAspectRatio`
    /// attributes.
    pub view_box: ViewBox,

    /// Rendering mode.
    ///
    /// `image-rendering` in SVG.
    pub rendering_mode: ImageRendering,

    /// Image data.
    pub kind: ImageKind,
}

/// Alias for `rctree::Node<NodeKind>`.
pub type Node = rctree::Node<NodeKind>;

// TODO: impl a Debug
/// A nodes tree container.
#[allow(missing_debug_implementations)]
#[derive(Clone)]
pub struct Tree {
    /// Image size.
    ///
    /// Size of an image that should be created to fit the SVG.
    ///
    /// `width` and `height` in SVG.
    pub size: Size,

    /// SVG viewbox.
    ///
    /// Specifies which part of the SVG image should be rendered.
    ///
    /// `viewBox` and `preserveAspectRatio` in SVG.
    pub view_box: ViewBox,

    /// The root element of the SVG tree.
    ///
    /// The root node is always `Group`.
    pub root: Node,
}

impl Tree {
    // TODO: remove
    /// Returns renderable node by ID.
    ///
    /// If an empty ID is provided, than this method will always return `None`.
    /// Even if tree has nodes with empty ID.
    pub fn node_by_id(&self, id: &str) -> Option<Node> {
        if id.is_empty() {
            return None;
        }

        self.root.descendants().find(|node| &*node.id() == id)
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
    pub fn clip_paths<F: FnMut(Rc<ClipPath>)>(&self, mut f: F) {
        loop_over_clip_paths(&self.root, &mut f)
    }

    /// Calls a closure for each [`Mask`] in the tree.
    ///
    /// Doesn't guarantee to have unique masks. A caller must deduplicate them manually.
    pub fn masks<F: FnMut(Rc<Mask>)>(&self, mut f: F) {
        loop_over_masks(&self.root, &mut f)
    }

    /// Calls a closure for each [`Filter`](filter::Filter) in the tree.
    ///
    /// Doesn't guarantee to have unique filters. A caller must deduplicate them manually.
    pub fn filters<F: FnMut(Rc<filter::Filter>)>(&self, mut f: F) {
        loop_over_filters(&self.root, &mut f)
    }
}

fn has_text_nodes(root: &Node) -> bool {
    for node in root.descendants() {
        if let NodeKind::Text(_) = *node.borrow() {
            return true;
        }

        let mut has_text = false;
        node.subroots(|subroot| {
            if has_text_nodes(&subroot) {
                has_text = true;
            }
        });

        if has_text {
            return true;
        }
    }

    false
}

fn loop_over_paint_servers(root: &Node, f: &mut dyn FnMut(&Paint)) {
    fn push(paint: Option<&Paint>, f: &mut dyn FnMut(&Paint)) {
        if let Some(paint) = paint {
            f(paint);
        }
    }

    for node in root.descendants() {
        if let NodeKind::Group(ref group) = *node.borrow() {
            push(group.filter_fill.as_ref(), f);
            push(group.filter_stroke.as_ref(), f);
        } else if let NodeKind::Path(ref path) = *node.borrow() {
            push(path.fill.as_ref().map(|f| &f.paint), f);
            push(path.stroke.as_ref().map(|f| &f.paint), f);
        } else if let NodeKind::Text(ref text) = *node.borrow() {
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

        node.subroots(|subroot| loop_over_paint_servers(&subroot, f));
    }
}

fn loop_over_clip_paths(root: &Node, f: &mut dyn FnMut(Rc<ClipPath>)) {
    for node in root.descendants() {
        if let NodeKind::Group(ref g) = *node.borrow() {
            if let Some(ref clip) = g.clip_path {
                f(clip.clone());

                if let Some(ref sub_clip) = clip.clip_path {
                    f(sub_clip.clone());
                }
            }
        }

        node.subroots(|subroot| loop_over_clip_paths(&subroot, f));
    }
}

fn loop_over_masks(root: &Node, f: &mut dyn FnMut(Rc<Mask>)) {
    for node in root.descendants() {
        if let NodeKind::Group(ref g) = *node.borrow() {
            if let Some(ref mask) = g.mask {
                f(mask.clone());

                if let Some(ref sub_mask) = mask.mask {
                    f(sub_mask.clone());
                }
            }
        }

        node.subroots(|subroot| loop_over_masks(&subroot, f));
    }
}

fn loop_over_filters(root: &Node, f: &mut dyn FnMut(Rc<filter::Filter>)) {
    for node in root.descendants() {
        if let NodeKind::Group(ref g) = *node.borrow() {
            for filter in &g.filters {
                f(filter.clone());
            }
        }

        node.subroots(|subroot| loop_over_filters(&subroot, f));
    }
}

fn node_subroots(node: &Node, f: &mut dyn FnMut(Node)) {
    let mut push_patt = |paint: Option<&Paint>| {
        if let Some(Paint::Pattern(ref patt)) = paint {
            f(patt.root.clone());
        }
    };

    match *node.borrow() {
        NodeKind::Group(ref g) => {
            push_patt(g.filter_fill.as_ref());
            push_patt(g.filter_stroke.as_ref());

            if let Some(ref clip) = g.clip_path {
                f(clip.root.clone());

                if let Some(ref sub_clip) = clip.clip_path {
                    f(sub_clip.root.clone());
                }
            }

            if let Some(ref mask) = g.mask {
                f(mask.root.clone());

                if let Some(ref sub_mask) = mask.mask {
                    f(sub_mask.root.clone());
                }
            }

            for filter in &g.filters {
                for primitive in &filter.primitives {
                    if let filter::Kind::Image(ref image) = primitive.kind {
                        if let filter::ImageKind::Use(ref use_node) = image.data {
                            f(use_node.clone());
                        }
                    }
                }
            }
        }
        NodeKind::Path(ref path) => {
            push_patt(path.fill.as_ref().map(|f| &f.paint));
            push_patt(path.stroke.as_ref().map(|f| &f.paint));
        }
        NodeKind::Image(_) => {}
        NodeKind::Text(ref text) => {
            for chunk in &text.chunks {
                for span in &chunk.spans {
                    push_patt(span.fill.as_ref().map(|f| &f.paint));
                    push_patt(span.stroke.as_ref().map(|f| &f.paint));

                    // Each text decoration can have paint.
                    if let Some(ref underline) = span.decoration.underline {
                        push_patt(underline.fill.as_ref().map(|f| &f.paint));
                        push_patt(underline.stroke.as_ref().map(|f| &f.paint));
                    }

                    if let Some(ref overline) = span.decoration.overline {
                        push_patt(overline.fill.as_ref().map(|f| &f.paint));
                        push_patt(overline.stroke.as_ref().map(|f| &f.paint));
                    }

                    if let Some(ref line_through) = span.decoration.line_through {
                        push_patt(line_through.fill.as_ref().map(|f| &f.paint));
                        push_patt(line_through.stroke.as_ref().map(|f| &f.paint));
                    }
                }
            }
        }
    }
}

/// Additional `Node` methods.
pub trait NodeExt {
    /// Returns node's ID.
    ///
    /// If a current node doesn't support ID - an empty string
    /// will be returned.
    fn id(&self) -> std::cell::Ref<str>;

    /// Returns node's transform.
    ///
    /// If a current node doesn't support transformation - a default
    /// transform will be returned.
    fn transform(&self) -> Transform;

    /// Returns node's absolute transform.
    ///
    /// If a current node doesn't support transformation - a default
    /// transform will be returned.
    fn abs_transform(&self) -> Transform;

    /// Appends `kind` as a node child.
    ///
    /// Shorthand for `Node::append(Node::new(Box::new(kind)))`.
    fn append_kind(&self, kind: NodeKind) -> Node;

    /// Calculates node's absolute bounding box.
    ///
    /// Can be expensive on large paths and groups.
    ///
    /// Always returns `None` for `NodeKind::Text` since we cannot calculate its bbox
    /// without converting it into paths first.
    fn calculate_bbox(&self) -> Option<PathBbox>;

    /// Returns the node starting from which the filter background should be rendered.
    fn filter_background_start_node(&self, filter: &filter::Filter) -> Option<Node>;

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
    /// use usvg_tree::NodeExt;
    ///
    /// fn all_nodes(root: &usvg_tree::Node) {
    ///     for node in root.descendants() {
    ///         // do stuff...
    ///
    ///         // hand subroots as well
    ///         node.subroots(|subroot| all_nodes(&subroot));
    ///     }
    /// }
    /// ```
    fn subroots<F: FnMut(Node)>(&self, f: F);
}

impl NodeExt for Node {
    #[inline]
    fn id(&self) -> std::cell::Ref<str> {
        std::cell::Ref::map(self.borrow(), |v| v.id())
    }

    #[inline]
    fn transform(&self) -> Transform {
        self.borrow().transform()
    }

    fn abs_transform(&self) -> Transform {
        let mut ts_list = Vec::new();
        for p in self.ancestors() {
            ts_list.push(p.transform());
        }

        let mut abs_ts = Transform::default();
        for ts in ts_list.iter().rev() {
            abs_ts.append(ts);
        }

        abs_ts
    }

    #[inline]
    fn append_kind(&self, kind: NodeKind) -> Node {
        let new_node = Node::new(kind);
        self.append(new_node.clone());
        new_node
    }

    #[inline]
    fn calculate_bbox(&self) -> Option<PathBbox> {
        calc_node_bbox(self, self.abs_transform())
    }

    fn filter_background_start_node(&self, filter: &filter::Filter) -> Option<Node> {
        fn has_enable_background(node: &Node) -> bool {
            if let NodeKind::Group(ref g) = *node.borrow() {
                g.enable_background.is_some()
            } else {
                false
            }
        }

        if !filter
            .primitives
            .iter()
            .any(|c| c.kind.has_input(&filter::Input::BackgroundImage))
            && !filter
                .primitives
                .iter()
                .any(|c| c.kind.has_input(&filter::Input::BackgroundAlpha))
        {
            return None;
        }

        // We should have an ancestor with `enable-background=new`.
        // Skip the current element.
        self.ancestors().skip(1).find(has_enable_background)
    }

    fn subroots<F: FnMut(Node)>(&self, mut f: F) {
        node_subroots(self, &mut f)
    }
}

fn calc_node_bbox(node: &Node, ts: Transform) -> Option<PathBbox> {
    match *node.borrow() {
        NodeKind::Path(ref path) => path.data.bbox_with_transform(ts, path.stroke.as_ref()),
        NodeKind::Image(ref img) => {
            let path = PathData::from_rect(img.view_box.rect);
            path.bbox_with_transform(ts, None)
        }
        NodeKind::Group(_) => {
            let mut bbox = PathBbox::new_bbox();

            for child in node.children() {
                let mut child_transform = ts;
                child_transform.append(&child.transform());
                if let Some(c_bbox) = calc_node_bbox(&child, child_transform) {
                    bbox = bbox.expand(c_bbox);
                }
            }

            // Make sure bbox was changed.
            if bbox.fuzzy_eq(&PathBbox::new_bbox()) {
                return None;
            }

            Some(bbox)
        }
        NodeKind::Text(_) => None,
    }
}
