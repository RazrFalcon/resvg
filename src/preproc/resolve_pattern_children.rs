// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom::{
    Document,
    Node,
};

// self
use short::{
    AId,
    AValue,
    EId,
};


pub fn resolve_pattern_children(doc: &Document) {
    let iter = doc.descendants().filter(|n| n.is_tag_name(EId::Pattern))
        .filter(|n| n.has_attribute(("xlink", AId::Href)))
        .filter(|n| !n.has_children());
    for mut node in iter {
        let link = node.clone();
        resolve(&mut node, &link);
    }

    // Remove 'xlink:href' in patterns, because we already resolved everything.
    let iter = doc.descendants().filter(|n| n.is_tag_name(EId::Pattern))
        .filter(|n| n.has_attribute(("xlink", AId::Href)));
    for mut node in iter {
        node.remove_attribute(("xlink", AId::Href));
    }
}

fn resolve(pattern: &mut Node, linked_pattern: &Node) {
    let av = linked_pattern.attributes().get_value(("xlink", AId::Href)).cloned();
    match av {
        Some(av) => {
            match av {
                AValue::Link(ref_node) => resolve(pattern, &ref_node),
                _ => unreachable!(),
            }
        }
        None => {
            for stop in linked_pattern.children() {
                let new_stop = stop.make_copy();
                pattern.append(&new_stop);
            }
        }
    }
}
