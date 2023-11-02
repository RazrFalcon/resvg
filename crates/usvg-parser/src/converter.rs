// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::str::FromStr;

use svgtypes::{Length, LengthUnit as Unit, TransformOrigin};
use usvg_tree::*;

use crate::svgtree::{self, AId, EId, FromValue, SvgNode};
use crate::units;
use crate::units::convert_length;
use crate::{Error, Options};

#[derive(Clone)]
pub struct State<'a> {
    pub(crate) parent_clip_path: Option<SvgNode<'a, 'a>>,
    pub(crate) parent_markers: Vec<SvgNode<'a, 'a>>,
    pub(crate) fe_image_link: bool,
    /// A viewBox of the parent SVG element.
    pub(crate) view_box: NonZeroRect,
    /// A size of the parent `use` element.
    /// Used only during nested `svg` size resolving.
    /// Width and height can be set independently.
    pub(crate) use_size: (Option<f32>, Option<f32>),
    pub(crate) opt: &'a Options,
}

#[derive(Default)]
pub struct Cache {
    pub clip_paths: HashMap<String, Rc<ClipPath>>,
    pub masks: HashMap<String, Rc<Mask>>,
    pub filters: HashMap<String, Rc<usvg_tree::filter::Filter>>,
    pub paint: HashMap<String, Paint>,

    // used for ID generation
    pub all_ids: HashSet<u64>,
    pub clip_path_index: usize,
    pub filter_index: usize,
}

impl Cache {
    pub fn gen_clip_path_id(&mut self) -> String {
        loop {
            self.clip_path_index += 1;
            let new_id = format!("clipPath{}", self.clip_path_index);
            let new_hash = string_hash(&new_id);
            if !self.all_ids.contains(&new_hash) {
                return new_id;
            }
        }
    }

    pub fn gen_filter_id(&mut self) -> String {
        loop {
            self.filter_index += 1;
            let new_id = format!("filter{}", self.filter_index);
            let new_hash = string_hash(&new_id);
            if !self.all_ids.contains(&new_hash) {
                return new_id;
            }
        }
    }
}

// TODO: is there a simpler way?
fn string_hash(s: &str) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

impl<'a, 'input: 'a> SvgNode<'a, 'input> {
    pub fn convert_length(&self, aid: AId, object_units: Units, state: &State, def: Length) -> f32 {
        units::convert_length(
            self.attribute(aid).unwrap_or(def),
            *self,
            aid,
            object_units,
            state,
        )
    }

    pub fn convert_user_length(&self, aid: AId, state: &State, def: Length) -> f32 {
        self.convert_length(aid, Units::UserSpaceOnUse, state, def)
    }

    pub fn parse_viewbox(&self) -> Option<NonZeroRect> {
        let vb: svgtypes::ViewBox = self.attribute(AId::ViewBox)?;
        NonZeroRect::from_xywh(vb.x as f32, vb.y as f32, vb.w as f32, vb.h as f32)
    }

    pub fn resolve_length(&self, aid: AId, state: &State, def: f32) -> f32 {
        debug_assert!(
            !matches!(aid, AId::BaselineShift | AId::FontSize),
            "{} cannot be resolved via this function",
            aid
        );

        if let Some(n) = self.ancestors().find(|n| n.has_attribute(aid)) {
            if let Some(length) = n.attribute(aid) {
                return units::convert_user_length(length, n, aid, state);
            }
        }

        def
    }

    pub fn resolve_valid_length(
        &self,
        aid: AId,
        state: &State,
        def: f32,
    ) -> Option<NonZeroPositiveF32> {
        let n = self.resolve_length(aid, state, def);
        NonZeroPositiveF32::new(n)
    }

    pub fn try_convert_length(&self, aid: AId, object_units: Units, state: &State) -> Option<f32> {
        Some(units::convert_length(
            self.attribute(aid)?,
            *self,
            aid,
            object_units,
            state,
        ))
    }

    pub fn has_valid_transform(&self, aid: AId) -> bool {
        // Do not use Node::attribute::<Transform>, because it will always
        // return a valid transform.

        let attr = match self.attribute(aid) {
            Some(attr) => attr,
            None => return true,
        };

        let ts = match svgtypes::Transform::from_str(attr) {
            Ok(v) => v,
            Err(_) => return true,
        };

        let ts = Transform::from_row(
            ts.a as f32,
            ts.b as f32,
            ts.c as f32,
            ts.d as f32,
            ts.e as f32,
            ts.f as f32,
        );
        ts.is_valid()
    }

    pub fn is_visible_element(&self, opt: &crate::Options) -> bool {
        self.attribute(AId::Display) != Some("none")
            && self.has_valid_transform(AId::Transform)
            && crate::switch::is_condition_passed(*self, opt)
    }
}

pub trait SvgColorExt {
    fn split_alpha(self) -> (usvg_tree::Color, Opacity);
}

impl SvgColorExt for svgtypes::Color {
    fn split_alpha(self) -> (usvg_tree::Color, Opacity) {
        (
            usvg_tree::Color::new_rgb(self.red, self.green, self.blue),
            Opacity::new_u8(self.alpha),
        )
    }
}

/// Converts an input `Document` into a `Tree`.
///
/// # Errors
///
/// - If `Document` doesn't have an SVG node - returns an empty tree.
/// - If `Document` doesn't have a valid size - returns `Error::InvalidSize`.
pub(crate) fn convert_doc(svg_doc: &svgtree::Document, opt: &Options) -> Result<Tree, Error> {
    let svg = svg_doc.root_element();
    let (size, restore_viewbox) = resolve_svg_size(&svg, opt);
    let size = size?;
    let view_box = ViewBox {
        rect: svg
            .parse_viewbox()
            .unwrap_or_else(|| size.to_non_zero_rect(0.0, 0.0)),
        aspect: svg.attribute(AId::PreserveAspectRatio).unwrap_or_default(),
    };

    let mut tree = Tree {
        size,
        view_box,
        root: Node::new(NodeKind::Group(Group::default())),
    };

    if !svg.is_visible_element(opt) {
        return Ok(tree);
    }

    let state = State {
        parent_clip_path: None,
        parent_markers: Vec::new(),
        fe_image_link: false,
        view_box: view_box.rect,
        use_size: (None, None),
        opt,
    };

    let mut cache = Cache::default();
    for node in svg_doc.descendants() {
        if let Some(tag) = node.tag_name() {
            if matches!(tag, EId::Filter | EId::ClipPath) {
                if !node.element_id().is_empty() {
                    cache.all_ids.insert(string_hash(node.element_id()));
                }
            }
        }
    }

    convert_children(svg_doc.root(), &state, &mut cache, &mut tree.root);

    remove_empty_groups(&mut tree);

    if restore_viewbox {
        calculate_svg_bbox(&mut tree);
    }

    Ok(tree)
}

fn resolve_svg_size(svg: &SvgNode, opt: &Options) -> (Result<Size, Error>, bool) {
    let mut state = State {
        parent_clip_path: None,
        parent_markers: Vec::new(),
        fe_image_link: false,
        view_box: NonZeroRect::from_xywh(0.0, 0.0, 100.0, 100.0).unwrap(),
        use_size: (None, None),
        opt,
    };

    let def = Length::new(100.0, Unit::Percent);
    let mut width: Length = svg.attribute(AId::Width).unwrap_or(def);
    let mut height: Length = svg.attribute(AId::Height).unwrap_or(def);

    let view_box = svg.parse_viewbox();

    let restore_viewbox =
        if (width.unit == Unit::Percent || height.unit == Unit::Percent) && view_box.is_none() {
            // Apply the percentages to the fallback size.
            if width.unit == Unit::Percent {
                width = Length::new(
                    (width.number / 100.0) * state.opt.default_size.width() as f64,
                    Unit::None,
                );
            }

            if height.unit == Unit::Percent {
                height = Length::new(
                    (height.number / 100.0) * state.opt.default_size.height() as f64,
                    Unit::None,
                );
            }

            true
        } else {
            false
        };

    let size = if let Some(vbox) = view_box {
        state.view_box = vbox;

        let w = if width.unit == Unit::Percent {
            vbox.width() * (width.number as f32 / 100.0)
        } else {
            svg.convert_user_length(AId::Width, &state, def)
        };

        let h = if height.unit == Unit::Percent {
            vbox.height() * (height.number as f32 / 100.0)
        } else {
            svg.convert_user_length(AId::Height, &state, def)
        };

        Size::from_wh(w, h)
    } else {
        Size::from_wh(
            svg.convert_user_length(AId::Width, &state, def),
            svg.convert_user_length(AId::Height, &state, def),
        )
    };

    (size.ok_or(Error::InvalidSize), restore_viewbox)
}

/// Calculates SVG's size and viewBox in case there were not set.
///
/// Simply iterates over all nodes and calculates a bounding box.
fn calculate_svg_bbox(tree: &mut Tree) {
    let mut right = 0.0;
    let mut bottom = 0.0;

    for node in tree.root.descendants() {
        if let Some(bbox) = node.calculate_bbox() {
            if bbox.right() > right {
                right = bbox.right();
            }
            if bbox.bottom() > bottom {
                bottom = bbox.bottom();
            }
        }
    }

    if let Some(rect) = NonZeroRect::from_xywh(0.0, 0.0, right, bottom) {
        tree.view_box.rect = rect;
    }

    if let Some(size) = Size::from_wh(right, bottom) {
        tree.size = size;
    }
}

#[inline(never)]
pub(crate) fn convert_children(
    parent_node: SvgNode,
    state: &State,
    cache: &mut Cache,
    parent: &mut Node,
) {
    for node in parent_node.children() {
        convert_element(node, state, cache, parent);
    }
}

#[inline(never)]
pub(crate) fn convert_element(
    node: SvgNode,
    state: &State,
    cache: &mut Cache,
    parent: &mut Node,
) -> Option<Node> {
    let tag_name = node.tag_name()?;

    if !tag_name.is_graphic() && !matches!(tag_name, EId::G | EId::Switch | EId::Svg) {
        return None;
    }

    if !node.is_visible_element(state.opt) {
        return None;
    }

    if tag_name == EId::Use {
        crate::use_node::convert(node, state, cache, parent);
        return None;
    }

    if tag_name == EId::Switch {
        crate::switch::convert(node, state, cache, parent);
        return None;
    }

    let parent = &mut match convert_group(node, state, false, cache, parent) {
        GroupKind::Create(g) => g,
        GroupKind::Skip => parent.clone(),
        GroupKind::Ignore => return None,
    };

    match tag_name {
        EId::Rect
        | EId::Circle
        | EId::Ellipse
        | EId::Line
        | EId::Polyline
        | EId::Polygon
        | EId::Path => {
            if let Some(path) = crate::shapes::convert(node, state) {
                convert_path(node, path, state, cache, parent);
            }
        }
        EId::Image => {
            crate::image::convert(node, state, parent);
        }
        EId::Text => {
            crate::text::convert(node, state, cache, parent);
        }
        EId::Svg => {
            if node.parent_element().is_some() {
                crate::use_node::convert_svg(node, state, cache, parent);
            } else {
                // Skip root `svg`.
                convert_children(node, state, cache, parent);
            }
        }
        EId::G => {
            convert_children(node, state, cache, parent);
        }
        _ => {}
    }

    Some(parent.clone())
}

// `clipPath` can have only shape and `text` children.
//
// `line` doesn't impact rendering because stroke is always disabled
// for `clipPath` children.
#[inline(never)]
pub(crate) fn convert_clip_path_elements(
    clip_node: SvgNode,
    state: &State,
    cache: &mut Cache,
    parent: &mut Node,
) {
    for node in clip_node.children() {
        let tag_name = match node.tag_name() {
            Some(v) => v,
            None => continue,
        };

        if !tag_name.is_graphic() {
            continue;
        }

        if !node.is_visible_element(state.opt) {
            continue;
        }

        if tag_name == EId::Use {
            crate::use_node::convert(node, state, cache, parent);
            continue;
        }

        let parent = &mut match convert_group(node, state, false, cache, parent) {
            GroupKind::Create(g) => g,
            GroupKind::Skip => parent.clone(),
            GroupKind::Ignore => continue,
        };

        match tag_name {
            EId::Rect | EId::Circle | EId::Ellipse | EId::Polyline | EId::Polygon | EId::Path => {
                if let Some(path) = crate::shapes::convert(node, state) {
                    convert_path(node, path, state, cache, parent);
                }
            }
            EId::Text => {
                crate::text::convert(node, state, cache, parent);
            }
            _ => {
                log::warn!("'{}' is no a valid 'clip-path' child.", tag_name);
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Isolation {
    Auto,
    Isolate,
}

impl Default for Isolation {
    fn default() -> Self {
        Self::Auto
    }
}

impl<'a, 'input: 'a> FromValue<'a, 'input> for Isolation {
    fn parse(_: SvgNode, _: AId, value: &str) -> Option<Self> {
        match value {
            "auto" => Some(Isolation::Auto),
            "isolate" => Some(Isolation::Isolate),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum GroupKind {
    /// Creates a new group.
    Create(Node),
    /// Skips an existing group, but processes its children.
    Skip,
    /// Skips an existing group and all its children.
    Ignore,
}

// TODO: explain
pub(crate) fn convert_group(
    node: SvgNode,
    state: &State,
    force: bool,
    cache: &mut Cache,
    parent: &mut Node,
) -> GroupKind {
    // A `clipPath` child cannot have an opacity.
    let opacity = if state.parent_clip_path.is_none() {
        node.attribute::<Opacity>(AId::Opacity)
            .unwrap_or(Opacity::ONE)
    } else {
        Opacity::ONE
    };

    // TODO: remove macro
    macro_rules! resolve_link {
        ($aid:expr, $f:expr) => {{
            let mut v = None;

            if let Some(link) = node.attribute::<SvgNode>($aid) {
                v = $f(link, state, cache);

                // If `$aid` is linked to an invalid element - skip this group completely.
                if v.is_none() {
                    return GroupKind::Ignore;
                }
            }

            v
        }};
    }

    // `mask` and `filter` cannot be set on `clipPath` children.
    // But `clip-path` can.

    let clip_path = resolve_link!(AId::ClipPath, crate::clippath::convert);

    let mask = if state.parent_clip_path.is_none() {
        resolve_link!(AId::Mask, crate::mask::convert)
    } else {
        None
    };

    let filters = {
        let mut filters = Vec::new();
        if state.parent_clip_path.is_none() {
            if node.attribute(AId::Filter) == Some("none") {
                // Do nothing.
            } else if node.has_attribute(AId::Filter) {
                if let Ok(f) = crate::filter::convert(node, state, cache) {
                    filters = f;
                } else {
                    // A filter that not a link or a filter with a link to a non existing element.
                    //
                    // Unlike `clip-path` and `mask`, when a `filter` link is invalid
                    // then the whole element should be ignored.
                    //
                    // This is kinda an undefined behaviour.
                    // In most cases, Chrome, Firefox and rsvg will ignore such elements,
                    // but in some cases Chrome allows it. Not sure why.
                    // Inkscape (0.92) simply ignores such attributes, rendering element as is.
                    // Batik (1.12) crashes.
                    //
                    // Test file: e-filter-051.svg
                    return GroupKind::Ignore;
                }
            }
        }

        filters
    };

    let transform: Transform = resolve_transform_origin(
        node,
        state,
        node.attribute(AId::Transform).unwrap_or_default(),
    );
    let blend_mode: BlendMode = node.attribute(AId::MixBlendMode).unwrap_or_default();
    let isolation: Isolation = node.attribute(AId::Isolation).unwrap_or_default();
    let isolate = isolation == Isolation::Isolate;

    // TODO: ignore just transform
    let is_g_or_use = matches!(node.tag_name(), Some(EId::G) | Some(EId::Use));
    let required = opacity.get().approx_ne_ulps(&1.0, 4)
        || clip_path.is_some()
        || mask.is_some()
        || !filters.is_empty()
        || !transform.is_identity()
        || blend_mode != BlendMode::Normal
        || isolate
        || is_g_or_use
        || force;

    if required {
        // Nodes generated by markers must not have an ID. Otherwise we would have duplicates.
        let id = if is_g_or_use && state.parent_markers.is_empty() {
            node.element_id().to_string()
        } else {
            String::new()
        };

        let g = parent.append_kind(NodeKind::Group(Group {
            id,
            transform,
            opacity,
            blend_mode,
            isolate,
            clip_path,
            mask,
            filters,
        }));

        GroupKind::Create(g)
    } else {
        GroupKind::Skip
    }
}

pub(crate) fn resolve_transform_origin(
    node: SvgNode,
    state: &State,
    mut transform: Transform,
) -> Transform {
    let transform_origin: Option<TransformOrigin> = node.attribute(AId::TransformOrigin);

    if let Some(transform_origin) = transform_origin {
        let transform_origin = Transform::from_translate(
            convert_length(
                transform_origin.x_offset,
                node,
                AId::Width,
                Units::UserSpaceOnUse,
                state,
            ),
            convert_length(
                transform_origin.y_offset,
                node,
                AId::Height,
                Units::UserSpaceOnUse,
                state,
            ),
        );
        transform = transform_origin
            .pre_concat(transform)
            .pre_concat(Transform::from_translate(
                -transform_origin.tx,
                -transform_origin.ty,
            ));
    }

    transform
}

fn remove_empty_groups(tree: &mut Tree) {
    fn rm(parent: Node) -> bool {
        let mut changed = false;

        let mut curr_node = parent.first_child();
        while let Some(node) = curr_node {
            curr_node = node.next_sibling();

            let is_g = if let NodeKind::Group(ref g) = *node.borrow() {
                // Skip empty groups when they do not have a `filter` property.
                // The `filter` property can be set on empty groups. For example:
                //
                // <filter id="filter1" filterUnits="userSpaceOnUse"
                //         x="20" y="20" width="160" height="160">
                //   <feFlood flood-color="green"/>
                // </filter>
                // <g filter="url(#filter1)"/>
                g.filters.is_empty()
            } else {
                false
            };

            if is_g && !node.has_children() {
                node.detach();
                changed = true;
            } else {
                if rm(node) {
                    changed = true;
                }
            }
        }

        changed
    }

    while rm(tree.root.clone()) {}
}

fn convert_path(
    node: SvgNode,
    path: Rc<tiny_skia_path::Path>,
    state: &State,
    cache: &mut Cache,
    parent: &mut Node,
) {
    debug_assert!(path.len() >= 2);
    if path.len() < 2 {
        return;
    }

    let has_bbox = path.bounds().width() > 0.0 && path.bounds().height() > 0.0;
    let fill = crate::style::resolve_fill(node, has_bbox, state, cache);
    let stroke = crate::style::resolve_stroke(node, has_bbox, state, cache);
    let mut visibility: Visibility = node.find_attribute(AId::Visibility).unwrap_or_default();
    let rendering_mode: ShapeRendering = node
        .find_attribute(AId::ShapeRendering)
        .unwrap_or(state.opt.shape_rendering);

    // TODO: handle `markers` before `stroke`
    let raw_paint_order: svgtypes::PaintOrder =
        node.find_attribute(AId::PaintOrder).unwrap_or_default();
    let paint_order = svg_paint_order_to_usvg(raw_paint_order);

    // If a path doesn't have a fill or a stroke than it's invisible.
    // By setting `visibility` to `hidden` we are disabling rendering of this path.
    if fill.is_none() && stroke.is_none() {
        visibility = Visibility::Hidden;
    }

    let mut markers_group = None;
    if crate::marker::is_valid(node) && visibility == Visibility::Visible {
        let mut g = parent.append_kind(NodeKind::Group(Group::default()));
        crate::marker::convert(node, &path, state, cache, &mut g);
        markers_group = Some(g);
    }

    // Nodes generated by markers must not have an ID. Otherwise we would have duplicates.
    let id = if state.parent_markers.is_empty() {
        node.element_id().to_string()
    } else {
        String::new()
    };

    parent.append_kind(NodeKind::Path(Path {
        id,
        visibility,
        fill,
        stroke,
        paint_order,
        rendering_mode,
        text_bbox: None,
        data: path,
    }));

    if raw_paint_order.order[2] == svgtypes::PaintOrderKind::Markers {
        // Insert markers group after `path`.
        if let Some(g) = markers_group {
            g.detach();
            parent.append(g);
        }
    }
}

pub fn svg_paint_order_to_usvg(order: svgtypes::PaintOrder) -> PaintOrder {
    match (order.order[0], order.order[1]) {
        (svgtypes::PaintOrderKind::Stroke, _) => PaintOrder::StrokeAndFill,
        (svgtypes::PaintOrderKind::Markers, svgtypes::PaintOrderKind::Stroke) => {
            PaintOrder::StrokeAndFill
        }
        _ => PaintOrder::FillAndStroke,
    }
}
