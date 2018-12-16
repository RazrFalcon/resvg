// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom::{
    NodeType,
};

use super::prelude::*;


pub fn resolve_tref(doc: &mut Document) {
    for mut tref in doc.root().descendants().filter(|n| n.is_tag_name(EId::Tref)) {
        let av = tref.attributes().get_value(AId::Href).cloned();
        let text_elem = if let Some(AValue::Link(ref link)) = av {
            link.clone()
        } else {
            continue;
        };

        // 'All character data within the referenced element, including character data enclosed
        // within additional markup, will be rendered.'
        //
        // So we don't care about attributes and everything. Just collecting text nodes data.
        let mut text = String::new();
        for node in text_elem.descendants().filter(|n| n.is_text()) {
            text.push_str(&node.text());
        }

        let text_node = doc.create_node(NodeType::Text, text);
        tref.append(text_node);

        tref.set_tag_name(EId::Tspan);
        tref.remove_attribute(AId::Href);

        for (aid, attr) in text_elem.attributes().iter().svg() {
            if !tref.has_attribute(aid) && attr.is_inheritable() {
                tref.set_attribute(attr.clone());
            }
        }
    }
}
