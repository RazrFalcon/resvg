// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/*!
`usvg` (micro SVG) is an [SVG] parser that tries to solve most of SVG complexity.

SVG is notoriously hard to parse. `usvg` presents a layer between an XML library and
a potential SVG rendering library. It will parse an input SVG into a strongly-typed tree structure
were all the elements, attributes, references and other SVG features are already resolved
and presented in a simplest way possible.
So a caller doesn't have to worry about most of the issues related to SVG parsing
and can focus just on the rendering part.

## Features

- All supported attributes are resolved.
  No need to worry about inheritable, implicit and default attributes
- CSS will be applied
- Only simple paths
  - Basic shapes (like `rect` and `circle`) will be converted into paths
  - Paths contain only absolute *MoveTo*, *LineTo*, *CurveTo* and *ClosePath* segments.
    ArcTo, implicit and relative segments will be converted
- `use` will be resolved and replaced with the reference content
- Nested `svg` will be resolved
- Invalid, malformed elements will be removed
- Relative length units (mm, em, etc.) will be converted into pixels/points
- External images will be loaded
- Internal, base64 images will be decoded
- Dummy groups will be removed
- All references (like `#elem` and `url(#elem)`) will be resolved
- `switch` will be resolved
- Text elements, which are probably the hardest part of SVG, will be completely resolved.
  This includes all the attributes resolving, whitespaces preprocessing (`xml:space`),
  text chunks and spans resolving
- Markers will be converted into regular elements. No need to place them manually
- All filters are supported. Including filter functions, like `filter="contrast(50%)"`
- Recursive elements will be detected an removed

## Limitations

- Unsupported SVG features will be ignored
- CSS support is minimal
- Only [static](http://www.w3.org/TR/SVG11/feature#SVG-static) SVG features,
  e.g. no `a`, `view`, `cursor`, `script`, no events and no animations

[SVG]: https://en.wikipedia.org/wiki/Scalable_Vector_Graphics
*/

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![warn(missing_copy_implementations)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::neg_cmp_op_on_partial_ord)]
#![allow(clippy::identity_op)]
#![allow(clippy::question_mark)]
#![allow(clippy::upper_case_acronyms)]

macro_rules! impl_enum_default {
    ($name:ident, $def_value:ident) => {
        impl Default for $name {
            #[inline]
            fn default() -> Self {
                $name::$def_value
            }
        }
    };
}

macro_rules! impl_enum_from_str {
    ($name:ident, $($string:pat => $result:expr),+) => {
        impl crate::svgtree::EnumFromStr for $name {
            fn enum_from_str(s: &str) -> Option<Self> {
                match s {
                    $($string => Some($result)),+,
                    _ => None,
                }
            }
        }
    };
}

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

mod clippath;
mod converter;
mod error;
pub mod filter;
mod geom;
mod image;
mod marker;
mod mask;
mod options;
mod paint_server;
mod pathdata;
mod shapes;
mod style;
mod svgtree;
mod switch;
mod text;
mod units;
mod use_node;
pub mod utils;

pub use image::ImageHrefResolver;
pub use strict_num::{ApproxEq, ApproxEqUlps, NonZeroPositiveF64, NormalizedF64, PositiveF64};
pub use svgtypes::{Align, AspectRatio};

use std::rc::Rc;

pub use roxmltree;

pub use crate::clippath::*;
pub use crate::error::*;
pub use crate::geom::*;
pub use crate::image::*;
pub use crate::mask::*;
pub use crate::options::*;
pub use crate::paint_server::*;
pub use crate::pathdata::*;
pub use crate::style::*;
pub use crate::text::*;

trait OptionLog {
    fn log_none<F: FnOnce()>(self, f: F) -> Self;
}

impl<T> OptionLog for Option<T> {
    #[inline]
    fn log_none<F: FnOnce()>(self, f: F) -> Self {
        self.or_else(|| {
            f();
            None
        })
    }
}

/// Checks that type has a default value.
pub trait IsDefault: Default {
    /// Checks that type has a default value.
    fn is_default(&self) -> bool;
}

impl<T: Default + PartialEq + Copy> IsDefault for T {
    #[inline]
    fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

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

impl_enum_from_str!(Units,
    "userSpaceOnUse"    => Units::UserSpaceOnUse,
    "objectBoundingBox" => Units::ObjectBoundingBox
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

impl_enum_default!(BlendMode, Normal);

impl_enum_from_str!(BlendMode,
    "normal" => BlendMode::Normal,
    "multiply" => BlendMode::Multiply,
    "screen" => BlendMode::Screen,
    "overlay" => BlendMode::Overlay,
    "darken" => BlendMode::Darken,
    "lighten" => BlendMode::Lighten,
    "color-dodge" => BlendMode::ColorDodge,
    "color-burn" => BlendMode::ColorBurn,
    "hard-light" => BlendMode::HardLight,
    "soft-light" => BlendMode::SoftLight,
    "difference" => BlendMode::Difference,
    "exclusion" => BlendMode::Exclusion,
    "hue" => BlendMode::Hue,
    "saturation" => BlendMode::Saturation,
    "color" => BlendMode::Color,
    "luminosity" => BlendMode::Luminosity
);

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
    /// Parses `Tree` from an SVG data.
    ///
    /// Can contain an SVG string or a gzip compressed data.
    pub fn from_data(data: &[u8], opt: &Options) -> Result<Self, Error> {
        if data.starts_with(&[0x1f, 0x8b]) {
            let data = decompress_svgz(data)?;
            let text = std::str::from_utf8(&data).map_err(|_| Error::NotAnUtf8Str)?;
            Self::from_str(text, opt)
        } else {
            let text = std::str::from_utf8(data).map_err(|_| Error::NotAnUtf8Str)?;
            Self::from_str(text, opt)
        }
    }

    /// Parses `Tree` from an SVG string.
    pub fn from_str(text: &str, opt: &Options) -> Result<Self, Error> {
        let mut xml_opt = roxmltree::ParsingOptions::default();
        xml_opt.allow_dtd = true;

        let doc =
            roxmltree::Document::parse_with_options(text, xml_opt).map_err(Error::ParsingFailed)?;

        Self::from_xmltree(&doc, opt)
    }

    /// Parses `Tree` from `roxmltree::Document`.
    pub fn from_xmltree(doc: &roxmltree::Document, opt: &Options) -> Result<Self, Error> {
        let doc = svgtree::Document::parse(doc)?;
        Self::from_svgtree(doc, opt)
    }

    /// Parses `Tree` from the `svgtree::Document`.
    ///
    /// An empty `Tree` will be returned on any error.
    fn from_svgtree(doc: svgtree::Document, opt: &Options) -> Result<Self, Error> {
        crate::converter::convert_doc(&doc, opt)
    }

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

    /// Ungroups groups inside the `root` node.
    pub fn ungroup_groups(root: Node, keep_named_groups: bool) {
        converter::ungroup_groups(root, keep_named_groups);
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
        self.ancestors()
            .skip(1)
            .find(|node| has_enable_background(node))
    }
}

/// Decompresses an SVGZ file.
pub fn decompress_svgz(data: &[u8]) -> Result<Vec<u8>, Error> {
    use std::io::Read;

    let mut decoder = flate2::read::GzDecoder::new(data);
    let mut decoded = Vec::with_capacity(data.len() * 2);
    decoder
        .read_to_end(&mut decoded)
        .map_err(|_| Error::MalformedGZip)?;
    Ok(decoded)
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
                let mut child_transform = ts.clone();
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
