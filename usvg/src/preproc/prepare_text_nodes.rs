// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use unicode_segmentation::UnicodeSegmentation;
use svgdom::{
    NodeType,
    NumberList,
};

use super::prelude::*;


pub fn prepare_text_nodes(doc: &mut Document) {
    // Resolve rotation for each character before any preprocessing,
    // because rotate angles depend on text children tree structure.
    {
        for mut node in doc.root().descendants().filter(|n| n.is_tag_name(EId::Text)) {
            if !node.descendants().any(|n| n.has_attribute(AId::Rotate)) {
                continue;
            }

            let mut rotate_list = Vec::new();
            resolve_rotate(&node, 0, &mut rotate_list);
            if !rotate_list.is_empty() {
                node.set_attribute((AId::Rotate, NumberList::from(rotate_list)));
            }
        }
    }

    let mut rm_nodes = Vec::new();

    for mut node in doc.root().descendants().filter(|n| n.is_tag_name(EId::Text)) {
        let mut new_text_elem = doc.create_element(EId::Text);
        new_text_elem.set_id(node.id().clone());
        prepare_text_elem(doc, &node, &mut new_text_elem);

        rm_nodes.push(node.clone());

        if !new_text_elem.has_children() {
            continue;
        }

        node.insert_before(new_text_elem.clone());

        for (_, attr) in node.attributes().iter().svg() {
            new_text_elem.set_attribute(attr.clone());
        }
    }

    for node in rm_nodes {
        doc.remove_node(node);
    }
}

fn resolve_rotate(parent: &Node, mut offset: usize, list: &mut Vec<f64>) {
    for child in parent.children() {
        if child.is_text() {
            let chars_count = UnicodeSegmentation::graphemes(child.text().as_str(), true).count();
            // TODO: should stop at the root 'text'
            if let Some(p) = child.find_node_with_attribute(AId::Rotate) {
                let attrs = p.attributes();
                if let Some(rotate_list) = attrs.get_number_list(AId::Rotate) {
                    for i in 0..chars_count {
                        let r = match rotate_list.get(i + offset) {
                            Some(r) => *r,
                            None => {
                                // Use last angle if the index is out of bounds.
                                *rotate_list.last().unwrap_or(&0.0)
                            }
                        };

                        list.push(r);
                    }

                    offset += chars_count;
                }
            }
        } else if child.is_tag_name(EId::Tspan) {
            // Use parent rotate list if it is not set.
            let sub_offset = if child.has_attribute(AId::Rotate) { 0 } else { offset };
            resolve_rotate(&child, sub_offset, list);

            // 'tspan' represent a single char.
            offset += 1;
        }
    }
}

fn prepare_text_elem(doc: &mut Document, elem: &Node, new_elem: &mut Node) {
    for node in elem.descendants().filter(|n| n.is_text()) {
        let text_parent = node.parent().unwrap();

        if node.text().is_empty() {
            continue;
        }

        if let Some(id) = text_parent.tag_id() {
            if id != EId::Text && id != EId::Tspan {
                warn!("Unsupported text child: {:?}.", id);
                continue;
            }
        } else {
            // Text node parent must be an SVG element.
            warn!("Invalid text node parent.");
            continue;
        }

        let attrs = text_parent.attributes();

        let mut new_tspan = doc.create_element(EId::Tspan);
        new_elem.append(new_tspan.clone());

        let new_text_node = doc.create_node(NodeType::Text, node.text().clone());
        new_tspan.append(new_text_node.clone());

        for (aid, attr) in attrs.iter().svg() {
            match aid {
                AId::X | AId::Y | AId::Dx | AId::Dy => {
                    if text_parent.is_tag_name(EId::Tspan) {
                        if text_parent.first_child() == Some(node.clone()) {
                            new_tspan.set_attribute(attr.clone());
                        }
                    }
                }
                _ => new_tspan.set_attribute(attr.clone()),
            }
        }
    }
}
