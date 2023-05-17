// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use usvg::{FuzzyEq, NodeExt};

use crate::clip::ClipPath;
use crate::image::Image;
use crate::mask::Mask;
use crate::paint_server::Paint;
use crate::path::{FillPath, StrokePath};

pub struct Group {
    pub transform: tiny_skia::Transform,
    pub opacity: f32,
    pub blend_mode: tiny_skia::BlendMode,
    pub clip_path: Option<ClipPath>,
    pub mask: Option<Mask>,
    pub isolate: bool,

    pub filters: Vec<crate::filter::Filter>,
    pub filter_fill: Option<Paint>,
    pub filter_stroke: Option<Paint>,
    /// Group's layer bounding box in canvas coordinates.
    pub bbox: usvg::PathBbox,

    pub children: Vec<Node>,
}

impl Group {
    pub fn is_transform_only(&self) -> bool {
        self.opacity == 1.0
            && self.blend_mode == tiny_skia::BlendMode::SourceOver
            && self.clip_path.is_none()
            && self.mask.is_none()
            && self.filters.is_empty()
            && !self.isolate
    }
}

pub enum Node {
    Group(Group), // TODO: box
    FillPath(FillPath),
    StrokePath(StrokePath),
    Image(Image),
}

// - No hidden nodes.
// - No text.
// - Uses mostly tiny-skia types.
// - No paint-order. Already resolved.
// - PNG/JPEG/GIF bitmaps are already decoded and are stored as tiny_skia::Pixmap.
//   SVG images will be rendered each time.
// - No `objectBoundingBox` units.

/// A render tree.
pub struct Tree {
    /// Image size.
    ///
    /// Size of an image that should be created to fit the SVG.
    ///
    /// `width` and `height` in SVG.
    pub size: usvg::Size,

    /// SVG viewbox.
    ///
    /// Specifies which part of the SVG image should be rendered.
    ///
    /// `viewBox` and `preserveAspectRatio` in SVG.
    pub view_box: usvg::ViewBox,

    /// Content area.
    ///
    /// A bounding box of all elements. Includes strokes and filter regions.
    ///
    /// Can be `None` when the tree has no children.
    pub content_area: Option<usvg::PathBbox>,

    pub(crate) children: Vec<Node>,
}

impl Tree {
    /// Creates a rendering tree from [`usvg::Tree`].
    ///
    /// Text nodes should be already converted into paths using
    /// [`usvg::TreeTextToPath::convert_text`].
    pub fn from_usvg(tree: &usvg::Tree) -> Self {
        if tree.has_text_nodes() {
            log::warn!("Text nodes should be already converted into paths.");
        }

        let (children, layer_bbox) = convert_node(tree.root.clone());

        Self {
            size: tree.size,
            view_box: tree.view_box,
            content_area: layer_bbox,
            children,
        }
    }

    /// Creates a rendering tree from [`usvg::Node`].
    ///
    /// Text nodes should be already converted into paths using
    /// [`usvg::TreeTextToPath::convert_text`].
    ///
    /// Returns `None` when `node` has a zero size.
    pub fn from_usvg_node(node: &usvg::Node) -> Option<Self> {
        let node_bbox = if let Some(bbox) = node.calculate_bbox().and_then(|r| r.to_rect()) {
            bbox
        } else {
            log::warn!("Node '{}' has zero size.", node.id());
            return None;
        };

        let view_box = usvg::ViewBox {
            rect: node_bbox,
            aspect: usvg::AspectRatio::default(),
        };

        let (children, layer_bbox) = convert_node(node.clone());

        Some(Self {
            size: node_bbox.size(),
            view_box: view_box,
            content_area: layer_bbox,
            children,
        })
    }
}

pub fn convert_node(node: usvg::Node) -> (Vec<Node>, Option<usvg::PathBbox>) {
    let mut children = Vec::new();
    let bboxes = convert_node_inner(node, &mut children);
    (children, bboxes.map(|b| b.layer))
}

#[derive(Debug)]
pub struct BBoxes {
    /// The object bounding box.
    ///
    /// Just a shape/image bbox as per SVG spec.
    pub object: usvg::PathBbox,

    /// The same as above, but transformed using object's transform.
    pub transformed_object: usvg::PathBbox,

    /// Similar to `object`, but expanded to fit the stroke as well.
    pub layer: usvg::PathBbox,
}

impl Default for BBoxes {
    fn default() -> Self {
        Self {
            object: usvg::PathBbox::new_bbox(),
            transformed_object: usvg::PathBbox::new_bbox(),
            layer: usvg::PathBbox::new_bbox(),
        }
    }
}

fn convert_node_inner(node: usvg::Node, children: &mut Vec<Node>) -> Option<BBoxes> {
    match &*node.borrow() {
        usvg::NodeKind::Group(ref ugroup) => convert_group(node.clone(), ugroup, children),
        usvg::NodeKind::Path(ref upath) => crate::path::convert(upath, children),
        usvg::NodeKind::Image(ref uimage) => crate::image::convert(uimage, children),
        usvg::NodeKind::Text(_) => None, // should be already converted into paths
    }
}

fn convert_group(
    node: usvg::Node,
    ugroup: &usvg::Group,
    children: &mut Vec<Node>,
) -> Option<BBoxes> {
    let mut group_children = Vec::new();
    let mut bboxes = match convert_children(node, &mut group_children) {
        Some(v) => v,
        None => return convert_empty_group(ugroup, children),
    };

    let (filters, filter_bbox) =
        crate::filter::convert(&ugroup.filters, Some(bboxes.transformed_object));

    // TODO: figure out a nicer solution
    // Ignore groups with filters but invalid filter bboxes.
    if !ugroup.filters.is_empty() && filter_bbox.is_none() {
        return None;
    }

    if let Some(filter_bbox) = filter_bbox {
        bboxes.layer = filter_bbox;
    }

    let mut filter_fill = None;
    if let Some(ref paint) = ugroup.filter_fill {
        filter_fill =
            crate::paint_server::convert(&paint, usvg::Opacity::ONE, bboxes.layer.to_skia_rect());
    }

    let mut filter_stroke = None;
    if let Some(ref paint) = ugroup.filter_stroke {
        filter_stroke =
            crate::paint_server::convert(&paint, usvg::Opacity::ONE, bboxes.layer.to_skia_rect());
    }

    let group = Group {
        transform: ugroup.transform.to_native(),
        opacity: ugroup.opacity.get() as f32,
        blend_mode: convert_blend_mode(ugroup.blend_mode),
        clip_path: crate::clip::convert(ugroup.clip_path.clone(), bboxes.object),
        mask: crate::mask::convert(ugroup.mask.clone(), bboxes.object),
        isolate: ugroup.isolate,
        filters,
        filter_fill,
        filter_stroke,
        bbox: bboxes.layer,
        children: group_children,
    };

    bboxes.object = bboxes.object.transform(&ugroup.transform)?;
    bboxes.transformed_object = bboxes.transformed_object.transform(&ugroup.transform)?;
    bboxes.layer = bboxes.layer.transform(&ugroup.transform)?;

    children.push(Node::Group(group));
    Some(bboxes)
}

fn convert_empty_group(ugroup: &usvg::Group, children: &mut Vec<Node>) -> Option<BBoxes> {
    if ugroup.filters.is_empty() {
        return None;
    }

    let (filters, layer_bbox) = crate::filter::convert(&ugroup.filters, None);
    let layer_bbox = layer_bbox?;

    let mut filter_fill = None;
    if let Some(ref paint) = ugroup.filter_fill {
        filter_fill =
            crate::paint_server::convert(&paint, usvg::Opacity::ONE, layer_bbox.to_skia_rect());
    }

    let mut filter_stroke = None;
    if let Some(ref paint) = ugroup.filter_stroke {
        filter_stroke =
            crate::paint_server::convert(&paint, usvg::Opacity::ONE, layer_bbox.to_skia_rect());
    }

    let group = Group {
        transform: ugroup.transform.to_native(),
        opacity: ugroup.opacity.get() as f32,
        blend_mode: convert_blend_mode(ugroup.blend_mode),
        clip_path: None,
        mask: None,
        isolate: ugroup.isolate,
        filters,
        filter_fill,
        filter_stroke,
        bbox: layer_bbox,
        children: Vec::new(),
    };

    let bboxes = BBoxes {
        // TODO: find a better solution
        object: usvg::PathBbox::new(0.0, 0.0, 1.0, 1.0).unwrap(),
        transformed_object: usvg::PathBbox::new(0.0, 0.0, 1.0, 1.0).unwrap(),
        layer: layer_bbox,
    };

    children.push(Node::Group(group));
    Some(bboxes)
}

fn convert_children(parent: usvg::Node, children: &mut Vec<Node>) -> Option<BBoxes> {
    let mut bboxes = BBoxes::default();

    for node in parent.children() {
        if let Some(bboxes2) = convert_node_inner(node, children) {
            bboxes.object = bboxes.object.expand(bboxes2.object);
            bboxes.transformed_object =
                bboxes.transformed_object.expand(bboxes2.transformed_object);
            bboxes.layer = bboxes.layer.expand(bboxes2.layer);
        }
    }

    if bboxes.layer.fuzzy_ne(&usvg::PathBbox::new_bbox())
        && bboxes.object.fuzzy_ne(&usvg::PathBbox::new_bbox())
    {
        Some(bboxes)
    } else {
        None
    }
}

pub fn convert_blend_mode(mode: usvg::BlendMode) -> tiny_skia::BlendMode {
    match mode {
        usvg::BlendMode::Normal => tiny_skia::BlendMode::SourceOver,
        usvg::BlendMode::Multiply => tiny_skia::BlendMode::Multiply,
        usvg::BlendMode::Screen => tiny_skia::BlendMode::Screen,
        usvg::BlendMode::Overlay => tiny_skia::BlendMode::Overlay,
        usvg::BlendMode::Darken => tiny_skia::BlendMode::Darken,
        usvg::BlendMode::Lighten => tiny_skia::BlendMode::Lighten,
        usvg::BlendMode::ColorDodge => tiny_skia::BlendMode::ColorDodge,
        usvg::BlendMode::ColorBurn => tiny_skia::BlendMode::ColorBurn,
        usvg::BlendMode::HardLight => tiny_skia::BlendMode::HardLight,
        usvg::BlendMode::SoftLight => tiny_skia::BlendMode::SoftLight,
        usvg::BlendMode::Difference => tiny_skia::BlendMode::Difference,
        usvg::BlendMode::Exclusion => tiny_skia::BlendMode::Exclusion,
        usvg::BlendMode::Hue => tiny_skia::BlendMode::Hue,
        usvg::BlendMode::Saturation => tiny_skia::BlendMode::Saturation,
        usvg::BlendMode::Color => tiny_skia::BlendMode::Color,
        usvg::BlendMode::Luminosity => tiny_skia::BlendMode::Luminosity,
    }
}

pub trait OptionLog {
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

pub trait ConvTransform {
    fn to_native(&self) -> tiny_skia::Transform;
    fn from_native(_: tiny_skia::Transform) -> Self;
}

impl ConvTransform for usvg::Transform {
    fn to_native(&self) -> tiny_skia::Transform {
        tiny_skia::Transform::from_row(
            self.a as f32,
            self.b as f32,
            self.c as f32,
            self.d as f32,
            self.e as f32,
            self.f as f32,
        )
    }

    fn from_native(ts: tiny_skia::Transform) -> Self {
        Self::new(
            ts.sx as f64,
            ts.ky as f64,
            ts.kx as f64,
            ts.sy as f64,
            ts.tx as f64,
            ts.ty as f64,
        )
    }
}

pub trait TinySkiaRectExt {
    fn to_path_bbox(&self) -> Option<usvg::PathBbox>;
}

impl TinySkiaRectExt for tiny_skia::Rect {
    fn to_path_bbox(&self) -> Option<usvg::PathBbox> {
        usvg::PathBbox::new(
            self.x() as f64,
            self.y() as f64,
            self.width() as f64,
            self.height() as f64,
        )
    }
}

pub trait UsvgPathBboxExt {
    fn to_skia_rect(&self) -> tiny_skia::Rect;
}

impl UsvgPathBboxExt for usvg::PathBbox {
    fn to_skia_rect(&self) -> tiny_skia::Rect {
        tiny_skia::Rect::from_xywh(
            self.x() as f32,
            self.y() as f32,
            self.width() as f32,
            self.height() as f32,
        )
        .unwrap()
    }
}

pub trait TinySkiaTransformExt {
    fn from_bbox(bbox: usvg::Rect) -> tiny_skia::Transform;
}

impl TinySkiaTransformExt for tiny_skia::Transform {
    fn from_bbox(bbox: usvg::Rect) -> tiny_skia::Transform {
        tiny_skia::Transform::from_row(
            bbox.width() as f32,
            0.0,
            0.0,
            bbox.height() as f32,
            bbox.x() as f32,
            bbox.y() as f32,
        )
    }
}
