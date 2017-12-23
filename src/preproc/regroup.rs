// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom::{
    Document,
    Node,
};

use svgdom::types::{
    FuzzyEq,
};

use short::{
    AId,
    EId,
};

use traits::{
    GetValue,
};


// TODO: images should not be grouped
pub fn regroup_elements(doc: &mut Document, parent: &Node) {
    let g_attrs = [AId::Mask, AId::ClipPath, AId::Filter, AId::Opacity];

    let mut ids = Vec::new();
    let mut curr_node = parent.first_child();
    while let Some(mut node) = curr_node {
        curr_node = node.next_sibling();
        ids.clear();

        if node.has_children() {
            regroup_elements(doc, &node);
        }

        if node.is_tag_name(EId::G) || node.is_tag_name(EId::Defs) {
            continue;
        }

        let opacity = node.attributes().get_number(AId::Opacity).unwrap_or(1.0);
        if opacity.fuzzy_eq(&1.0) && !node.has_attributes(&g_attrs) {
            continue;
        }

        if node.parents().any(|n| n.is_tag_name(EId::ClipPath)) {
            continue;
        }

        let mut g_node = doc.create_element(EId::G);

        {
            let attrs = node.attributes();
            for aid in &g_attrs {
                if let Some(attr) = attrs.get(*aid) {
                    g_node.set_attribute(attr.clone());
                    ids.push(*aid);
                }
            }

            if let Some(ts) = attrs.get(AId::Transform) {
                g_node.set_attribute(ts.clone());
                ids.push(AId::Transform);
            }
        }
        node.remove_attributes(&ids);

        node.insert_before(&g_node);
        node.detach();
        g_node.append(&node);
    }
}
