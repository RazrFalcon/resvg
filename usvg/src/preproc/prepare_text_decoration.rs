// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use super::prelude::*;


// <g fill="red" text-decoration="underline">
//   <g fill="blue" text-decoration="overline">
//     <text fill="green" text-decoration="line-through">Text</text>
//   </g>
// </g>
//
// In this example 'text' element will have all decorators enabled, but color
// will be green for all of them.
//
// There is no simpler way to express 'text-decoration' property
// without groups than collect all the options to the string.
// It's not by the SVG spec, but easier than keeping all the groups.
pub fn prepare_text_decoration(doc: &mut Document) {
    for mut node in doc.root().descendants().filter(|n| n.is_tag_name(EId::Text)) {
        let mut td = String::new();
        if has_attr(&node, "underline") {
            td.push_str("underline;");
        }

        if has_attr(&node, "overline") {
            td.push_str("overline;");
        }

        if has_attr(&node, "line-through") {
            td.push_str("line-through;");
        }

        if !td.is_empty() {
            td.pop();
            node.set_attribute((AId::TextDecoration, td));
        }
    }
}

fn has_attr(root: &Node, decoration_id: &str) -> bool {
    for (_, node) in root.ancestors().svg() {
        let attrs = node.attributes();

        if let Some(text) = attrs.get_str(AId::TextDecoration) {
            if text == decoration_id {
                return true;
            }
        }
    }

    false
}
