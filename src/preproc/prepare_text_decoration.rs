// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom::{
    Document,
    Node,
    ValueId,
};

// self
use short::{
    AId,
    EId,
};
use traits::{
    GetValue,
};


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
// It's not by the SVG spec, but easier than keepeng all the groups.
//
// Tested by:
// - text-deco-*.svg
pub fn prepare_text_decoration(doc: &mut Document) {
    for mut node in doc.descendants().filter(|n| n.is_tag_name(EId::Text)) {
        let mut td = String::new();
        if has_attr(&node, ValueId::Underline) {
            td.push_str("underline;");
        }

        if has_attr(&node, ValueId::Overline) {
            td.push_str("overline;");
        }

        if has_attr(&node, ValueId::LineThrough) {
            td.push_str("linethrough;");
        }

        if !td.is_empty() {
            td.pop();
            node.set_attribute((AId::TextDecoration, td));
        }
    }
}

fn has_attr(root: &Node, decoration_id: ValueId) -> bool {
    for (_, node) in root.parents_with_self().svg() {
        let attrs = node.attributes();

        if let Some(id) = attrs.get_predef(AId::TextDecoration) {
            if id == decoration_id {
                return true;
            }
        }
    }

    false
}
