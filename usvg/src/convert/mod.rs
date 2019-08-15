// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::cell::RefCell;
use std::rc::Rc;

use svgtypes::Length;

use crate::{svgtree, tree, tree::prelude::*, fontdb, Error};

mod clip_and_mask;
mod filter;
mod image;
mod marker;
mod paint_server;
mod shapes;
mod style;
mod switch;
mod text;
mod units;
mod use_node;

mod prelude {
    pub use log::warn;
    pub use svgtypes::{FuzzyEq, FuzzyZero, Length};
    pub use crate::{geom::*, short::*, svgtree::{AId, EId}, Options};
    pub use super::{SvgNodeExt, State};
}
use self::prelude::*;


#[derive(Clone)]
pub struct State<'a> {
    parent_clip_path: Option<svgtree::Node<'a>>,
    parent_marker: Option<svgtree::Node<'a>>,
    size: Size,
    view_box: Rect,
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

    let view_box = {
        tree::ViewBox {
            rect: get_view_box(&svg, size),
            aspect: svg.attribute(AId::PreserveAspectRatio).unwrap_or_default(),
        }
    };

    if !style::is_visible_element(svg, opt) {
        let svg_kind = tree::Svg {
            size,
            view_box,
        };

        return Ok(tree::Tree::create(svg_kind));
    }

    let svg_kind = tree::Svg {
        size,
        view_box,
    };

    let state = State {
        parent_clip_path: None,
        parent_marker: None,
        size,
        view_box: view_box.rect,
        db: Rc::new(RefCell::new(fontdb::Database::new())),
        opt: &opt,
    };

    let mut tree = tree::Tree::create(svg_kind);

    convert_children(svg_doc.root(), &state, &mut tree.root(), &mut tree);

    remove_empty_groups(&mut tree);
    ungroup_groups(&mut tree, opt);
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
        size: Size::new(100.0, 100.0).unwrap(),
        view_box: Rect::new(0.0, 0.0, 100.0, 100.0).unwrap(),
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

    if let Some(size) = size {
        Ok(size)
    } else {
        Err(Error::InvalidSize)
    }
}

fn get_view_box(
    svg: &svgtree::Node,
    size: Size,
) -> Rect {
    match svg.get_viewbox() {
        Some(vb) => vb,
        None => size.to_rect(0.0, 0.0),
    }
}

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

fn convert_element(
    node: svgtree::Node,
    state: &State,
    parent: &mut tree::Node,
    tree: &mut tree::Tree,
) {
    let tag_name = try_opt!(node.tag_name());

    let is_valid_child =    tag_name.is_graphic()
                         || tag_name == EId::G
                         || tag_name == EId::Switch
                         || tag_name == EId::Svg;
    if !is_valid_child {
        return;
    }

    if !style::is_visible_element(node, state.opt) {
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

        if !style::is_visible_element(node, state.opt) {
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

    let transform: tree::Transform = node.attribute(AId::Transform).unwrap_or_default();

    let required =    opacity.value().fuzzy_ne(&1.0)
                   || clip_path.is_some()
                   || mask.is_some()
                   || filter.is_some()
                   || !transform.is_default()
                   || (node.has_tag_name(EId::G) && state.opt.keep_named_groups)
                   || force;

    if required {
        let id = if node.has_tag_name(EId::G) {
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
        }));

        GroupKind::Create(g)
    } else {
        GroupKind::Skip
    }
}

fn remove_empty_groups(
    tree: &mut tree::Tree,
) {
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
    tree: &mut tree::Tree,
    opt: &Options,
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
                && !(opt.keep_named_groups && !g.id.is_empty())
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

    fn is_used(tree: &tree::Tree, id: &str) -> bool {
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
                }
                _ => {}
            }
        }

        false
    }

    let mut is_changed = true;
    while is_changed {
        is_changed = false;

        let mut curr_node = tree.defs().first_child();
        while let Some(mut node) = curr_node {
            curr_node = node.next_sibling();

            if !is_used(tree, node.id().as_ref()) {
                node.detach();
                is_changed = true;
            }
        }
    }
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
        visibility = tree::Visibility::Hidden
    }

    let mut markers_group = None;
    if visibility == tree::Visibility::Visible {
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
    fn convert_length(&self, aid: AId, object_units: tree::Units, state: &State, def: Length) -> f64;
    fn try_convert_length(&self, aid: AId, object_units: tree::Units, state: &State) -> Option<f64>;
    fn convert_user_length(&self, aid: AId, state: &State, def: Length) -> f64;
    fn try_convert_user_length(&self, aid: AId, state: &State) -> Option<f64>;
}

impl<'a> SvgNodeExt for svgtree::Node<'a> {
    fn resolve_length(&self, aid: AId, state: &State, def: f64) -> f64 {
        let is_inheritable = match aid {
              AId::BaselineShift
            | AId::FontSize => false,
            _ => true,
        };

        debug_assert!(is_inheritable);

        if let Some(n) = self.ancestors().find(|n| n.has_attribute(aid)) {
            if let Some(length) = n.attribute(aid) {
                return units::convert_length(length, n, aid, tree::Units::UserSpaceOnUse, state);
            }
        }

        def
    }

    fn convert_length(&self, aid: AId, object_units: tree::Units, state: &State, def: Length) -> f64 {
        let length = self.attribute(aid).unwrap_or(def);
        units::convert_length(length, *self, aid, object_units, state)
    }

    fn try_convert_length(&self, aid: AId, object_units: tree::Units, state: &State) -> Option<f64> {
        let length = self.attribute(aid)?;
        Some(units::convert_length(length, *self, aid, object_units, state))
    }

    fn convert_user_length(&self, aid: AId, state: &State, def: Length) -> f64 {
        self.convert_length(aid, tree::Units::UserSpaceOnUse, state, def)
    }

    fn try_convert_user_length(&self, aid: AId, state: &State) -> Option<f64> {
        self.try_convert_length(aid, tree::Units::UserSpaceOnUse, state)
    }
}
