// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::rc::Rc;

use crate::{svgtree, tree, tree::prelude::*, utils};
use super::prelude::*;


pub fn convert(
    node: svgtree::Node,
    state: &State,
    parent: &mut tree::Node,
    tree: &mut tree::Tree,
) {
    let child = try_opt!(node.first_child());

    if state.parent_clip_path.is_some() && child.tag_name() == Some(EId::Symbol) {
        // Ignore `symbol` referenced by `use` inside a `clipPath`.
        // It will be ignored later anyway, but this will prevent
        // a redundant `clipPath` creation (which is required for `symbol`).
        return;
    }

    // We require an original transformation to setup 'clipPath'.
    let mut orig_ts: tree::Transform = node.attribute(AId::Transform).unwrap_or_default();
    let mut new_ts = tree::Transform::default();

    {
        let x = node.convert_user_length(AId::X, state, Length::zero());
        let y = node.convert_user_length(AId::Y, state, Length::zero());
        new_ts.translate(x, y);
    }

    let linked_to_symbol = child.tag_name() == Some(EId::Symbol);

    if linked_to_symbol {
        if let Some(ts) = viewbox_transform(node, child, state) {
            new_ts.append(&ts);
        }

        if let Some(clip_rect) = get_clip_rect(node, child, state) {
            let mut g = clip_element(node, clip_rect, orig_ts, parent, tree);
            convert_children(child, new_ts, state, &mut g, tree);
            return;
        }
    }

    orig_ts.append(&new_ts);

    if linked_to_symbol {
        convert_children(child, orig_ts, state, parent, tree);
    } else {
        convert_children(node, orig_ts, state, parent, tree);
    }
}

pub fn convert_svg(
    node: svgtree::Node,
    state: &State,
    parent: &mut tree::Node,
    tree: &mut tree::Tree,
) {
    // We require original transformation to setup 'clipPath'.
    let mut orig_ts: tree::Transform = node.attribute(AId::Transform).unwrap_or_default();
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
    node: svgtree::Node,
    clip_rect: Rect,
    transform: tree::Transform,
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
        data: Rc::new(tree::PathData::from_rect(clip_rect)),
        ..tree::Path::default()
    }));

    parent.append_kind(tree::NodeKind::Group(tree::Group {
        id: node.element_id().to_string(),
        transform,
        clip_path: Some(id),
        ..tree::Group::default()
    }))
}

fn convert_children(
    node: svgtree::Node,
    transform: tree::Transform,
    state: &State,
    parent: &mut tree::Node,
    tree: &mut tree::Tree,
) {
    let required = !transform.is_default();
    let mut parent = match super::convert_group(node, state, required, parent, tree) {
        super::GroupKind::Create(mut g) => {
            if let tree::NodeKind::Group(ref mut g) = *g.borrow_mut() {
                g.transform = transform;
            }

            g.clone()
        }
        super::GroupKind::Skip => {
            parent.clone()
        }
        super::GroupKind::Ignore => return,
    };

    if state.parent_clip_path.is_some() {
        super::convert_clip_path_elements(node, state, &mut parent, tree);
    } else {
        super::convert_children(node, state, &mut parent, tree);
    }
}

fn get_clip_rect(
    use_node: svgtree::Node,
    symbol_node: svgtree::Node,
    state: &State,
) -> Option<Rect> {
    // No need to clip elements with overflow:visible.
    if matches!(symbol_node.attribute(AId::Overflow), Some("visible") | Some("auto")) {
        return None;
    }

    let (x, y, w, h) = {
        let x = use_node.convert_user_length(AId::X, state, Length::zero());
        let y = use_node.convert_user_length(AId::Y, state, Length::zero());
        let w = use_node.convert_user_length(AId::Width, state, Length::new(100.0, Unit::Percent));
        let h = use_node.convert_user_length(AId::Height, state, Length::new(100.0, Unit::Percent));
        (x, y, w, h)
    };

    if !w.is_valid_length() || !h.is_valid_length() {
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
    node: svgtree::Node,
    tree: &tree::Tree,
) -> String {
    let mut idx = 1;
    let mut id = format!("clipPath{}", idx);
    while    node.document().descendants().any(|n| n.element_id() == id)
          || tree.defs().children().any(|n| *n.id() == id)
    {
        idx += 1;
        id = format!("clipPath{}", idx);
    }

    id
}

fn viewbox_transform(
    node: svgtree::Node,
    linked: svgtree::Node,
    state: &State,
) -> Option<tree::Transform> {
    let size = {
        let def = Length::new(100.0, Unit::Percent);
        let w = node.convert_user_length(AId::Width, state, def);
        let h = node.convert_user_length(AId::Height, state, def);
        Size::new(w, h)
    }?;

    let vb = linked.get_viewbox()?;
    let aspect = linked.attribute(AId::PreserveAspectRatio).unwrap_or_default();

    Some(utils::view_box_to_transform(vb, aspect, size))
}
