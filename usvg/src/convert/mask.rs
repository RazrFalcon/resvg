// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::tree;
use crate::svgtree;
use super::prelude::*;


pub fn convert(
    node: svgtree::Node,
    state: &State,
    tree: &mut tree::Tree,
) -> Option<String> {
    // A `mask` attribute must reference a `mask` element.
    if !node.has_tag_name(EId::Mask) {
        return None;
    }

    // Check if this element was already converted.
    if let Some(id) = node.attribute(AId::Id) {
        if tree.defs_by_id(id).is_some() {
            return Some(id.to_string());
        }
    }

    let units = node.attribute(AId::MaskUnits).unwrap_or(tree::Units::ObjectBoundingBox);
    let content_units = node.attribute(AId::MaskContentUnits).unwrap_or(tree::Units::UserSpaceOnUse);

    let rect = Rect::new(
        node.convert_length(AId::X, units, state, Length::new(-10.0, Unit::Percent)),
        node.convert_length(AId::Y, units, state, Length::new(-10.0, Unit::Percent)),
        node.convert_length(AId::Width, units, state, Length::new(120.0, Unit::Percent)),
        node.convert_length(AId::Height, units, state, Length::new(120.0, Unit::Percent)),
    );
    let rect = try_opt_warn_or!(
        rect, None,
        "Mask '{}' has an invalid size. Skipped.", node.element_id(),
    );

    // Resolve linked mask.
    let mut mask = None;
    if let Some(link) = node.attribute::<svgtree::Node>(AId::Mask) {
        mask = convert(link, state, tree);

        // Linked `mask` must be valid.
        if mask.is_none() {
            return None;
        }
    }

    let mut mask = tree.append_to_defs(tree::NodeKind::Mask(tree::Mask {
        id: node.element_id().to_string(),
        units,
        content_units,
        rect,
        mask,
    }));

    super::convert_children(node, state, &mut mask, tree);

    if mask.has_children() {
        Some(node.element_id().to_string())
    } else {
        // A mask without children is invalid.
        mask.detach();
        None
    }
}

