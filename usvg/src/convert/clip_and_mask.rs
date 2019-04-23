// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom;

// self
use crate::tree;
use super::prelude::*;


pub fn convert_clip(
    node: &svgdom::Node,
    state: &State,
    tree: &mut tree::Tree,
) -> Option<String> {
    if !node.is_tag_name(EId::ClipPath) {
        return None;
    }

    if !node.is_valid_transform(AId::Transform) {
        return None;
    }

    if tree.defs_by_id(node.id().as_str()).is_some() {
        return Some(node.id().clone());
    }

    let ref attrs = node.attributes();

    let mut clip_path = None;
    if let Some(&AValue::FuncLink(ref link)) = attrs.get_value(AId::ClipPath) {
        clip_path = convert_clip(link, state, tree);

        // Linked `clipPath` must be valid.
        if clip_path.is_none() {
            return None;
        }
    }

    let units = convert_element_units(attrs, AId::ClipPathUnits, tree::Units::UserSpaceOnUse);

    let mut clip = tree.append_to_defs(
        tree::NodeKind::ClipPath(tree::ClipPath {
            id: node.id().clone(),
            units,
            transform: attrs.get_transform(AId::Transform),
            clip_path,
        })
    );

    let mut clip_state = state.clone();
    clip_state.current_root = node.clone();
    super::convert_children(node, &clip_state, &mut clip, tree);

    if clip.has_children() {
        Some(node.id().clone())
    } else {
        clip.detach();
        None
    }
}

pub fn convert_mask(
    node: &svgdom::Node,
    state: &State,
    tree: &mut tree::Tree,
) -> Option<String> {
    if !node.is_tag_name(EId::Mask) {
        return None;
    }

    if tree.defs_by_id(node.id().as_str()).is_some() {
        return Some(node.id().clone());
    }

    let rect = Rect::new(
        node.convert_user_length(AId::X, state, Length::new(-10.0, Unit::Percent)),
        node.convert_user_length(AId::Y, state, Length::new(-10.0, Unit::Percent)),
        node.convert_user_length(AId::Width, state, Length::new(120.0, Unit::Percent)),
        node.convert_user_length(AId::Height, state, Length::new(120.0, Unit::Percent)),
    );
    let rect = try_opt_warn!(rect, None, "Mask '{}' has an invalid size. Skipped.", node.id());

    let ref attrs = node.attributes();

    let mut mask = None;
    if let Some(&AValue::FuncLink(ref link)) = attrs.get_value(AId::Mask) {
        mask = convert_mask(link, state, tree);

        // Linked `mask` must be valid.
        if mask.is_none() {
            return None;
        }
    }

    let units = convert_element_units(attrs, AId::MaskUnits,
                                      tree::Units::ObjectBoundingBox);
    let content_units = convert_element_units(attrs, AId::MaskContentUnits,
                                              tree::Units::UserSpaceOnUse);

    let mut mask = tree.append_to_defs(tree::NodeKind::Mask(tree::Mask {
        id: node.id().clone(),
        units,
        content_units,
        rect,
        mask,
    }));

    super::convert_children(node, state, &mut mask, tree);

    if mask.has_children() {
        Some(node.id().clone())
    } else {
        mask.detach();
        None
    }
}

fn convert_element_units(attrs: &svgdom::Attributes, aid: AId, def: tree::Units) -> tree::Units {
    match attrs.get_str(aid) {
        Some("userSpaceOnUse") => tree::Units::UserSpaceOnUse,
        Some("objectBoundingBox") => tree::Units::ObjectBoundingBox,
        _ => def,
    }
}
