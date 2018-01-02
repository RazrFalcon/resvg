// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom::{
    Document,
    FuzzyEq,
    ValueId,
    Node,
};

use short::{
    AId,
    AValue,
    EId,
};

use traits::{
    GetValue,
};


/// Remove invalid patterns and replace links to this patterns with 'none'.
pub fn remove_invalid_patterns(doc: &mut Document) {
    let mut ids = Vec::new();
    let mut nodes = Vec::new();

    for node in doc.descendants().filter(|n| n.is_tag_name(EId::Pattern)) {
        {
            let ref attrs = node.attributes();
            let w = attrs.get_number(AId::Width).unwrap_or(0.0);
            let h = attrs.get_number(AId::Height).unwrap_or(0.0);

            // If width or height is zero - remove this pattern.
            if !w.is_fuzzy_zero() && !h.is_fuzzy_zero() {
                continue;
            }
        }

        for mut linked in node.linked_nodes().collect::<Vec<Node>>() {
            ids.clear();

            for (aid, attr) in linked.attributes().iter_svg() {
                match attr.value {
                      AValue::Link(ref link)
                    | AValue::FuncLink(ref link) => {
                        if link == &node {
                            ids.push(aid);
                        }
                    }
                    _ => {}
                }
            }

            for id in &ids {
                linked.set_attribute((*id, ValueId::None));
            }
        }

        nodes.push(node.clone());
    }

    for mut node in nodes {
        node.remove();
    }
}
