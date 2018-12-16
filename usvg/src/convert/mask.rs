// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom;

// self
use tree;
use super::prelude::*;


pub fn convert(
    node: &svgdom::Node,
    tree: &mut tree::Tree,
) -> Option<tree::Node> {
    let ref attrs = node.attributes();

    let rect = super::convert_rect(attrs);
    if !(rect.width > 0.0 && rect.height > 0.0) {
        warn!("Mask '{}' has an invalid size. Skipped.", node.id());
        return None;
    }

    let mut mask = None;
    if let Some(&AValue::FuncLink(ref link)) = attrs.get_type(AId::Mask) {
        if link.is_tag_name(EId::Mask) {
            mask = Some(link.id().to_string());
        }
    }

    Some(tree.append_to_defs(tree::NodeKind::Mask(tree::Mask {
        id: node.id().clone(),
        units: super::convert_element_units(attrs, AId::MaskUnits),
        content_units: super::convert_element_units(attrs, AId::MaskContentUnits),
        rect,
        mask,
    })))
}
