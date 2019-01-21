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

pub fn prepare_text(doc: &mut Document, opt: &Options) {
    sanitize_text(doc.root(), doc);
    prepare_baseline_shift(doc, opt);
}

// Removes `text` inside `text`, since it should be ignored.
fn sanitize_text(parent: svgdom::Node, doc: &mut svgdom::Document) {
    for node in parent.children() {
        if node.is_tag_name(EId::Text) {
            doc.drain(node, |n| n.is_tag_name(EId::Text));
            continue;
        }

        if node.has_children() {
            sanitize_text(node, doc);
        }
    }

    // TODO: no textPath in textPath
    // TODO: only `text` can have a `textPath` child
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
