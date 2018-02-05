// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom::{
    ElementType,
    Node,
};


pub fn remove_unused_defs(svg: &mut Node) {
    remove_unused_defs_impl(svg);
}

fn remove_unused_defs_impl(parent: &mut Node) {
    let mut mv_nodes = Vec::new();
    let mut rm_nodes = Vec::new();

    for mut node in parent.children() {
        if node.is_referenced() && !node.is_used() {
            ungroup_children(&node, &mut mv_nodes, &mut rm_nodes);
        } else if node.has_children() {
            remove_unused_defs_impl(&mut node);
        }
    }

    for node in mv_nodes {
        parent.append(&node);
    }

    for mut node in rm_nodes {
        node.remove();
    }
}

fn ungroup_children(node: &Node, mv_nodes: &mut Vec<Node>, rm_nodes: &mut Vec<Node>) {
    if node.has_children() {
        // Element can be unused, but elements in it can be,
        // so we need to move them to parent element before removing.
        for c in node.children() {
            if c.is_used() {
                mv_nodes.push(c.clone());
            }
        }
    }

    rm_nodes.push(node.clone());
}
