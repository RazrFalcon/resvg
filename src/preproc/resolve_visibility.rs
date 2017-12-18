// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom::{
    Document,
    Node,
    ValueId,
};

use short::{
    AId,
    EId,
};

use traits::{
    GetValue,
};


// TODO: Note that if the ‘visibility’ property is set to hidden on a ‘tspan’, ‘tref’ or ‘altGlyph’
//       element, then the text is invisible but still takes up space in text layout calculations.

/// 'Setting ‘visibility’ to hidden on a ‘g’ will make its children invisible as
/// long as the children do not specify their own ‘visibility’ properties as visible.
/// Note that ‘visibility’ is not an inheritable property.'
pub fn resolve_visibility(doc: &Document) {
    // Instead of removing invisible elements directly we mark them with display:none.
    // That way they can be properly removed by existing algorithms.
    for (id, mut node) in doc.descendants().svg() {
        if id == EId::G {
            if !is_hidden(&node) {
                continue;
            }

            node.remove_attribute(AId::Visibility);

            for mut child in node.children() {
                if child.attributes().get_predef(AId::Visibility) != Some(ValueId::Visible) {
                    child.set_attribute((AId::Display, ValueId::None));
                }
            }
        } else {
            if is_hidden(&node) {
                node.set_attribute((AId::Display, ValueId::None));
            }
        }
    }

    // Remove all 'visibility' attributes, because we don't need them anymore.
    for mut node in doc.descendants() {
        node.remove_attribute(AId::Visibility);
    }
}

/// Checks that element has 'visibility' set to 'hidden' or 'collapse'.
fn is_hidden(node: &Node) -> bool {
    match node.attributes().get_predef(AId::Visibility) {
        Some(ValueId::Visible) => false,
        None => false,
        _ => true,
    }
}
