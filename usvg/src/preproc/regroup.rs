// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use super::prelude::*;


pub fn regroup_elements(doc: &mut Document, parent: &Node) {
    let g_attrs = [AId::ClipPath, AId::Mask, AId::Filter, AId::Opacity];

    let mut ids = Vec::new();
    let mut curr_node = parent.first_child();
    while let Some(mut node) = curr_node {
        curr_node = node.next_sibling();
        ids.clear();

        if node.has_children() {
            regroup_elements(doc, &node);
        }

        if !node.is_graphic() {
            continue;
        }

        let opacity = node.attributes().get_number_or(AId::Opacity, 1.0);
        if opacity.fuzzy_eq(&1.0) && !has_links(&node) {
            continue;
        }

        let mut g_node = doc.create_element(EId::G);

        {
            let attrs = node.attributes();
            for aid in &g_attrs {
                if *aid == AId::Opacity && opacity.fuzzy_eq(&1.0) {
                    continue;
                }

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

        for id in &ids {
            node.remove_attribute(*id);
        }

        node.insert_before(g_node.clone());
        node.detach();
        g_node.append(node.clone());
    }
}

fn has_links(node: &Node) -> bool {
    if let Some(&AValue::FuncLink(_)) = node.attributes().get_value(AId::ClipPath) {
        return true;
    }

    if let Some(&AValue::FuncLink(_)) = node.attributes().get_value(AId::Mask) {
        return true;
    }

    if let Some(&AValue::FuncLink(_)) = node.attributes().get_value(AId::Filter) {
        return true;
    }

    false
}
