// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/*!
`usvg` (micro SVG) is an [SVG] simplification tool.

## Purpose

Imagine, that you have to extract some data from the [SVG] file, but your
library/framework/language doesn't have a good SVG library.
And all you need is paths data.

You can try to export it by yourself (how hard can it be, right).
All you need is an XML library (I'll hope that your language has one).
But soon you realize that paths data has a pretty complex format and a lot
of edge-cases. And we didn't mention attributes propagation, transforms,
visibility flags, attribute values validation, XML quirks, etc.
It will take a lot of time and code to implement this stuff correctly.

So, instead of creating a library that can be used from any language (impossible),
*usvg* takes a different approach. It converts an input SVG to an extremely
simple representation, which is still a valid SVG.
And now, all you need is to convert your SVG to a simplified one via *usvg*
and an XML library with some small amount of code.

## Key features of the simplified SVG

- No basic shapes (rect, circle, etc). Only paths
- Simple paths:
  - Only MoveTo, LineTo, CurveTo and ClosePath will be produced
  - All path segments are in absolute coordinates
  - No implicit segment commands
  - All values are separated by space
- All (supported) attributes are resolved. No implicit one
- No `use`. Everything is resolved
- No invisible elements
- No invalid elements (like `rect` with negative/zero size)
- No units (mm, em, etc.)
- No comments
- No DTD
- No CSS (partial support)
- No `script` (simply ignoring it)

Full spec can be found [here](https://github.com/RazrFalcon/resvg/blob/master/docs/usvg_spec.adoc).

## Limitations

- Currently, it's not lossless. Some SVG features isn't supported yet and will be ignored.
- CSS support is minimal.
- Scripting and animation isn't supported and not planned.
- `a` elements will be removed.
- Unsupported elements:
  - some filter-based elements
  - font-based elements

[SVG]: https://en.wikipedia.org/wiki/Scalable_Vector_Graphics
*/

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

pub mod utils;
mod clippath;
mod converter;
mod error;
#[cfg(feature = "export")] mod export;
#[cfg(feature = "filter")] pub mod filter;
mod geom;
mod image;
mod marker;
mod mask;
mod numbers;
mod options;
mod paint_server;
mod pathdata;
mod shapes;
mod style;
mod svgtree;
mod switch;
#[cfg(feature = "text")] mod text;
mod units;
mod use_node;

pub use svgtypes::{Align, AspectRatio};

#[cfg(feature = "text")] pub use fontdb;

pub use crate::clippath::*;
pub use crate::error::*;
pub use crate::geom::*;
pub use crate::image::*;
pub use crate::mask::*;
pub use crate::numbers::*;
pub use crate::options::*;
pub use crate::paint_server::*;
pub use crate::pathdata::*;
pub use crate::style::*;


trait OptionLog {
    fn log_none<F: FnOnce()>(self, f: F) -> Self;
}

impl<T> OptionLog for Option<T> {
    #[inline]
    fn log_none<F: FnOnce()>(self, f: F) -> Self {
        self.or_else(|| { f(); None })
    }
}


/// XML writing options.
#[cfg(feature = "export")]
#[derive(Clone, Default, Debug)]
pub struct XmlOptions {
    /// Used to add a custom prefix to each element ID during writing.
    pub id_prefix: Option<String>,

    /// `xmlwriter` options.
    pub writer_opts: xmlwriter::Options,
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
            ShapeRendering::OptimizeSpeed         => false,
            ShapeRendering::CrispEdges            => false,
            ShapeRendering::GeometricPrecision    => true,
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


/// Node's kind.
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub enum NodeKind {
    Svg(Svg),
    Defs,
    LinearGradient(LinearGradient),
    RadialGradient(RadialGradient),
    ClipPath(ClipPath),
    Mask(Mask),
    Pattern(Pattern),
    #[cfg(feature = "filter")] Filter(filter::Filter),
    Path(Path),
    Image(Image),
    Group(Group),
}

impl NodeKind {
    /// Returns node's ID.
    ///
    /// If a current node doesn't support ID - an empty string
    /// will be returned.
    pub fn id(&self) -> &str {
        match *self {
            NodeKind::Svg(_) => "",
            NodeKind::Defs => "",
            NodeKind::LinearGradient(ref e) => e.id.as_str(),
            NodeKind::RadialGradient(ref e) => e.id.as_str(),
            NodeKind::ClipPath(ref e) => e.id.as_str(),
            NodeKind::Mask(ref e) => e.id.as_str(),
            NodeKind::Pattern(ref e) => e.id.as_str(),
            #[cfg(feature = "filter")]
            NodeKind::Filter(ref e) => e.id.as_str(),
            NodeKind::Path(ref e) => e.id.as_str(),
            NodeKind::Image(ref e) => e.id.as_str(),
            NodeKind::Group(ref e) => e.id.as_str(),
        }
    }

    /// Returns node's transform.
    ///
    /// If a current node doesn't support transformation - a default
    /// transform will be returned.
    pub fn transform(&self) -> Transform {
        match *self {
            NodeKind::Svg(_) => Transform::default(),
            NodeKind::Defs => Transform::default(),
            NodeKind::LinearGradient(ref e) => e.transform,
            NodeKind::RadialGradient(ref e) => e.transform,
            NodeKind::ClipPath(ref e) => e.transform,
            NodeKind::Mask(_) => Transform::default(),
            NodeKind::Pattern(ref e) => e.transform,
            #[cfg(feature = "filter")]
            NodeKind::Filter(_) => Transform::default(),
            NodeKind::Path(ref e) => e.transform,
            NodeKind::Image(ref e) => e.transform,
            NodeKind::Group(ref e) => e.transform,
        }
    }
}


/// An SVG root element.
#[derive(Clone, Copy, Debug)]
pub struct Svg {
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
    pub data: std::rc::Rc<PathData>,
}

impl Default for Path {
    fn default() -> Self {
        Path {
            id: String::new(),
            transform: Transform::default(),
            visibility: Visibility::Visible,
            fill: None,
            stroke: None,
            rendering_mode: ShapeRendering::default(),
            text_bbox: None,
            data: std::rc::Rc::new(PathData::default()),
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

    /// Element's clip path.
    pub clip_path: Option<String>,

    /// Element's mask.
    pub mask: Option<String>,

    /// Element's filters.
    pub filter: Vec<String>,

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
            opacity: Opacity::default(),
            clip_path: None,
            mask: None,
            filter: Vec::new(),
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
    root: Node,
}

impl Tree {
    /// Parses `Tree` from the SVG data.
    ///
    /// Can contain an SVG string or a gzip compressed data.
    pub fn from_data(data: &[u8], opt: &OptionsRef) -> Result<Self, Error> {
        if data.starts_with(&[0x1f, 0x8b]) {
            let text = deflate(data)?;
            Self::from_str(&text, opt)
        } else {
            let text = std::str::from_utf8(data).map_err(|_| Error::NotAnUtf8Str)?;
            Self::from_str(text, opt)
        }
    }

    /// Parses `Tree` from the SVG string.
    pub fn from_str(text: &str, opt: &OptionsRef) -> Result<Self, Error> {
        let mut xml_opt = roxmltree::ParsingOptions::default();
        xml_opt.allow_dtd = true;

        let doc = roxmltree::Document::parse_with_options(text, xml_opt)
            .map_err(Error::ParsingFailed)?;

        Self::from_xmltree(&doc, opt)
    }

    /// Parses `Tree` from `roxmltree::Document`.
    pub fn from_xmltree(doc: &roxmltree::Document, opt: &OptionsRef) -> Result<Self, Error> {
        let doc = svgtree::Document::parse(doc)?;
        Self::from_svgtree(doc, opt)
    }

    /// Parses `Tree` from the `svgtree::Document`.
    ///
    /// An empty `Tree` will be returned on any error.
    fn from_svgtree(doc: svgtree::Document, opt: &OptionsRef) -> Result<Self, Error> {
        crate::converter::convert_doc(&doc, opt)
    }

    /// Creates a new `Tree`.
    pub fn create(svg: Svg) -> Self {
        let mut root_node = Node::new(NodeKind::Svg(svg));
        let defs_node = Node::new(NodeKind::Defs);
        root_node.append(defs_node);

        Tree {
            root: root_node,
        }
    }

    /// Returns the `Svg` node.
    #[inline]
    pub fn root(&self) -> Node {
        self.root.clone()
    }

    /// Returns the `Svg` node value.
    #[inline]
    pub fn svg_node(&self) -> std::cell::Ref<Svg> {
        std::cell::Ref::map(self.root.borrow(), |v| {
            match *v {
                NodeKind::Svg(ref svg) => svg,
                _ => unreachable!(),
            }
        })
    }

    /// Returns the `Defs` node.
    #[inline]
    pub fn defs(&self) -> Node {
        self.root.first_child().unwrap()
    }

    /// Checks that `node` is part of the `Defs` children.
    pub fn is_in_defs(&self, node: &Node) -> bool {
        let defs = self.defs();
        node.ancestors().any(|n| n == defs)
    }

    /// Appends `NodeKind` to the `Defs` node.
    pub fn append_to_defs(&mut self, kind: NodeKind) -> Node {
        debug_assert!(self.defs_by_id(kind.id()).is_none(),
                      "Element #{} already exists in 'defs'.", kind.id());

        let new_node = Node::new(kind);
        self.defs().append(new_node.clone());
        new_node
    }

    /// Returns `defs` child node by ID.
    pub fn defs_by_id(&self, id: &str) -> Option<Node> {
        for n in self.defs().children() {
            if &*n.id() == id {
                return Some(n);
            }
        }

        None
    }

    /// Returns renderable node by ID.
    ///
    /// If an empty ID is provided, than this method will always return `None`.
    /// Even if tree has nodes with empty ID.
    pub fn node_by_id(&self, id: &str) -> Option<Node> {
        if id.is_empty() {
            return None;
        }

        for node in self.root().descendants() {
            if !self.is_in_defs(&node) && &*node.id() == id {
                    return Some(node);
                }
        }

        None
    }

    /// Converts an SVG.
    #[inline]
    #[cfg(feature = "export")]
    pub fn to_string(&self, opt: &XmlOptions) -> String {
        crate::export::convert(self, opt)
    }

    /// Set a view box for the tree.
    pub(crate) fn set_view_box(&mut self, rect: Rect) {
        if let NodeKind::Svg(svg) = &mut *self.root.borrow_mut() {
            svg.view_box.rect = rect;
        }
    }

    /// Set dimensions for the tree.
    pub(crate) fn set_dimensions(&mut self, width: f64, height: f64) {
        if let NodeKind::Svg(svg) = &mut *self.root.borrow_mut() {
            if let Some(size) = Size::new(width, height) {
                svg.size = size;
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

    /// Returns node's paint server units.
    ///
    /// Returns `None` when node is not a `LinearGradient`, `RadialGradient` or `Pattern`.
    fn units(&self) -> Option<Units>;

    /// Appends `kind` as a node child.
    ///
    /// Shorthand for `Node::append(Node::new(Box::new(kind)))`.
    fn append_kind(&mut self, kind: NodeKind) -> Node;

    /// Calculates node's absolute bounding box.
    ///
    /// Can be expensive on large paths and groups.
    fn calculate_bbox(&self) -> Option<PathBbox>;

    /// Returns the node starting from which the filter background should be rendered.
    #[cfg(feature = "filter")]
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
    fn units(&self) -> Option<Units> {
        match *self.borrow() {
            NodeKind::LinearGradient(ref lg) => Some(lg.units),
            NodeKind::RadialGradient(ref rg) => Some(rg.units),
            NodeKind::Pattern(ref patt) => Some(patt.units),
            _ => None,
        }
    }

    #[inline]
    fn append_kind(&mut self, kind: NodeKind) -> Node {
        let new_node = Node::new(kind);
        self.append(new_node.clone());
        new_node
    }

    #[inline]
    fn calculate_bbox(&self) -> Option<PathBbox> {
        calc_node_bbox(self, self.abs_transform())
    }

    #[cfg(feature = "filter")]
    fn filter_background_start_node(&self, filter: &filter::Filter) -> Option<Node> {
        fn has_enable_background(node: &Node) -> bool {
            if let NodeKind::Group(ref g) = *node.borrow() {
                g.enable_background.is_some()
            } else {
                false
            }
        }

        if !filter.primitives.iter().any(|c| c.kind.has_input(&filter::Input::BackgroundImage)) &&
           !filter.primitives.iter().any(|c| c.kind.has_input(&filter::Input::BackgroundAlpha))
        {
            return None;
        }

        // We should have an ancestor with `enable-background=new`.
        // Skip the current element.
        self.ancestors().skip(1).find(|node| has_enable_background(node))
    }
}

fn deflate(data: &[u8]) -> Result<String, Error> {
    use std::io::Read;

    let mut decoder = flate2::read::GzDecoder::new(data);
    let mut decoded = Vec::with_capacity(data.len() * 2);
    decoder.read_to_end(&mut decoded).map_err(|_| Error::MalformedGZip)?;
    let decoded = String::from_utf8(decoded).map_err(|_| Error::NotAnUtf8Str)?;
    Ok(decoded)
}

fn calc_node_bbox(
    node: &Node,
    ts: Transform,
) -> Option<PathBbox> {
    match *node.borrow() {
        NodeKind::Path(ref path) => {
            path.data.bbox_with_transform(ts, path.stroke.as_ref())
        }
        NodeKind::Image(ref img) => {
            let path = PathData::from_rect(img.view_box.rect);
            path.bbox_with_transform(ts, None)
        }
        NodeKind::Svg(_) | NodeKind::Group(_) => {
            let mut bbox = PathBbox::new_bbox();

            for child in node.children() {
                if let Some(c_bbox) = calc_node_bbox(&child, ts) {
                    bbox = bbox.expand(c_bbox);
                }
            }

            Some(bbox)
        }
        _ => None,
    }
}
