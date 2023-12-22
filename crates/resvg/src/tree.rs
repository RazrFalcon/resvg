// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::clip::ClipPath;
use crate::image::Image;
use crate::mask::Mask;
use crate::path::{FillPath, StrokePath};

pub struct Group {
    pub transform: tiny_skia::Transform,
    pub opacity: usvg::Opacity,
    pub blend_mode: tiny_skia::BlendMode,
    pub clip_path: Option<ClipPath>,
    pub mask: Option<Mask>,
    pub filters: Vec<crate::filter::Filter>,
    pub isolate: bool,
    /// Group's layer bounding box in object coordinates.
    pub bbox: tiny_skia::Rect,

    pub children: Vec<Node>,
}

impl Group {
    pub fn is_transform_only(&self) -> bool {
        self.opacity == usvg::Opacity::ONE
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
    pub content_area: Option<tiny_skia::Rect>,

    pub(crate) children: Vec<Node>,
}

impl Tree {
    /// Creates a rendering tree from [`usvg::Tree`].
    ///
    /// Text nodes should be already converted into paths using
    /// [`usvg::TreeTextToPath::convert_text`].
    pub fn from_usvg(tree: &usvg::Tree) -> Self {
        let mut children = Vec::new();
        let layer_bbox = convert_children(&tree.root, None, &mut children);

        Self {
            size: tree.size,
            view_box: tree.view_box,
            content_area: layer_bbox.and_then(|r| r.to_rect()),
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
        let node_bbox =
            if let Some(bbox) = node.abs_bounding_box().and_then(|r| r.to_non_zero_rect()) {
                bbox
            } else {
                log::warn!("Node '{}' has zero size.", node.id());
                return None;
            };

        let view_box = usvg::ViewBox {
            rect: node_bbox,
            aspect: usvg::AspectRatio::default(),
        };

        let mut children = Vec::new();
        let layer_bbox = convert_node_inner(node, None, &mut children);

        Some(Self {
            size: node_bbox.size(),
            view_box,
            content_area: layer_bbox.and_then(|r| r.to_rect()),
            children,
        })
    }

    // TODO: find a better solution
    pub(crate) fn from_usvg_group(node: &usvg::Group) -> Option<Self> {
        let node_bbox =
            if let Some(bbox) = node.abs_bounding_box().and_then(|r| r.to_non_zero_rect()) {
                bbox
            } else {
                log::warn!("Node '{}' has zero size.", node.id);
                return None;
            };

        let view_box = usvg::ViewBox {
            rect: node_bbox,
            aspect: usvg::AspectRatio::default(),
        };

        let mut children = Vec::new();
        let layer_bbox = convert_children(node, None, &mut children);

        Some(Self {
            size: node_bbox.size(),
            view_box,
            content_area: layer_bbox.and_then(|r| r.to_rect()),
            children,
        })
    }
}

pub fn convert_root(node: &usvg::Group) -> Vec<Node> {
    let mut children = Vec::new();
    convert_children(node, None, &mut children);
    children
}

fn convert_node_inner(
    node: &usvg::Node,
    text_bbox: Option<tiny_skia::NonZeroRect>,
    children: &mut Vec<Node>,
) -> Option<usvg::BBox> {
    match node {
        usvg::Node::Group(ref ugroup) => convert_group(ugroup, text_bbox, children),
        usvg::Node::Path(ref upath) => crate::path::convert(upath, text_bbox, children),
        usvg::Node::Image(ref uimage) => crate::image::convert(uimage, children),
        usvg::Node::Text(ref utext) => {
            if let (Some(bbox), Some(ref flattened)) = (utext.bounding_box, &utext.flattened) {
                convert_children(flattened, Some(bbox), children)
            } else {
                log::warn!("Text nodes should be flattened before rendering.");
                None
            }
        }
    }
}

fn convert_group(
    ugroup: &usvg::Group,
    text_bbox: Option<tiny_skia::NonZeroRect>,
    children: &mut Vec<Node>,
) -> Option<usvg::BBox> {
    let mut group_children = Vec::new();
    let mut layer_bbox = match convert_children(ugroup, text_bbox, &mut group_children) {
        Some(v) => v,
        None => return convert_empty_group(ugroup, children),
    };

    let (filters, filter_bbox) = crate::filter::convert(&ugroup.filters, ugroup.bounding_box);

    // TODO: figure out a nicer solution
    // Ignore groups with filters but invalid filter bboxes.
    if !ugroup.filters.is_empty() && filter_bbox.is_none() {
        return None;
    }

    if let Some(filter_bbox) = filter_bbox {
        layer_bbox = usvg::BBox::from(filter_bbox);
    }

    let bounding_box = ugroup.bounding_box?;
    let bbox = layer_bbox.to_rect()?;

    let group = Group {
        transform: ugroup.transform,
        opacity: ugroup.opacity,
        blend_mode: convert_blend_mode(ugroup.blend_mode),
        clip_path: crate::clip::convert(ugroup.clip_path.clone(), bounding_box),
        mask: crate::mask::convert(ugroup.mask.clone(), bounding_box),
        isolate: ugroup.isolate,
        filters,
        bbox,
        children: group_children,
    };

    layer_bbox = layer_bbox.transform(ugroup.transform)?;

    children.push(Node::Group(group));
    Some(layer_bbox)
}

fn convert_empty_group(ugroup: &usvg::Group, children: &mut Vec<Node>) -> Option<usvg::BBox> {
    if ugroup.filters.is_empty() {
        return None;
    }

    let (filters, layer_bbox) = crate::filter::convert(&ugroup.filters, None);
    let layer_bbox = layer_bbox?;

    let group = Group {
        transform: ugroup.transform,
        opacity: ugroup.opacity,
        blend_mode: convert_blend_mode(ugroup.blend_mode),
        clip_path: None,
        mask: None,
        isolate: ugroup.isolate,
        filters,
        bbox: layer_bbox,
        children: Vec::new(),
    };

    children.push(Node::Group(group));
    Some(usvg::BBox::from(layer_bbox))
}

fn convert_children(
    parent: &usvg::Group,
    text_bbox: Option<tiny_skia::NonZeroRect>,
    children: &mut Vec<Node>,
) -> Option<usvg::BBox> {
    let mut layer_bbox = usvg::BBox::default();

    for node in &parent.children {
        if let Some(layer_bbox_2) = convert_node_inner(node, text_bbox, children) {
            layer_bbox = layer_bbox.expand(layer_bbox_2);
        }
    }

    if !layer_bbox.is_default() {
        Some(layer_bbox)
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
