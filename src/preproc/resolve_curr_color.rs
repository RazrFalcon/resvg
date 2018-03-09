// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom::{
    Attribute,
    Document,
    Node,
    ValueId,
};

// self
use short::{
    AId,
    AValue,
};


/// Resolve 'currentColor' attribute.
///
/// The function will fallback to a default value when possible.
//
// a-fill-022.svg
pub fn resolve_current_color(doc: &Document) {
    let mut ids = Vec::new();

    for (_, mut node) in doc.descendants().svg() {
        ids.clear();

        {
            let attrs = node.attributes();
            for (aid, attr) in attrs.iter_svg() {
                if let AValue::PredefValue(ref v) = attr.value {
                    if *v == ValueId::CurrentColor {
                        ids.push(aid);
                    }
                }
            }
        }

        for id in &ids {
            let av = node.attributes().get_value(AId::Color).cloned();
            if let Some(av) = av {
                node.set_attribute((*id, av.clone()));
            } else {
                resolve_impl(&mut node, *id, AId::Color);
            }
        }
    }
}

fn resolve_impl(node: &mut Node, curr_attr: AId, parent_attr: AId) {
    if let Some(n) = node.parents().find(|n| n.has_attribute(parent_attr)) {
        let av = n.attributes().get_value(parent_attr).cloned();
        if let Some(av) = av {
            node.set_attribute((curr_attr, av.clone()));
        }
    } else {
        match Attribute::default(curr_attr) {
            Some(a) => node.set_attribute((curr_attr, a.value)),
            None => {
                warn!("Failed to resolve attribute: {}. Removing it.",
                      node.attributes().get(curr_attr).unwrap());
                node.remove_attribute(curr_attr);
            }
        }
    }
}
