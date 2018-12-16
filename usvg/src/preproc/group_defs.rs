// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use super::prelude::*;


/// Moves all referenceable elements to the `defs` element.
pub fn group_defs(doc: &mut Document, svg: &mut Node) {
    // Create 'defs' node if it didn't exist already.
    let mut defs = match svg.descendants().filter(|n| n.is_tag_name(EId::Defs)).nth(0) {
        Some(n) => n,
        None => doc.create_element(EId::Defs),
    };

    // Make 'defs' a first child of the 'svg'.
    if svg.first_child() != Some(defs.clone()) {
        defs.detach();
        svg.prepend(defs.clone());
    }

    // Move all referenced elements to the main 'defs'.
    {
        let mut nodes = Vec::new();

        for (_, node) in svg.descendants().svg() {
            if node.is_referenced() {
                if let Some(parent) = node.parent() {
                    if parent != defs {
                        nodes.push(node.clone());
                    }
                }
            }
        }

        for n in &mut nodes {
            resolve_attrs(n);
            n.detach();
            defs.append(n.clone());
        }
    }

    // Ungroup all existing 'defs', except main.
    {
        let mut nodes = Vec::new();
        for (_, node) in svg.descendants().svg() {
            if node.is_tag_name(EId::Defs) && node != defs {
                for child in node.children() {
                    nodes.push(child.clone());
                }
            }
        }

        for n in &mut nodes {
            n.detach();
            defs.append(n.clone());
        }
    }

    // Remove empty 'defs', except main.
    {
        let mut nodes = Vec::new();
        for (_, node) in svg.descendants().svg() {
            if node.is_tag_name(EId::Defs) && node != defs {
                nodes.push(node.clone());
            }
        }

        for n in &mut nodes {
            // Unneeded defs already ungrouped and must be empty.
            debug_assert!(!n.has_children());
            doc.remove_node(n.clone());
        }
    }
}

// Graphical elements inside referenced elements inherits parent attributes,
// so if we want to move this elements to the 'defs' - we should resolve attributes too.
fn resolve_attrs(node: &Node) {
    match node.tag_id().unwrap() {
          EId::ClipPath
        | EId::Marker
        | EId::Mask
        | EId::Pattern
        | EId::Symbol => {
            let mut parent = Some(node.clone());
            while let Some(p) = parent {
                let attrs = p.attributes();
                for (aid, attr) in attrs.iter().svg().filter(|&(_, a)| a.is_inheritable()) {
                    for mut child in node.children() {
                        if child.has_attribute(aid) {
                            continue;
                        }

                        child.set_attribute(attr.clone());
                    }
                }

                parent = p.parent();
            }
        }
        _ => {}
    }
}
