// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom::{
    Document,
    Node,
};
use svgdom::types::{
    Transform,
};

use short::{
    AId,
    AValue,
    EId,
};

use traits::{
    GetValue,
};


// Tested by:
// - struct-use-*.svg
pub fn resolve_use(doc: &Document) {
    let mut nodes = Vec::new();

    // 'use' elements can be linked in any order,
    // so we have to process the tree until all 'use' are solved.
    let mut is_any_resolved = true;
    while is_any_resolved {
        is_any_resolved = false;
        nodes.clear();

        for mut node in doc.descendants().filter(|n| n.is_tag_name(EId::Use)) {
            let av = node.attributes().get_value(AId::XlinkHref).cloned();
            if let Some(AValue::Link(link)) = av {
                // Ignore 'use' elements linked to other 'use' elements.
                if link.is_tag_name(EId::Use) {
                    continue;
                }

                // We don't support 'use' elements linked to 'svg' element.
                if link.is_tag_name(EId::Svg) {
                    nodes.push(node);
                    continue;
                }

                if link.is_tag_name(EId::Symbol) {
                    nodes.push(node);
                    continue;
                }

                _resolve_use(&mut node, &link);
                is_any_resolved = true;
            }

            // 'use' elements without 'xlink:href' attribute will be removed
            // by 'remove_invisible_elements()'.
        }

        // Remove unresolved 'use' elements, since there is not need
        // to keep them around and they will be skipped anyway.
        for node in &mut nodes {
            node.remove();
        }
    }
}

fn _resolve_use(use_node: &mut Node, linked_node: &Node) {
    // Unlink 'use'.
    use_node.remove_attribute(AId::XlinkHref);

    {
        // 'use' element support 'x', 'y' and 'transform' attributes
        // and we should process them.
        // So we apply translate transform to the linked element transform.

        let mut attrs = use_node.attributes_mut();

        // 'x' or 'y' should be set.
        if attrs.contains(AId::X) || attrs.contains(AId::Y) {
            let x = attrs.get_number(AId::X).unwrap_or(0.0);
            let y = attrs.get_number(AId::Y).unwrap_or(0.0);

            let mut ts = attrs.get_transform(AId::Transform)
                              .unwrap_or(Transform::default());

            ts.translate(x, y);

            attrs.insert_from(AId::Transform, ts);
            attrs.remove(AId::X);
            attrs.remove(AId::Y);
        }
    }

    // Create a deep copy of the linked node.
    let mut new_node = linked_node.make_deep_copy();
    use_node.insert_after(&new_node);

    // Copy attributes from 'use'.
    for (aid, attr) in use_node.attributes().iter_svg() {
        // Do not replace existing attributes.
        if !new_node.has_attribute(aid) {
            new_node.set_attribute(attr.clone());
        }
    }

    // Copy old ID.
    new_node.set_id(use_node.id().clone());

    // Relink linked nodes to the new node.
    for mut n in use_node.linked_nodes().collect::<Vec<Node>>() {
        n.set_attribute((AId::XlinkHref, new_node.clone()));
    }

    // Remove resolved 'use'.
    use_node.remove();
}
