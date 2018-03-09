// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom::{
    Document,
};

// self
use short::{
    AId,
    EId,
};
use traits::{
    GetValue,
};


// Remove any text/tspan nodes with `font-size` <= 0.
//
// Must be ran only after text prepossessing.
//
// a-font-size-009.svg
// a-font-size-010.svg
// a-font-size-011.svg
// a-font-size-012.svg
// a-font-size-013.svg
pub fn remove_invalid_font_size(doc: &mut Document) {
    let mut rm_nodes = Vec::new();

    for text_node in doc.descendants().filter(|n| n.is_tag_name(EId::Text)) {
        for text_chunk in text_node.children() {
            let size = text_chunk.attributes().get_number(AId::FontSize)
                                 .unwrap_or(super::DEFAULT_FONT_SIZE);
            if size <= 0.0 {
                rm_nodes.push(text_chunk);
                continue;
            }

            for text_span in text_chunk.children() {
                let size = text_span.attributes().get_number(AId::FontSize)
                                    .unwrap_or(super::DEFAULT_FONT_SIZE);
                if size <= 0.0 {
                    rm_nodes.push(text_span);
                }
            }
        }
    }
    rm_nodes.iter_mut().for_each(|n| n.remove());
    rm_nodes.clear();


    // Remove empty chunks.
    for text_node in doc.descendants().filter(|n| n.is_tag_name(EId::Text)) {
        for text_chunk in text_node.children() {
            if !text_chunk.has_children() {
                rm_nodes.push(text_chunk);
            }
        }
    }
    rm_nodes.iter_mut().for_each(|n| n.remove());
    rm_nodes.clear();


    // Remove empty text nodes.
    rm_nodes.clear();
    for text_node in doc.descendants().filter(|n| n.is_tag_name(EId::Text)) {
        if !text_node.has_children() {
            rm_nodes.push(text_node);
        }
    }
    rm_nodes.iter_mut().for_each(|n| n.remove());
}
