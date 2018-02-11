// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom::{
    Document,
    Node,
    NodeType,
};

// self
use short::{
    AId,
    EId,
};


pub fn prepare_text_nodes(doc: &mut Document) {
    let mut rm_nodes = Vec::new();

    for (id, mut node) in doc.descendants().svg() {
        if id != EId::Text {
            continue;
        }

        let mut new_text_elem = doc.create_element(EId::Text);
        new_text_elem.set_id(node.id().clone());
        prepare_text_elem(doc, &node, &mut new_text_elem);

        rm_nodes.push(node.clone());

        if !new_text_elem.has_children() {
            continue;
        }

        node.insert_before(&new_text_elem);

        let ref attrs = node.attributes();
        for (_, attr) in attrs.iter_svg() {
            new_text_elem.set_attribute(attr.clone());
        }
    }

    for mut node in rm_nodes {
        node.remove();
    }
}

fn prepare_text_elem(doc: &mut Document, elem: &Node, new_elem: &mut Node) {
    for node in elem.descendants().filter(|n| n.node_type() == NodeType::Text) {
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

        let ref attrs = text_parent.attributes();

        let mut new_tspan = doc.create_element(EId::Tspan);
        new_elem.append(&new_tspan);

        let new_text_node = doc.create_node(NodeType::Text, &node.text());
        new_tspan.append(&new_text_node);

        for (aid, attr) in attrs.iter_svg() {
            match aid {
                AId::X | AId::Y => {
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
