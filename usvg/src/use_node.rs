// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::rc::Rc;

use svgtypes::{Length, LengthUnit};

use crate::svgtree::{self, EId, AId};
use crate::{converter, clippath, style, utils};
use crate::{Group, Node, NodeExt, NodeKind, Path, PathData, Tree};
use crate::geom::{FuzzyEq, IsValidLength, Rect, Size, Transform};

pub(crate) fn convert(
    node: svgtree::Node,
    state: &converter::State,
    id_generator: &mut converter::NodeIdGenerator,
    parent: &mut Node,
    tree: &mut Tree,
) -> Option<()> {
    let child = node.first_child()?;

    if state.parent_clip_path.is_some() && child.tag_name() == Some(EId::Symbol) {
        // Ignore `symbol` referenced by `use` inside a `clipPath`.
        // It will be ignored later anyway, but this will prevent
        // a redundant `clipPath` creation (which is required for `symbol`).
        return None;
    }

    // We require an original transformation to setup 'clipPath'.
    let mut orig_ts: Transform = node.attribute(AId::Transform).unwrap_or_default();
    let mut new_ts = Transform::default();

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
            let mut g = clip_element(node, clip_rect, orig_ts, id_generator, parent, tree);

            // Make group for `use`.
            let mut parent = match converter::convert_group(node, state, true, id_generator, &mut g, tree) {
                converter::GroupKind::Create(mut g) => {
                    // We must reset transform, because it was already set
                    // to the group with clip-path.
                    if let NodeKind::Group(ref mut g) = *g.borrow_mut() {
                        g.transform = Transform::default();
                    }

                    g
                }
                converter::GroupKind::Skip => g.clone(),
                converter::GroupKind::Ignore => return None,
            };

            convert_children(child, new_ts, state, id_generator, &mut parent, tree);
            return None;
        }
    }

    orig_ts.append(&new_ts);

    if linked_to_symbol {
        // Make group for `use`.
        let mut parent = match converter::convert_group(node, state, false, id_generator, parent, tree) {
            converter::GroupKind::Create(g) => g,
            converter::GroupKind::Skip => parent.clone(),
            converter::GroupKind::Ignore => return None,
        };

        convert_children(child, orig_ts, state, id_generator, &mut parent, tree);
    } else {
        convert_children(node, orig_ts, state, id_generator, parent, tree);
    }

    Some(())
}

pub(crate) fn convert_svg(
    node: svgtree::Node,
    state: &converter::State,
    id_generator: &mut converter::NodeIdGenerator,
    parent: &mut Node,
    tree: &mut Tree,
) {
    // We require original transformation to setup 'clipPath'.
    let mut orig_ts: Transform = node.attribute(AId::Transform).unwrap_or_default();
    let mut new_ts = Transform::default();

    {
        let x = node.convert_user_length(AId::X, state, Length::zero());
        let y = node.convert_user_length(AId::Y, state, Length::zero());
        new_ts.translate(x, y);
    }

    if let Some(ts) = viewbox_transform(node, node, state) {
        new_ts.append(&ts);
    }

    if let Some(clip_rect) = get_clip_rect(node, node, state) {
        let mut g = clip_element(node, clip_rect, orig_ts, id_generator, parent, tree);
        convert_children(node, new_ts, state, id_generator, &mut g, tree);
    } else {
        orig_ts.append(&new_ts);
        convert_children(node, orig_ts, state, id_generator, parent, tree);
    }
}

fn clip_element(
    node: svgtree::Node,
    clip_rect: Rect,
    transform: Transform,
    id_generator: &mut converter::NodeIdGenerator,
    parent: &mut Node,
    tree: &mut Tree,
) -> Node {
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

    let id = id_generator.gen_clip_path_id();

    let mut clip_path = tree.append_to_defs(NodeKind::ClipPath(clippath::ClipPath {
        id: id.clone(),
        ..clippath::ClipPath::default()
    }));

    clip_path.append_kind(NodeKind::Path(Path {
        fill: Some(style::Fill::default()),
        data: Rc::new(PathData::from_rect(clip_rect)),
        ..Path::default()
    }));

    parent.append_kind(NodeKind::Group(Group {
        id: node.element_id().to_string(),
        transform,
        clip_path: Some(id),
        ..Group::default()
    }))
}

fn convert_children(
    node: svgtree::Node,
    transform: Transform,
    state: &converter::State,
    id_generator: &mut converter::NodeIdGenerator,
    parent: &mut Node,
    tree: &mut Tree,
) {
    let required = !transform.is_default();
    let mut parent = match converter::convert_group(node, state, required, id_generator, parent, tree) {
        converter::GroupKind::Create(mut g) => {
            if let NodeKind::Group(ref mut g) = *g.borrow_mut() {
                g.transform = transform;
            }

            g
        }
        converter::GroupKind::Skip => {
            parent.clone()
        }
        converter::GroupKind::Ignore => return,
    };

    if state.parent_clip_path.is_some() {
        converter::convert_clip_path_elements(node, state, id_generator, &mut parent, tree);
    } else {
        converter::convert_children(node, state, id_generator, &mut parent, tree);
    }
}

fn get_clip_rect(
    use_node: svgtree::Node,
    symbol_node: svgtree::Node,
    state: &converter::State,
) -> Option<Rect> {
    // No need to clip elements with overflow:visible.
    if matches!(symbol_node.attribute(AId::Overflow), Some("visible") | Some("auto")) {
        return None;
    }

    let (x, y, w, h) = {
        let x = use_node.convert_user_length(AId::X, state, Length::zero());
        let y = use_node.convert_user_length(AId::Y, state, Length::zero());
        let w = use_node.convert_user_length(AId::Width, state, Length::new(100.0, LengthUnit::Percent));
        let h = use_node.convert_user_length(AId::Height, state, Length::new(100.0, LengthUnit::Percent));
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

fn viewbox_transform(
    node: svgtree::Node,
    linked: svgtree::Node,
    state: &converter::State,
) -> Option<Transform> {
    let size = {
        let def = Length::new(100.0, LengthUnit::Percent);
        let w = node.convert_user_length(AId::Width, state, def);
        let h = node.convert_user_length(AId::Height, state, def);
        Size::new(w, h)
    }?;

    let vb = linked.get_viewbox()?;
    let aspect = linked.attribute(AId::PreserveAspectRatio).unwrap_or_default();

    Some(utils::view_box_to_transform(vb, aspect, size))
}
