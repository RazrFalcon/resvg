// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#[cfg(feature = "text")]
use std::cell::RefCell;
#[cfg(feature = "text")]
use std::rc::Rc;

use svgtypes::Length;

use crate::{svgtree, tree, tree::prelude::*, Error};
#[cfg(feature = "text")]
use crate::fontdb;

mod clip_and_mask;
mod filter;
mod image;
mod marker;
mod paint_server;
mod shapes;
mod style;
mod switch;
mod units;
mod use_node;
#[cfg(feature = "text")] mod text;

mod prelude {
    pub use log::warn;
    pub use svgtypes::{FuzzyEq, FuzzyZero, Length};
    pub use crate::{geom::*, short::*, svgtree::{AId, EId}, Options, IsValidLength};
    pub use super::{SvgNodeExt, State};
}
use self::prelude::*;


#[derive(Clone)]
pub struct State<'a> {
    parent_clip_path: Option<svgtree::Node<'a>>,
    parent_marker: Option<svgtree::Node<'a>>,
    fe_image_link: bool,
    size: Size,
    view_box: Rect,
    #[cfg(feature = "text")]
    db: Rc<RefCell<fontdb::Database>>,
    opt: &'a Options,
}


/// Converts an input `Document` into a `Tree`.
///
/// # Errors
///
/// - If `Document` doesn't have an SVG node - returns an empty tree.
/// - If `Document` doesn't have a valid size - returns `Error::InvalidSize`.
pub fn convert_doc(
    svg_doc: &svgtree::Document,
    opt: &Options,
) -> Result<tree::Tree, Error> {
    let svg = svg_doc.root_element();
    let size = resolve_svg_size(&svg, opt)?;
    let view_box = tree::ViewBox {
        rect: svg.get_viewbox().unwrap_or(size.to_rect(0.0, 0.0)),
        aspect: svg.attribute(AId::PreserveAspectRatio).unwrap_or_default(),
    };

    let svg_kind = tree::Svg { size, view_box };
    let mut tree = tree::Tree::create(svg_kind);

    if !svg.is_visible_element(opt) {
        return Ok(tree);
    }

    let state = State {
        parent_clip_path: None,
        parent_marker: None,
        fe_image_link: false,
        size,
        view_box: view_box.rect,
        #[cfg(feature = "text")]
        db: Rc::new(RefCell::new(fontdb::Database::new())),
        opt: &opt,
    };

    convert_children(svg_doc.root(), &state, &mut tree.root(), &mut tree);

    link_fe_image(svg_doc, &state, &mut tree);
    remove_empty_groups(&mut tree);
    ungroup_groups(opt, &mut tree);
    remove_unused_defs(&mut tree);

    Ok(tree)
}

fn resolve_svg_size(
    svg: &svgtree::Node,
    opt: &Options,
) -> Result<Size, Error> {
    let mut state = State {
        parent_clip_path: None,
        parent_marker: None,
        fe_image_link: false,
        size: Size::new(100.0, 100.0).unwrap(),
        view_box: Rect::new(0.0, 0.0, 100.0, 100.0).unwrap(),
        #[cfg(feature = "text")]
        db: Rc::new(RefCell::new(fontdb::Database::new())),
        opt,
    };

    let def = Length::new(100.0, Unit::Percent);
    let width: Length = svg.attribute(AId::Width).unwrap_or(def);
    let height: Length = svg.attribute(AId::Height).unwrap_or(def);

    let view_box = svg.get_viewbox();

    if (width.unit == Unit::Percent || height.unit == Unit::Percent) && view_box.is_none() {
        // TODO: it this case we should detect the bounding box of all elements,
        //       which is currently impossible
        return Err(Error::InvalidSize);
    }

    let size = if let Some(vbox) = view_box {
        state.view_box = vbox;

        let w = if width.unit == Unit::Percent {
            vbox.width() * (width.num / 100.0)
        } else {
            svg.convert_user_length(AId::Width, &state, def)
        };

        let h = if height.unit == Unit::Percent {
            vbox.height() * (height.num / 100.0)
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

    size.ok_or_else(|| Error::InvalidSize)
}

#[inline(never)]
fn convert_children(
    parent_node: svgtree::Node,
    state: &State,
    parent: &mut tree::Node,
    tree: &mut tree::Tree,
) {
    for node in parent_node.children() {
        convert_element(node, state, parent, tree);
    }
}

#[inline(never)]
fn convert_element(
    node: svgtree::Node,
    state: &State,
    parent: &mut tree::Node,
    tree: &mut tree::Tree,
) {
    let tag_name = try_opt!(node.tag_name());

    if !tag_name.is_graphic() && !matches!(tag_name, EId::G | EId::Switch | EId::Svg) {
        return;
    }

    if !node.is_visible_element(state.opt) {
        return;
    }

    if tag_name == EId::Use {
        use_node::convert(node, state, parent, tree);
        return;
    }

    if tag_name == EId::Switch {
        switch::convert(node, state, parent, tree);
        return;
    }

    let parent = &mut match convert_group(node, state, false, parent, tree) {
        GroupKind::Create(g) => g,
        GroupKind::Skip => parent.clone(),
        GroupKind::Ignore => return,
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
                convert_path(node, path, state, parent, tree);
            }
        }
        EId::Image => {
            image::convert(node, state, parent);
        }
        EId::Text => {
            #[cfg(feature = "text")]
            text::convert(node, state, parent, tree);
        }
        EId::Svg => {
            if node.parent_element().is_some() {
                use_node::convert_svg(node, state, parent, tree);
            } else {
                // Skip root `svg`.
                convert_children(node, state, parent, tree);
            }
        }
        EId::G => {
            convert_children(node, state, parent, tree);
        }
        _ => {}
    }
}

// `clipPath` can have only shape and `text` children.
//
// `line` doesn't impact rendering because stroke is always disabled
// for `clipPath` children.
#[inline(never)]
fn convert_clip_path_elements(
    clip_node: svgtree::Node,
    state: &State,
    parent: &mut tree::Node,
    tree: &mut tree::Tree,
) {
    for node in clip_node.children() {
        let tag_name = try_opt!(node.tag_name());

        if !tag_name.is_graphic() {
            continue;
        }

        if !node.is_visible_element(state.opt) {
            continue;
        }

        if tag_name == EId::Use {
            use_node::convert(node, state, parent, tree);
            continue;
        }

        let parent = &mut match convert_group(node, state, false, parent, tree) {
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
                    convert_path(node, path, state, parent, tree);
                }
            }
            EId::Text => {
                #[cfg(feature = "text")]
                text::convert(node, state, parent, tree);
            }
            _ => {
                warn!("'{}' is no a valid 'clip-path' child.", tag_name);
            }
        }
    }
}

enum GroupKind {
    /// Creates a new group.
    Create(tree::Node),
    /// Skips an existing group, but processes its children.
    Skip,
    /// Skips an existing group and all its children.
    Ignore,
}

fn convert_group(
    node: svgtree::Node,
    state: &State,
    force: bool,
    parent: &mut tree::Node,
    tree: &mut tree::Tree,
) -> GroupKind {
    // A `clipPath` child cannot have an opacity.
    let opacity = if state.parent_clip_path.is_none() {
        node.attribute(AId::Opacity).unwrap_or_default()
    } else {
        tree::Opacity::default()
    };

    macro_rules! resolve_link {
        ($aid:expr, $f:expr) => {{
            let mut v = None;

            if let Some(link) = node.attribute::<svgtree::Node>($aid) {
                v = $f(link, state, tree);

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

    let clip_path = resolve_link!(AId::ClipPath, clip_and_mask::convert_clip);

    let mask = if state.parent_clip_path.is_none() {
        resolve_link!(AId::Mask, clip_and_mask::convert_mask)
    } else {
        None
    };


    let mut filter = None;
    if state.parent_clip_path.is_none() {
        if let Some(link) = node.attribute::<svgtree::Node>(AId::Filter) {
            filter = filter::convert(link, state, tree);

            // If `filter` is linked to an invalid element - skip this group completely.
            if filter.is_none() {
                return GroupKind::Ignore;
            }
        } else if node.attribute(AId::Filter) == Some("none") {
            // Do nothing.
        } else if node.has_attribute(AId::Filter) {
            // Unlike `clip-path` and `mask`, when `filter` is invalid
            // than the whole element should be removed.
            return GroupKind::Ignore;
        }
    }

    // TODO: move to `::deref` later.
    let filter_fill = resolve_filter_fill(node, state, filter.as_ref().map(|t| t.as_str()), tree);
    let filter_stroke = resolve_filter_stroke(node, state, filter.as_ref().map(|t| t.as_str()), tree);

    let transform: tree::Transform = node.attribute(AId::Transform).unwrap_or_default();

    let enable_background = node.attribute(AId::EnableBackground);

    let is_g_or_use = node.has_tag_name(EId::G) || node.has_tag_name(EId::Use);
    let required =
           opacity.value().fuzzy_ne(&1.0)
        || clip_path.is_some()
        || mask.is_some()
        || filter.is_some()
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

        let g = parent.append_kind(tree::NodeKind::Group(tree::Group {
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

fn resolve_filter_fill(
    node: svgtree::Node,
    state: &State,
    filter_id: Option<&str>,
    tree: &mut tree::Tree,
) -> Option<tree::Paint> {
    let filter_node = tree.defs_by_id(filter_id?)?;
    if let tree::NodeKind::Filter(ref filter) = *filter_node.borrow() {
        if !filter.children.iter().any(|c| c.kind.has_input(&tree::FilterInput::FillPaint)) {
            return None;
        }
    }

    let stroke = style::resolve_fill(node, true, state, tree)?;
    Some(stroke.paint)
}

fn resolve_filter_stroke(
    node: svgtree::Node,
    state: &State,
    filter_id: Option<&str>,
    tree: &mut tree::Tree,
) -> Option<tree::Paint> {
    let filter_node = tree.defs_by_id(filter_id?)?;
    if let tree::NodeKind::Filter(ref filter) = *filter_node.borrow() {
        if !filter.children.iter().any(|c| c.kind.has_input(&tree::FilterInput::StrokePaint)) {
            return None;
        }
    }

    let stroke = style::resolve_stroke(node, true, state, tree)?;
    Some(stroke.paint)
}

fn remove_empty_groups(tree: &mut tree::Tree) {
    fn rm(parent: tree::Node) -> bool {
        let mut changed = false;

        let mut curr_node = parent.first_child();
        while let Some(mut node) = curr_node {
            curr_node = node.next_sibling();

            let is_g = if let tree::NodeKind::Group(ref g) = *node.borrow() {
                // Skip empty groups when they do not have a `filter` property.
                // The `filter` property can be set on empty groups. For example:
                //
                // <filter id="filter1" filterUnits="userSpaceOnUse"
                //         x="20" y="20" width="160" height="160">
                //   <feFlood flood-color="green"/>
                // </filter>
                // <g filter="url(#filter1)"/>
                g.filter.is_none()
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
    opt: &Options,
    tree: &mut tree::Tree,
) {
    fn ungroup(parent: tree::Node, opt: &Options) -> bool {
        let mut changed = false;

        let mut curr_node = parent.first_child();
        while let Some(mut node) = curr_node {
            curr_node = node.next_sibling();

            let mut ts = tree::Transform::default();
            let is_ok = if let tree::NodeKind::Group(ref g) = *node.borrow() {
                ts = g.transform;

                   g.opacity.is_default()
                && g.clip_path.is_none()
                && g.mask.is_none()
                && g.filter.is_none()
                && g.enable_background.is_none()
                && !(opt.keep_named_groups && !g.id.is_empty())
                && !is_id_used(&parent.tree(), &g.id)
            } else {
                false
            };

            if is_ok {
                let mut curr_child = node.last_child();
                while let Some(mut child) = curr_child {
                    curr_child = child.previous_sibling();

                    // Update transform.
                    match *child.borrow_mut() {
                        tree::NodeKind::Path(ref mut path) => {
                            path.transform.prepend(&ts);
                        }
                        tree::NodeKind::Image(ref mut img) => {
                            img.transform.prepend(&ts);
                        }
                        tree::NodeKind::Group(ref mut g) => {
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
                if ungroup(node, opt) {
                    changed = true;
                }
            }
        }

        changed
    }

    while ungroup(tree.root(), opt) {}
}

fn remove_unused_defs(
    tree: &mut tree::Tree,
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

fn link_fe_image(
    svg_doc: &svgtree::Document,
    state: &State,
    tree: &mut tree::Tree,
) {
    let mut ids = Vec::new();
    // TODO: simplify
    for filter_node in tree.defs().children() {
        if let tree::NodeKind::Filter(ref filter) = *filter_node.borrow() {
            for fe in &filter.children {
                if let tree::FilterKind::FeImage(ref fe_img) = fe.kind {
                    if let tree::FeImageKind::Use(ref id) = fe_img.data {
                        if tree.defs_by_id(id).or(tree.node_by_id(id)).is_none() {
                            // If `feImage` references a non-existing element,
                            // create it in `defs`.
                            if svg_doc.element_by_id(id).is_some() {
                                ids.push(id.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    ids.sort();
    ids.dedup();

    // TODO: simplify
    for id in ids {
        if let Some(node) = svg_doc.element_by_id(&id) {
            let mut state = state.clone();
            state.fe_image_link = true;
            convert_element(node, &state, &mut tree.defs(), tree);

            // Check that node was actually created.
            // If not, reset to a dummy primitive.
            if !tree.defs().descendants().any(|n| *n.id() == id) {
                for mut filter_node in tree.defs().children() {
                    if let tree::NodeKind::Filter(ref mut filter) = *filter_node.borrow_mut() {
                        for fe in &mut filter.children {
                            fe.kind = filter::create_dummy_primitive();
                        }
                    }
                }
            }
        }
    }
}

fn is_id_used(tree: &tree::Tree, id: &str) -> bool {
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
                if let tree::Paint::Link(ref paint_id) = v.paint {
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
                if let tree::Paint::Link(ref paint_id) = v {
                    if $id == paint_id {
                        return true;
                    }
                }
            }
        };
    }

    for node in tree.root().descendants() {
        match *node.borrow() {
            tree::NodeKind::ClipPath(ref clip) => {
                check_id!(clip.clip_path, id);
            }
            tree::NodeKind::Mask(ref mask) => {
                check_id!(mask.mask, id);
            }
            tree::NodeKind::Path(ref path) => {
                check_paint_id!(path.fill, id);
                check_paint_id!(path.stroke, id);
            }
            tree::NodeKind::Group(ref g) => {
                check_id!(g.clip_path, id);
                check_id!(g.mask, id);
                check_id!(g.filter, id);
                check_paint_id2!(g.filter_fill, id);
                check_paint_id2!(g.filter_stroke, id);
            }
            tree::NodeKind::Filter(ref filter) => {
                for fe in &filter.children {
                    if let tree::FilterKind::FeImage(ref fe_img) = fe.kind {
                        if let tree::FeImageKind::Use(ref fe_id) = fe_img.data {
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
    path: tree::SharedPathData,
    state: &State,
    parent: &mut tree::Node,
    tree: &mut tree::Tree,
) {
    debug_assert!(path.len() >= 2);
    if path.len() < 2 {
        return;
    }

    let has_bbox = path.has_bbox();
    let fill = style::resolve_fill(node, has_bbox, state, tree);
    let stroke = style::resolve_stroke(node, has_bbox, state, tree);
    let mut visibility = node.find_attribute(AId::Visibility).unwrap_or_default();
    let rendering_mode = node
        .find_attribute(AId::ShapeRendering)
        .unwrap_or(state.opt.shape_rendering);

    // If a path doesn't have a fill or a stroke than it's invisible.
    // By setting `visibility` to `hidden` we are disabling the rendering of this path.
    if fill.is_none() && stroke.is_none() {
        visibility = tree::Visibility::Hidden;
    }

    let mut markers_group = None;
    if marker::is_valid(node) && visibility == tree::Visibility::Visible {
        let mut g = parent.append_kind(tree::NodeKind::Group(tree::Group::default()));
        marker::convert(node, &path, state, &mut g, tree);
        markers_group = Some(g);
    }

    parent.append_kind(tree::NodeKind::Path(tree::Path {
        id: node.element_id().to_string(),
        transform: Default::default(),
        visibility,
        fill,
        stroke,
        rendering_mode,
        data: path,
    }));

    // Insert markers group after `path`.
    if let Some(mut g) = markers_group {
        g.detach();
        parent.append(g);
    }
}


pub trait SvgNodeExt {
    fn resolve_length(&self, aid: AId, state: &State, def: f64) -> f64;
    fn resolve_valid_length(&self, aid: AId, state: &State, def: f64) -> Option<f64>;
    fn convert_length(&self, aid: AId, object_units: tree::Units, state: &State, def: Length) -> f64;
    fn try_convert_length(&self, aid: AId, object_units: tree::Units, state: &State) -> Option<f64>;
    fn convert_user_length(&self, aid: AId, state: &State, def: Length) -> f64;
    fn try_convert_user_length(&self, aid: AId, state: &State) -> Option<f64>;
    fn is_visible_element(&self, opt: &Options) -> bool;
}

impl<'a> SvgNodeExt for svgtree::Node<'a> {
    fn resolve_length(&self, aid: AId, state: &State, def: f64) -> f64 {
        debug_assert!(!matches!(aid, AId::BaselineShift | AId::FontSize),
                      "{} cannot be resolved via this function", aid);

        if let Some(n) = self.find_node_with_attribute(aid) {
            if let Some(length) = n.attribute(aid) {
                return units::convert_length(length, n, aid, tree::Units::UserSpaceOnUse, state);
            }
        }

        def
    }

    fn resolve_valid_length(&self, aid: AId, state: &State, def: f64) -> Option<f64> {
        let n = self.resolve_length(aid, state, def);
        if n.is_valid_length() { Some(n) } else { None }
    }

    fn convert_length(&self, aid: AId, object_units: tree::Units, state: &State, def: Length) -> f64 {
        units::convert_length(self.attribute(aid).unwrap_or(def), *self, aid, object_units, state)
    }

    fn try_convert_length(&self, aid: AId, object_units: tree::Units, state: &State) -> Option<f64> {
        Some(units::convert_length(self.attribute(aid)?, *self, aid, object_units, state))
    }

    fn convert_user_length(&self, aid: AId, state: &State, def: Length) -> f64 {
        self.convert_length(aid, tree::Units::UserSpaceOnUse, state, def)
    }

    fn try_convert_user_length(&self, aid: AId, state: &State) -> Option<f64> {
        self.try_convert_length(aid, tree::Units::UserSpaceOnUse, state)
    }

    fn is_visible_element(&self, opt: &Options) -> bool {
           self.attribute(AId::Display) != Some("none")
        && self.has_valid_transform(AId::Transform)
        && switch::is_condition_passed(*self, opt)
    }
}
