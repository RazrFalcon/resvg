// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use super::prelude::*;


/// Removes any text/tspan nodes with `font-size` <= 0.
///
/// Must be ran only after text prepossessing.
pub fn remove_invalid_font_size(doc: &mut Document, opt: &Options) {
    let mut rm_nodes = Vec::new();

    for text_node in doc.root().descendants().filter(|n| n.is_tag_name(EId::Text)) {
        for text_chunk in text_node.children() {
            let size = text_chunk.attributes()
                                 .get_number_or(AId::FontSize, opt.font_size);
            if size <= 0.0 {
                rm_nodes.push(text_chunk);
                continue;
            }
        }
    }
    rm_nodes.iter_mut().for_each(|n| doc.remove_node(n.clone()));
    rm_nodes.clear();


    // Remove empty chunks.
    for text_node in doc.root().descendants().filter(|n| n.is_tag_name(EId::Text)) {
        for text_chunk in text_node.children() {
            if !text_chunk.has_children() {
                rm_nodes.push(text_chunk);
            }
        }
    }
    rm_nodes.iter_mut().for_each(|n| doc.remove_node(n.clone()));
    rm_nodes.clear();


    // Remove empty text nodes.
    for text_node in doc.root().descendants().filter(|n| n.is_tag_name(EId::Text)) {
        if !text_node.has_children() {
            rm_nodes.push(text_node);
        }
    }
    rm_nodes.iter_mut().for_each(|n| doc.remove_node(n.clone()));
}
