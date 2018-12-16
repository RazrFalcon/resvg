// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use super::prelude::*;


pub fn resolve_gradient_stops(doc: &mut Document) {
    let iter = doc.root().descendants()
        .filter(|n| n.is_gradient())
        .filter(|n| n.has_attribute(AId::Href))
        .filter(|n| !n.has_children());
    for node in iter {
        let link = node.clone();
        resolve(doc, node, &link);
    }
}

pub fn resolve_pattern_children(doc: &mut Document) {
    let iter = doc.root().descendants()
        .filter(|n| n.is_tag_name(EId::Pattern))
        .filter(|n| n.has_attribute(AId::Href))
        .filter(|n| !n.has_children());
    for node in iter {
        let link = node.clone();
        resolve(doc, node, &link);
    }
}

pub fn resolve_filter_children(doc: &mut Document) {
    let iter = doc.root().descendants()
        .filter(|n| n.is_tag_name(EId::Filter))
        .filter(|n| n.has_attribute(AId::Href))
        .filter(|n| !n.has_children());
    for node in iter {
        let link = node.clone();
        resolve(doc, node, &link);
    }
}

fn resolve(doc: &mut Document, mut node: Node, link: &Node) {
    // We do not check that `link` has a valid element type,
    // because it was already done in `fix_xlinks()`.

    if !link.has_children() {
        let av = link.attributes().get_value(AId::Href).cloned();
        if let Some(AValue::Link(ref_node)) = av {
            resolve(doc, node, &ref_node);
            return;
        }
    }

    for stop in link.children() {
        let new_stop = doc.copy_node_deep(stop);
        node.append(new_stop);
    }
}
