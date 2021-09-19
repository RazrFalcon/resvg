// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use svgtypes::{Length, LengthUnit as Unit};

use crate::svgtree::{self, AId, EId};
use crate::*;

#[derive(Clone)]
pub struct State<'a> {
    pub(crate) parent_clip_path: Option<svgtree::Node<'a>>,
    pub(crate) parent_marker: Option<svgtree::Node<'a>>,
    pub(crate) fe_image_link: bool,
    pub(crate) size: Size,
    pub(crate) view_box: Rect,
    pub(crate) opt: &'a OptionsRef<'a>,
}

pub struct NodeIdGenerator {
    all_ids: HashSet<u64>,
    clip_path_index: usize,
    #[allow(dead_code)] filter_index: usize,
}

impl NodeIdGenerator {
    fn new(doc: &svgtree::Document) -> Self {
        let mut all_ids = HashSet::new();
        for node in doc.descendants() {
            if node.has_element_id() {
                all_ids.insert(string_hash(node.element_id()));
            }
        }

        NodeIdGenerator {
            all_ids,
            clip_path_index: 0,
            filter_index: 0,
        }
    }

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

    #[cfg(feature = "filter")]
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
pub(crate) fn convert_doc(
    svg_doc: &svgtree::Document,
    opt: &OptionsRef,
) -> Result<Tree, Error> {
    let svg = svg_doc.root_element();
    let (size, restore_viewbox) = resolve_svg_size(&svg, opt);
    let size = size?;
    let view_box = ViewBox {
        rect: svg.get_viewbox().unwrap_or_else(|| size.to_rect(0.0, 0.0)),
        aspect: svg.attribute(AId::PreserveAspectRatio).unwrap_or_default(),
    };

    let svg_kind = Svg { size, view_box };
    let mut tree = Tree::create(svg_kind);

    if !svg.is_visible_element(opt) {
        return Ok(tree);
    }

    let state = State {
        parent_clip_path: None,
        parent_marker: None,
        fe_image_link: false,
        size,
        view_box: view_box.rect,
        opt,
    };

    let mut id_generator = NodeIdGenerator::new(svg_doc);

    convert_children(svg_doc.root(), &state, &mut id_generator, &mut tree.root(), &mut tree);

    // The `convert_children` method doesn't convert elements inside `defs`
    // and non-graphic elements (like gradients, patters, filters, etc.).
    // Those elements are only converted when referenced.
    // For example, a gradient can be referenced by a rect's fill property.
    // This way we're automatically ignoring unused elements.
    //
    // But since `convert_children` processes elements in a linear order,
    // `feImage` can reference an element that was not converted yet.
    // In which case we have to process this `feImage` afterwards.
    //
    // And since `link_fe_image` processes only direct `feImage` links,
    // we have to run it until there are no more links left.
    // For example, when `feImage` references an element that also uses `feImage`,
    // we have to run this methods twice. And so on.
    #[cfg(feature = "filter")] {
        while link_fe_image(svg_doc, &state, &mut id_generator, &mut tree) {}
    }

    remove_empty_groups(&mut tree);
    ungroup_groups(opt, &mut tree);
    remove_unused_defs(&mut tree);

    if restore_viewbox {
        let mut right = 0.0;
        let mut bottom = 0.0;

        for node in tree.root().descendants() {
            if !tree.is_in_defs(&node) {
                if let Some(bbox) = node.calculate_bbox() {
                    if bbox.right() > right {
                        right = bbox.right();
                    }
                    if bbox.bottom() > bottom {
                        bottom = bbox.bottom();
                    }
                }
            }
        }

        if let Some(rect) = Rect::new(0.0, 0.0, right, bottom) {
            tree.set_view_box(rect);
        }

        tree.set_dimensions(right, bottom);
    }

    Ok(tree)
}

fn resolve_svg_size(
    svg: &svgtree::Node,
    opt: &OptionsRef,
) -> (Result<Size, Error>, bool) {
    let mut state = State {
        parent_clip_path: None,
        parent_marker: None,
        fe_image_link: false,
        size: Size::new(100.0, 100.0).unwrap(),
        view_box: Rect::new(0.0, 0.0, 100.0, 100.0).unwrap(),
        opt,
    };

    let def = Length::new(100.0, Unit::Percent);
    let mut width: Length = svg.attribute(AId::Width).unwrap_or(def);
    let mut height: Length = svg.attribute(AId::Height).unwrap_or(def);

    let view_box = svg.get_viewbox();

    let restore_viewbox = if (width.unit == Unit::Percent || height.unit == Unit::Percent) && view_box.is_none() {
        // Apply the percentages to the fallback size.
        if width.unit == Unit::Percent {
            width = Length::new((width.number / 100.0) * state.opt.default_size.width(), Unit::None);
        }

        if height.unit == Unit::Percent {
            height = Length::new((height.number / 100.0) * state.opt.default_size.height(), Unit::None);
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

#[inline(never)]
pub(crate) fn convert_children(
    parent_node: svgtree::Node,
    state: &State,
    id_generator: &mut NodeIdGenerator,
    parent: &mut Node,
    tree: &mut Tree,
) {
    for node in parent_node.children() {
        convert_element(node, state, id_generator, parent, tree);
    }
}

#[inline(never)]
pub(crate) fn convert_element(
    node: svgtree::Node,
    state: &State,
    id_generator: &mut NodeIdGenerator,
    parent: &mut Node,
    tree: &mut Tree,
) -> Option<Node> {
    let tag_name = node.tag_name()?;

    if !tag_name.is_graphic() && !matches!(tag_name, EId::G | EId::Switch | EId::Svg) {
        return None;
    }

    if !node.is_visible_element(state.opt) {
        return None;
    }

    if tag_name == EId::Use {
        use_node::convert(node, state, id_generator, parent, tree);
        return None;
    }

    if tag_name == EId::Switch {
        switch::convert(node, state, id_generator, parent, tree);
        return None;
    }

    let parent = &mut match convert_group(node, state, false, id_generator, parent, tree) {
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
                convert_path(node, path, state, id_generator, parent, tree);
            }
        }
        EId::Image => {
            image::convert(node, state, parent);
        }
        EId::Text => {
            #[cfg(feature = "text")]
            if !state.opt.fontdb.is_empty() {
                text::convert(node, state, id_generator, parent, tree);
            }
        }
        EId::Svg => {
            if node.parent_element().is_some() {
                use_node::convert_svg(node, state, id_generator, parent, tree);
            } else {
                // Skip root `svg`.
                convert_children(node, state, id_generator, parent, tree);
            }
        }
        EId::G => {
            convert_children(node, state, id_generator, parent, tree);
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
    id_generator: &mut NodeIdGenerator,
    parent: &mut Node,
    tree: &mut Tree,
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
            use_node::convert(node, state, id_generator, parent, tree);
            continue;
        }

        let parent = &mut match convert_group(node, state, false, id_generator, parent, tree) {
            GroupKind::Create(g) => g,
            GroupKind::Skip => parent.clone(),
            GroupKind::Ignore => continue,
        };

        match tag_name {
              EId::Rect
            | EId::Circle
            | EId::Ellipse
            | EId::Polyline
            | EId::Polygon
            | EId::Path => {
                if let Some(path) = shapes::convert(node, state) {
                    convert_path(node, path, state, id_generator, parent, tree);
                }
            }
            EId::Text => {
                #[cfg(feature = "text")]
                if !state.opt.fontdb.is_empty() {
                    text::convert(node, state, id_generator, parent, tree);
                }
            }
            _ => {
                log::warn!("'{}' is no a valid 'clip-path' child.", tag_name);
            }
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

pub(crate) fn convert_group(
    node: svgtree::Node,
    state: &State,
    force: bool,
    id_generator: &mut NodeIdGenerator,
    parent: &mut Node,
    tree: &mut Tree,
) -> GroupKind {
    // A `clipPath` child cannot have an opacity.
    let opacity = if state.parent_clip_path.is_none() {
        node.attribute(AId::Opacity).unwrap_or_default()
    } else {
        Opacity::default()
    };

    macro_rules! resolve_link {
        ($aid:expr, $f:expr) => {{
            let mut v = None;

            if let Some(link) = node.attribute::<svgtree::Node>($aid) {
                v = $f(link, state, id_generator, tree);

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

    #[allow(unused_mut)]
    let mut filter = Vec::new();
    #[cfg(feature = "filter")]
    if state.parent_clip_path.is_none() {
        if node.attribute(AId::Filter) == Some("none") {
            // Do nothing.
        } else if node.has_attribute(AId::Filter) {
            if let Ok(id) = filter::convert(node, state, id_generator, tree) {
                filter = id;
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

    #[cfg(feature = "filter")]
    let filter_fill = resolve_filter_fill(node, state, &filter, id_generator, tree);
    #[cfg(feature = "filter")]
    let filter_stroke = resolve_filter_stroke(node, state, &filter, id_generator, tree);

    #[cfg(not(feature = "filter"))]
    let filter_fill = None;
    #[cfg(not(feature = "filter"))]
    let filter_stroke = None;

    let transform: Transform = node.attribute(AId::Transform).unwrap_or_default();

    let enable_background = node.attribute(AId::EnableBackground);

    let is_g_or_use = node.has_tag_name(EId::G) || node.has_tag_name(EId::Use);
    let required =
           opacity.value().fuzzy_ne(&1.0)
        || clip_path.is_some()
        || mask.is_some()
        || !filter.is_empty()
        || !transform.is_default()
        || enable_background.is_some()
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
            clip_path,
            mask,
            filter,
            filter_fill,
            filter_stroke,
            enable_background,
        }));

        GroupKind::Create(g)
    } else {
        GroupKind::Skip
    }
}

#[cfg(feature = "filter")]
fn resolve_filter_fill(
    node: svgtree::Node,
    state: &State,
    filter_id: &[String],
    id_generator: &mut NodeIdGenerator,
    tree: &mut  Tree,
) -> Option<Paint> {
    let mut has_fill_paint = false;
    for id in filter_id {
        if let Some(filter_node) = tree.defs_by_id(id) {
            if let NodeKind::Filter(ref filter) = *filter_node.borrow() {
                if filter.primitives.iter().any(|c| c.kind.has_input(&filter::Input::FillPaint)) {
                    has_fill_paint = true;
                    break;
                }
            }
        }
    }

    if !has_fill_paint {
        return None;
    }

    let stroke = style::resolve_fill(node, true, state, id_generator, tree)?;
    Some(stroke.paint)
}

#[cfg(feature = "filter")]
fn resolve_filter_stroke(
    node: svgtree::Node,
    state: &State,
    filter_id: &[String],
    id_generator: &mut NodeIdGenerator,
    tree: &mut  Tree,
) -> Option<Paint> {
    let mut has_fill_paint = false;
    for id in filter_id {
        if let Some(filter_node) = tree.defs_by_id(id) {
            if let NodeKind::Filter(ref filter) = *filter_node.borrow() {
                if filter.primitives.iter().any(|c| c.kind.has_input(&filter::Input::StrokePaint)) {
                    has_fill_paint = true;
                    break;
                }
            }
        }
    }

    if !has_fill_paint {
        return None;
    }

    let stroke = style::resolve_stroke(node, true, state, id_generator, tree)?;
    Some(stroke.paint)
}

fn remove_empty_groups(tree: &mut  Tree) {
    fn rm(parent: Node) -> bool {
        let mut changed = false;

        let mut curr_node = parent.first_child();
        while let Some(mut node) = curr_node {
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
                g.filter.is_empty()
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

    while rm(tree.root()) {}
}

fn ungroup_groups(
    opt: &OptionsRef,
    tree: &mut  Tree,
) {
    fn ungroup(tree: & Tree, parent: Node, opt: &OptionsRef) -> bool {
        let mut changed = false;

        let mut curr_node = parent.first_child();
        while let Some(mut node) = curr_node {
            curr_node = node.next_sibling();

            let mut ts = Transform::default();
            let is_ok = if let NodeKind::Group(ref g) = *node.borrow() {
                ts = g.transform;

                   g.opacity.is_default()
                && g.clip_path.is_none()
                && g.mask.is_none()
                && g.filter.is_empty()
                && g.enable_background.is_none()
                && !(opt.keep_named_groups && !g.id.is_empty())
                && !is_id_used(tree, &g.id)
            } else {
                false
            };

            if is_ok {
                let mut curr_child = node.last_child();
                while let Some(mut child) = curr_child {
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
                        _ => {}
                    }

                    child.detach();
                    node.insert_after(child.clone());
                }

                node.detach();
                changed = true;
            } else {
                if ungroup(tree, node, opt) {
                    changed = true;
                }
            }
        }

        changed
    }

    while ungroup(tree, tree.root(), opt) {}
}

fn remove_unused_defs(
    tree: &mut  Tree,
) {
    let mut is_changed = true;
    while is_changed {
        is_changed = false;

        let mut curr_node = tree.defs().first_child();
        while let Some(mut node) = curr_node {
            curr_node = node.next_sibling();

            if !is_id_used(tree, node.id().as_ref()) {
                node.detach();
                is_changed = true;
            }
        }
    }
}

#[cfg(feature = "filter")]
fn link_fe_image(
    svg_doc: &svgtree::Document,
    state: &State,
    id_generator: &mut NodeIdGenerator,
    tree: &mut  Tree,
) -> bool {
    let mut ids = Vec::new();
    // TODO: simplify
    for filter_node in tree.defs().children() {
        if let NodeKind::Filter(ref filter) = *filter_node.borrow() {
            for fe in &filter.primitives {
                if let filter::Kind::Image(ref fe_img) = fe.kind {
                    if let filter::ImageKind::Use(ref id) = fe_img.data {
                        if tree.defs_by_id(id).or_else(|| tree.node_by_id(id)).is_none() {
                            // If `feImage` references a non-existing element,
                            // create it in `defs`.
                            if svg_doc.element_by_id(id).is_some() {
                                ids.push((filter_node.id().to_string(), id.to_string()));
                            }
                        }
                    }
                }
            }
        }
    }

    ids.dedup_by(|a, b| a.1 == b.1);

    // TODO: simplify
    let mut has_resolved = false;
    for (filter_id, id) in ids {
        if let Some(node) = svg_doc.element_by_id(&id) {
            let mut state = state.clone();
            state.fe_image_link = true;
            let mut new_node = match convert_element(node, &state, id_generator, &mut tree.defs(), tree) {
                Some(n) => n,
                None => continue,
            };

            // `convert_element` can create a subgroup in some cases, which is not what we need.
            // In this case we should move child element's id to the group,
            // so `feImage` would reference the whole group and not just a child.
            if new_node != tree.defs() {
                if let NodeKind::Group(ref mut g) = *new_node.borrow_mut() {
                    g.id = id.clone();
                }

                // Remove ids from children.
                for mut n in new_node.children() {
                    match *n.borrow_mut() {
                        NodeKind::Path(ref mut p) => p.id.clear(),
                        NodeKind::Image(ref mut p) => p.id.clear(),
                        _ => {}
                    }
                }
            }

            // Make sure the new element doesn't reference the current filter.
            if let NodeKind::Group(ref mut g) = *new_node.borrow_mut() {
                if g.filter.iter().any(|id| *id == filter_id) {
                    log::warn!(
                        "Recursive 'feImage' detected. \
                         The 'filter' attribute will be removed from '{}'.",
                        id
                    );

                    g.filter = Vec::new();
                }
            }

            // Check that node was actually created.
            // If not, reset to a dummy primitive.
            if !tree.defs().children().any(|n| *n.id() == id) {
                for mut filter_node in tree.defs().children() {
                    if let NodeKind::Filter(ref mut filter) = *filter_node.borrow_mut() {
                        for fe in &mut filter.primitives {
                            fe.kind = filter::create_dummy_primitive();
                        }
                    }
                }
            }

            has_resolved = true;
        }
    }

    has_resolved
}

fn is_id_used(tree: & Tree, id: &str) -> bool {
    macro_rules! check_id {
        ($from:expr, $id:expr) => {
            if let Some(ref id) = $from {
                if $id == id {
                    return true;
                }
            }
        };
    }

    macro_rules! check_paint_id {
        ($from:expr, $id:expr) => {
            if let Some(ref v) = $from {
                if let Paint::Link(ref paint_id) = v.paint {
                    if $id == paint_id {
                        return true;
                    }
                }
            }
        };
    }

    macro_rules! check_paint_id2 {
        ($from:expr, $id:expr) => {
            if let Some(ref v) = $from {
                if let Paint::Link(ref paint_id) = v {
                    if $id == paint_id {
                        return true;
                    }
                }
            }
        };
    }

    for node in tree.root().descendants() {
        match *node.borrow() {
            NodeKind::ClipPath(ref clip) => {
                check_id!(clip.clip_path, id);
            }
            NodeKind::Mask(ref mask) => {
                check_id!(mask.mask, id);
            }
            NodeKind::Path(ref path) => {
                check_paint_id!(path.fill, id);
                check_paint_id!(path.stroke, id);
            }
            NodeKind::Group(ref g) => {
                check_id!(g.clip_path, id);
                check_id!(g.mask, id);
                check_paint_id2!(g.filter_fill, id);
                check_paint_id2!(g.filter_stroke, id);

                if g.filter.iter().any(|v| v == id) {
                    return true;
                }
            }
            #[cfg(feature = "filter")]
            NodeKind::Filter(ref filter) => {
                for fe in &filter.primitives {
                    if let filter::Kind::Image(ref fe_img) = fe.kind {
                        if let filter::ImageKind::Use(ref fe_id) = fe_img.data {
                            if fe_id == id {
                                return true;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    false
}

fn convert_path(
    node: svgtree::Node,
    path: SharedPathData,
    state: &State,
    id_generator: &mut NodeIdGenerator,
    parent: &mut Node,
    tree: &mut  Tree,
) {
    debug_assert!(path.len() >= 2);
    if path.len() < 2 {
        return;
    }

    let has_bbox = path.has_bbox();
    let fill = style::resolve_fill(node, has_bbox, state, id_generator, tree);
    let stroke = style::resolve_stroke(node, has_bbox, state, id_generator, tree);
    let mut visibility = node.find_attribute(AId::Visibility).unwrap_or_default();
    let rendering_mode = node
        .find_attribute(AId::ShapeRendering)
        .unwrap_or(state.opt.shape_rendering);

    // If a path doesn't have a fill or a stroke than it's invisible.
    // By setting `visibility` to `hidden` we are disabling rendering of this path.
    if fill.is_none() && stroke.is_none() {
        visibility = Visibility::Hidden;
    }

    let mut markers_group = None;
    if marker::is_valid(node) && visibility == Visibility::Visible {
        let mut g = parent.append_kind(NodeKind::Group(Group::default()));
        marker::convert(node, &path, state, id_generator, &mut g, tree);
        markers_group = Some(g);
    }

    parent.append_kind(NodeKind::Path(Path {
        id: node.element_id().to_string(),
        transform: Default::default(),
        visibility,
        fill,
        stroke,
        rendering_mode,
        text_bbox: None,
        data: path,
    }));

    // Insert markers group after `path`.
    if let Some(mut g) = markers_group {
        g.detach();
        parent.append(g);
    }
}
