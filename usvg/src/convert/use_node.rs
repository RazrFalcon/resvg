// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom;

// self
use crate::tree;
use crate::utils;
use crate::tree::prelude::*;
use super::prelude::*;


pub fn convert(
    node: &svgdom::Node,
    state: &State,
    parent: &mut tree::Node,
    tree: &mut tree::Tree,
) {
    debug_assert!(node.has_attribute("usvg-use"));

    // We require original transformation to setup 'clipPath'.
    let mut orig_ts = node.attributes().get_transform(AId::Transform);
    let mut new_ts = tree::Transform::default();

    {
        let x = node.convert_user_length(AId::X, state, Length::zero());
        let y = node.convert_user_length(AId::Y, state, Length::zero());
        new_ts.translate(x, y);
    }

    if node.has_attribute("usvg-symbol") {
        let mut symbol = match node.attributes().get_value(AId::Href) {
            Some(&AValue::Link(ref link)) => link.clone(),
            _ => return,
        };

        debug_assert!(symbol.is_tag_name(EId::Symbol));

        node.copy_attribute_to(AId::Width, &mut symbol);
        node.copy_attribute_to(AId::Height, &mut symbol);
        if let Some(ts) = viewbox_transform(node, &symbol, state) {
            new_ts.append(&ts);
        }

        if let Some(clip_rect) = get_clip_rect(node, &symbol, state) {
            let mut g = clip_element(node, clip_rect, orig_ts, parent, tree);
            convert_children(node, new_ts, state, &mut g, tree);
            return;
        }
    }

    orig_ts.append(&new_ts);
    convert_children(node, orig_ts, state, parent, tree);
}

pub fn convert_svg(
    node: &svgdom::Node,
    state: &State,
    parent: &mut tree::Node,
    tree: &mut tree::Tree,
) {
    // We require original transformation to setup 'clipPath'.
    let mut orig_ts = node.attributes().get_transform(AId::Transform);
    let mut new_ts = tree::Transform::default();

    {
        let x = node.convert_user_length(AId::X, state, Length::zero());
        let y = node.convert_user_length(AId::Y, state, Length::zero());
        new_ts.translate(x, y);
    }

    if let Some(ts) = viewbox_transform(node, node, state) {
        new_ts.append(&ts);
    }

    if let Some(clip_rect) = get_clip_rect(node, node, state) {
        let mut g = clip_element(node, clip_rect, orig_ts, parent, tree);
        convert_children(node, new_ts, state, &mut g, tree);
    } else {
        orig_ts.append(&new_ts);
        convert_children(node, orig_ts, state, parent, tree);
    }
}

fn clip_element(
    node: &svgdom::Node,
    clip_rect: Rect,
    transform: svgdom::Transform,
    parent: &mut tree::Node,
    tree: &mut tree::Tree,
) -> tree::Node {
    // We can't set `clip-path` on the element itself,
    // because it will be affected by a possible transform.
    // So we have to create an additional group.

    // Emulate a new viewport via clipPath.
    //
    // From:
    // <defs/>
    // <elem/>
    //
    // To:
    // <defs>
    //   <clipPath id="clipPath1">
    //     <rect/>
    //   </clipPath>
    // </defs>
    // <g clip-path="ulr(#clipPath1)">
    //   <elem/>
    // </g>

    let id = gen_clip_path_id(node, tree);

    let mut clip_path = tree.append_to_defs(tree::NodeKind::ClipPath(tree::ClipPath {
        id: id.clone(),
        ..tree::ClipPath::default()
    }));

    clip_path.append_kind(tree::NodeKind::Path(tree::Path {
        fill: Some(tree::Fill::default()),
        segments: utils::rect_to_path(clip_rect),
        ..tree::Path::default()
    }));

    parent.append_kind(tree::NodeKind::Group(tree::Group {
        id: node.id().clone(),
        transform,
        clip_path: Some(id),
        ..tree::Group::default()
    }))
}

fn convert_children(
    node: &svgdom::Node,
    transform: svgdom::Transform,
    state: &State,
    parent: &mut tree::Node,
    tree: &mut tree::Tree,
) {
    let required = !transform.is_default();

    match super::convert_group(node, state, required, parent, tree) {
        super::GroupKind::Keep(mut g) => {
            if let tree::NodeKind::Group(ref mut g) = *g.borrow_mut() {
                g.transform = transform;
            }

            super::convert_children(node, state, &mut g, tree);
        }
        super::GroupKind::Skip => {
            super::convert_children(node, state, parent, tree);
        }
        super::GroupKind::Ignore => {}
    }
}

fn get_clip_rect(
    use_node: &svgdom::Node,
    symbol_node: &svgdom::Node,
    state: &State,
) -> Option<Rect> {
    // No need to clip elements with overflow:visible.
    {
        let attrs = symbol_node.attributes();
        let overflow = attrs.get_str_or(AId::Overflow, "hidden");
        if overflow != "hidden" && overflow != "scroll" {
            return None;
        }
    }

    let (x, y, w, h) = {
        let x = use_node.convert_user_length(AId::X, state, Length::zero());
        let y = use_node.convert_user_length(AId::Y, state, Length::zero());
        let w = use_node.convert_user_length(AId::Width, state, Length::new(100.0, Unit::Percent));
        let h = use_node.convert_user_length(AId::Height, state, Length::new(100.0, Unit::Percent));
        (x, y, w, h)
    };

    if w.is_fuzzy_zero() || h.is_fuzzy_zero() {
        return None;
    }

    // TODO: add a test case
    // Clip rect is not needed when it has the same size as a whole image.
    if w.fuzzy_eq(&state.size.width()) && h.fuzzy_eq(&state.size.height()) {
        return None;
    }

    Rect::new(x, y, w, h)
}

/// Creates a free id for `clipPath`.
pub fn gen_clip_path_id(
    node: &svgdom::Node,
    tree: &tree::Tree,
) -> String {
    // TODO: speed up

    let mut idx = 1;
    let mut id = format!("clipPath{}", idx);
    while    node.root().descendants().any(|n| *n.id() == id)
          || tree.defs().children().any(|n| *n.id() == id)
    {
        idx += 1;
        id = format!("clipPath{}", idx);
    }

    id
}

fn viewbox_transform(
    node: &svgdom::Node,
    linked: &svgdom::Node,
    state: &State,
) -> Option<svgdom::Transform> {
    let size = {
        let w = node.convert_user_length(AId::Width, state, Length::new(100.0, Unit::Percent));
        let h = node.convert_user_length(AId::Height, state, Length::new(100.0, Unit::Percent));
        Size::new(w, h)
    }?;

    let vb = linked.get_viewbox()?;
    let aspect = match linked.attributes().get_value(AId::PreserveAspectRatio) {
        Some(&AValue::AspectRatio(aspect)) => aspect,
        _ => svgdom::AspectRatio::default(),
    };

    Some(utils::view_box_to_transform(vb, aspect, size))
}
