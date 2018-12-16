// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom::{
    Attribute,
};

use super::prelude::*;


/// Resolves the `inherit` attribute value.
///
/// The function will fallback to a default value when possible.
pub fn resolve_inherit(doc: &Document) {
    let mut ids = Vec::new();
    for (_, mut node) in doc.root().descendants().svg() {
        ids.clear();

        for (aid, attr) in node.attributes().iter().svg() {
            if let AValue::Inherit = attr.value {
                ids.push(aid);
            }
        }

        for id in &ids {
            resolve_impl(&mut node, *id);
        }
    }
}

fn resolve_impl(node: &mut Node, attr: AId) {
    if attr.is_inheritable() {
        if let Some(n) = node.ancestors().skip(1).find(|n| n.has_attribute(attr)) {
            let av = n.attributes().get_value(attr).cloned();
            if let Some(av) = av {
                node.set_attribute((attr, av.clone()));
                return;
            }
        }
    } else {
        if let Some(parent) = node.parent() {
            let av = parent.attributes().get_value(attr).cloned();
            if let Some(av) = av {
                node.set_attribute((attr, av.clone()));
                return;
            }
        }
    }

    match Attribute::new_default(attr) {
        Some(a) => node.set_attribute((attr, a.value)),
        None => {
            warn!("Failed to resolve attribute: {}. Removing it.",
                    node.attributes().get(attr).unwrap());
            node.remove_attribute(attr);
        }
    }
}
