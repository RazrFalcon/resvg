// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use svgtypes::{Length, LengthUnit as Unit};

use crate::svgtree::{self, AId, EId};
use crate::*;

#[derive(Clone)]
pub struct State<'a> {
    pub(crate) parent_clip_path: Option<svgtree::Node<'a>>,
    pub(crate) parent_marker: Option<svgtree::Node<'a>>,
    pub(crate) fe_image_link: bool,
    /// The size of the root SVG element.
    /// Right now, used only by use_node::get_clip_rect.
    pub(crate) size: Size,
    /// A viewBox of the parent SVG element.
    pub(crate) view_box: Rect,
    /// A size of the parent `use` element.
    /// Used only during nested `svg` size resolving.
    /// Width and height can be set independently.
    pub(crate) use_size: (Option<f64>, Option<f64>),
    pub(crate) opt: &'a Options,
}

#[derive(Default)]
pub struct Cache {
    pub clip_paths: HashMap<String, Rc<ClipPath>>,
    pub masks: HashMap<String, Rc<Mask>>,
    pub filters: HashMap<String, Rc<filter::Filter>>,
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
        rect: svg.get_viewbox().unwrap_or_else(|| size.to_rect(0.0, 0.0)),
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
        parent_marker: None,
        fe_image_link: false,
        size,
        view_box: view_box.rect,
        use_size: (None, None),
        opt,
    };

    let mut cache = Cache::default();
    for node in svg_doc.descendants() {
        if let Some(tag) = node.tag_name() {
            if matches!(tag, EId::Filter | EId::ClipPath) {
                if node.has_element_id() {
                    cache.all_ids.insert(string_hash(node.element_id()));
                }
            }
        }
    }

    convert_children(svg_doc.root(), &state, &mut cache, &mut tree.root);

    remove_empty_groups(&mut tree);
    ungroup_groups(tree.root.clone(), opt.keep_named_groups);

    if restore_viewbox {
        calculate_svg_bbox(&mut tree);
    }

    Ok(tree)
}

fn resolve_svg_size(svg: &svgtree::Node, opt: &Options) -> (Result<Size, Error>, bool) {
    let mut state = State {
        parent_clip_path: None,
        parent_marker: None,
        fe_image_link: false,
        size: Size::new(100.0, 100.0).unwrap(),
        view_box: Rect::new(0.0, 0.0, 100.0, 100.0).unwrap(),
        use_size: (None, None),
        opt,
    };

    let def = Length::new(100.0, Unit::Percent);
    let mut width: Length = svg.attribute(AId::Width).unwrap_or(def);
    let mut height: Length = svg.attribute(AId::Height).unwrap_or(def);

    let view_box = svg.get_viewbox();

    let restore_viewbox =
        if (width.unit == Unit::Percent || height.unit == Unit::Percent) && view_box.is_none() {
            // Apply the percentages to the fallback size.
            if width.unit == Unit::Percent {
                width = Length::new(
                    (width.number / 100.0) * state.opt.default_size.width(),
                    Unit::None,
                );
            }

            if height.unit == Unit::Percent {
                height = Length::new(
                    (height.number / 100.0) * state.opt.default_size.height(),
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
            vbox.width() * (width.number / 100.0)
        } else {
            svg.convert_user_length(AId::Width, &state, def)
        };

        let h = if height.unit == Unit::Percent {
            vbox.height() * (height.number / 100.0)
        } else {
            svg.convert_user_length(AId::Height, &state, def)
        };

        Size::new(w, h)
    } else {
        Size::new(
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

    if let Some(rect) = Rect::new(0.0, 0.0, right, bottom) {
        tree.view_box.rect = rect;
    }

    if let Some(size) = Size::new(right, bottom) {
        tree.size = size;
    }
}

#[inline(never)]
pub(crate) fn convert_children(
    parent_node: svgtree::Node,
    state: &State,
    cache: &mut converter::Cache,
    parent: &mut Node,
) {
    for node in parent_node.children() {
        convert_element(node, state, cache, parent);
    }
}

#[inline(never)]
pub(crate) fn convert_element(
    node: svgtree::Node,
    state: &State,
    cache: &mut converter::Cache,
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
        use_node::convert(node, state, cache, parent);
        return None;
    }

    if tag_name == EId::Switch {
        switch::convert(node, state, cache, parent);
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
            if let Some(path) = shapes::convert(node, state) {
                convert_path(node, path, state, cache, parent);
            }
        }
        EId::Image => {
            image::convert(node, state, parent);
        }
        EId::Text => {
            text::convert(node, state, cache, parent);
        }
        EId::Svg => {
            if node.parent_element().is_some() {
                use_node::convert_svg(node, state, cache, parent);
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
    clip_node: svgtree::Node,
    state: &State,
    cache: &mut converter::Cache,
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
            use_node::convert(node, state, cache, parent);
            continue;
        }

        let parent = &mut match convert_group(node, state, false, cache, parent) {
            GroupKind::Create(g) => g,
            GroupKind::Skip => parent.clone(),
            GroupKind::Ignore => continue,
        };

        match tag_name {
            EId::Rect | EId::Circle | EId::Ellipse | EId::Polyline | EId::Polygon | EId::Path => {
                if let Some(path) = shapes::convert(node, state) {
                    convert_path(node, path, state, cache, parent);
                }
            }
            EId::Text => {
                text::convert(node, state, cache, parent);
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

impl_enum_default!(Isolation, Auto);

impl_enum_from_str!(Isolation,
    "auto" => Isolation::Auto,
    "isolate" => Isolation::Isolate
);

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
    node: svgtree::Node,
    state: &State,
    force: bool,
    cache: &mut Cache,
    parent: &mut Node,
) -> GroupKind {
    // A `clipPath` child cannot have an opacity.
    let opacity = if state.parent_clip_path.is_none() {
        node.attribute(AId::Opacity).unwrap_or(Opacity::ONE)
    } else {
        Opacity::ONE
    };

    macro_rules! resolve_link {
        ($aid:expr, $f:expr) => {{
            let mut v = None;

            if let Some(link) = node.attribute::<svgtree::Node>($aid) {
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

    let clip_path = resolve_link!(AId::ClipPath, clippath::convert);

    let mask = if state.parent_clip_path.is_none() {
        resolve_link!(AId::Mask, mask::convert)
    } else {
        None
    };

    let (filters, filter_fill, filter_stroke) = {
        let mut filters = Vec::new();
        if state.parent_clip_path.is_none() {
            if node.attribute(AId::Filter) == Some("none") {
                // Do nothing.
            } else if node.has_attribute(AId::Filter) {
                if let Ok(f) = filter::convert(node, state, cache) {
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

        let filter_fill = resolve_filter_fill(node, state, &filters, cache);
        let filter_stroke = resolve_filter_stroke(node, state, &filters, cache);

        (filters, filter_fill, filter_stroke)
    };

    let transform: Transform = node.attribute(AId::Transform).unwrap_or_default();
    let blend_mode: BlendMode = node.attribute(AId::MixBlendMode).unwrap_or_default();
    let isolation: Isolation = node.attribute(AId::Isolation).unwrap_or_default();
    let isolate = isolation == Isolation::Isolate;
    let enable_background = node.attribute(AId::EnableBackground);

    let is_g_or_use = node.has_tag_name(EId::G) || node.has_tag_name(EId::Use);
    let required = opacity.get().fuzzy_ne(&1.0)
        || clip_path.is_some()
        || mask.is_some()
        || !filters.is_empty()
        || !transform.is_default()
        || enable_background.is_some()
        || blend_mode != BlendMode::Normal
        || isolate
        || (is_g_or_use
            && node.has_element_id()
            && (state.opt.keep_named_groups || state.fe_image_link))
        || force;

    if required {
        let id = if is_g_or_use {
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
            filter_fill,
            filter_stroke,
            enable_background,
        }));

        GroupKind::Create(g)
    } else {
        GroupKind::Skip
    }
}

fn resolve_filter_fill(
    node: svgtree::Node,
    state: &State,
    filters: &[Rc<filter::Filter>],
    cache: &mut converter::Cache,
) -> Option<Paint> {
    let mut has_fill_paint = false;
    for filter in filters {
        if filter
            .primitives
            .iter()
            .any(|c| c.kind.has_input(&filter::Input::FillPaint))
        {
            has_fill_paint = true;
            break;
        }
    }

    if !has_fill_paint {
        return None;
    }

    let stroke = style::resolve_fill(node, true, state, cache)?;
    Some(stroke.paint)
}

fn resolve_filter_stroke(
    node: svgtree::Node,
    state: &State,
    filters: &[Rc<filter::Filter>],
    cache: &mut converter::Cache,
) -> Option<Paint> {
    let mut has_stroke_paint = false;
    for filter in filters {
        if filter
            .primitives
            .iter()
            .any(|c| c.kind.has_input(&filter::Input::StrokePaint))
        {
            has_stroke_paint = true;
            break;
        }
    }

    if !has_stroke_paint {
        return None;
    }

    let stroke = style::resolve_stroke(node, true, state, cache)?;
    Some(stroke.paint)
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

pub(crate) fn ungroup_groups(root: Node, keep_named_groups: bool) {
    fn ungroup(root: Node, parent: Node, keep_named_groups: bool) -> bool {
        let mut changed = false;

        let mut curr_node = parent.first_child();
        while let Some(node) = curr_node {
            curr_node = node.next_sibling();

            let mut ts = Transform::default();
            let is_ok = if let NodeKind::Group(ref g) = *node.borrow() {
                ts = g.transform;
                g.opacity == Opacity::ONE
                    && g.clip_path.is_none()
                    && g.mask.is_none()
                    && g.filters.is_empty()
                    && g.enable_background.is_none()
                    && g.blend_mode == BlendMode::Normal
                    && !g.isolate
                    && !(keep_named_groups && !g.id.is_empty())
            } else {
                false
            };

            if is_ok {
                let mut curr_child = node.last_child();
                while let Some(child) = curr_child {
                    curr_child = child.previous_sibling();

                    // Update transform.
                    match *child.borrow_mut() {
                        NodeKind::Path(ref mut path) => {
                            path.transform.prepend(&ts);
                        }
                        NodeKind::Image(ref mut img) => {
                            img.transform.prepend(&ts);
                        }
                        NodeKind::Group(ref mut g) => {
                            g.transform.prepend(&ts);
                        }
                        NodeKind::Text(ref mut text) => {
                            text.transform.prepend(&ts);
                        }
                    }

                    child.detach();
                    node.insert_after(child.clone());
                }

                node.detach();
                changed = true;
            } else {
                if ungroup(root.clone(), node, keep_named_groups) {
                    changed = true;
                }
            }
        }

        changed
    }

    while ungroup(root.clone(), root.clone(), keep_named_groups) {}
}

fn convert_path(
    node: svgtree::Node,
    path: SharedPathData,
    state: &State,
    cache: &mut converter::Cache,
    parent: &mut Node,
) {
    debug_assert!(path.len() >= 2);
    if path.len() < 2 {
        return;
    }

    let has_bbox = path.has_bbox();
    let fill = style::resolve_fill(node, has_bbox, state, cache);
    let stroke = style::resolve_stroke(node, has_bbox, state, cache);
    let mut visibility = node.find_attribute(AId::Visibility).unwrap_or_default();
    let rendering_mode = node
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
    if marker::is_valid(node) && visibility == Visibility::Visible {
        let mut g = parent.append_kind(NodeKind::Group(Group::default()));
        marker::convert(node, &path, state, cache, &mut g);
        markers_group = Some(g);
    }

    parent.append_kind(NodeKind::Path(Path {
        id: node.element_id().to_string(),
        transform: Default::default(),
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
