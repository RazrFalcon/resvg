// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom::{
    Document,
    ValueId,
};

// self
use short::{
    AId,
    AValue,
    EId,
};


pub fn fix_recursive_pattern(doc: &Document) {
    // If a pattern child has a link to the pattern itself
    // then we have to replace it with `none`.
    // Otherwise we will get endless loop/recursion and stack overflow.
    for pattern_node in doc.descendants().filter(|n| n.is_tag_name(EId::Pattern)) {
        for mut node in pattern_node.descendants() {
            let mut check_attr = |aid: AId| {
                let av = node.attributes().get_value(aid).cloned();
                if let Some(AValue::FuncLink(link)) = av {
                    if link == pattern_node {
                        node.set_attribute((aid, ValueId::None));
                    } else {
                        // Check that linked node children doesn't link this pattern.
                        for mut node2 in link.descendants() {
                            let av2 = node2.attributes().get_value(aid).cloned();
                            if let Some(AValue::FuncLink(link2)) = av2 {
                                if link2 == pattern_node {
                                    node.set_attribute((aid, ValueId::None));
                                }
                            }
                        }
                    }
                }
            };

            check_attr(AId::Fill);
            check_attr(AId::Stroke);
        }
    }
}
