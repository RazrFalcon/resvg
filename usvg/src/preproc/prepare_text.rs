// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use unicode_segmentation::UnicodeSegmentation;
use svgdom::{
    NodeType,
    NumberList,
};

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
pub fn prepare_text_decoration(doc: &Document) {
    for mut node in doc.root().descendants().filter(|n| n.is_tag_name(EId::Text)) {
        let mut td = String::new();
        if has_decoration_attr(&node, "underline") {
            td.push_str("underline;");
        }

        if has_decoration_attr(&node, "overline") {
            td.push_str("overline;");
        }

        if has_decoration_attr(&node, "line-through") {
            td.push_str("line-through;");
        }

        if !td.is_empty() {
            td.pop();
            node.set_attribute((AId::TextDecoration, td));
        }
    }
}

fn has_decoration_attr(root: &Node, decoration_id: &str) -> bool {
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


fn prepare_baseline_shift(doc: &Document, opt: &Options) {
    let mut resolved = Vec::new();

    for node in doc.root().descendants().filter(|n| n.is_tag_name(EId::Text)) {
        for text_node in node.descendants().filter(|n| n.is_text()) {
            let mut shift = 0.0;

            for tspan in text_node.ancestors().take_while(|n| !n.is_tag_name(EId::Text)) {
                let attrs = tspan.attributes();

                let av = attrs.get_value(AId::BaselineShift);
                let font_size = attrs.get_number(AId::FontSize).unwrap_or(opt.font_size);

                match av {
                    Some(AValue::String(ref s)) => {
                        match s.as_str() {
                            "baseline" => {}
                            "sub" => shift += font_size * -0.2,
                            "super" => shift += font_size * 0.4,
                            _ => {}
                        }
                    }
                    Some(AValue::Length(len)) => {
                        if len.unit == Unit::Percent {
                            shift += font_size * (len.num / 100.0);
                        }
                    }
                    Some(AValue::Number(n)) => shift += n,
                    _ => {}
                }
            }

            let mut tspan = text_node.parent().unwrap();
            resolved.push((tspan, shift));
        }
    }

    for (mut node, shift) in resolved {
        node.set_attribute((AId::BaselineShift, shift));
    }
}


pub fn prepare_text_nodes(doc: &mut Document, opt: &Options) {
    sanitize_text(doc.root(), doc);
    prepare_baseline_shift(doc, opt);

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

// Removes `text` inside `text`, since it should be ignored.
fn sanitize_text(parent: Node, doc: &mut Document) {
    for node in parent.children() {
        if node.is_tag_name(EId::Text) {
            doc.drain(node, |n| n.is_tag_name(EId::Text));
            continue;
        }

        if node.has_children() {
            sanitize_text(node, doc);
        }
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
            } else {
                for _ in 0..chars_count {
                    list.push(0.0);
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
                warn!("Unsupported text child: {}.", id);
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
