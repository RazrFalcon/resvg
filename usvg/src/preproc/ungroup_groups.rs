// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use super::prelude::*;


pub fn ungroup_groups(doc: &mut Document, svg: &Node, opt: &Options) {
    let mut groups = Vec::with_capacity(16);

    loop {
        _ungroup_groups(svg, opt, &mut groups);

        if groups.is_empty() {
            break;
        }

        while let Some(mut g) = groups.pop() {
            ungroup_group(&mut g);
            doc.remove_node(g);
        }
    }
}

fn _ungroup_groups(parent: &Node, opt: &Options, groups: &mut Vec<Node>) {
    for node in parent.children() {
        if node.has_children() {
            _ungroup_groups(&node, opt, groups);
        }

        if node.is_tag_name(EId::G) {
            if !node.has_children() {
                // Groups with a `filter` attribute can't be ungroupped.
                //
                // Because this is a valid SVG:
                // <filter id="filter1" filterUnits="userSpaceOnUse" x="20" y="20" width="160" height="160">
                //   <feFlood flood-color="green"/>
                // </filter>
                // <g filter="url(#filter1)"/>
                if let Some(&AValue::FuncLink(_)) = node.attributes().get_value(AId::Filter) {
                    continue;
                }

                groups.push(node.clone());
                continue;
            }

            if opt.keep_named_groups && node.has_id() {
                continue;
            }

            // Do not ungroup groups inside `clipPath`.
            // They will be removed during conversion.
            if node.ancestors().skip(1).any(|n| n.is_tag_name(EId::ClipPath)) {
                // Groups that was created from 'use' can be ungroupped.
                if !node.has_attribute("usvg-group") {
                    continue;
                }
            }

            // Groups with a `clip-path` attribute can't be ungroupped.
            if let Some(&AValue::FuncLink(_)) = node.attributes().get_value(AId::ClipPath) {
                continue;
            }

            // Groups with a `mask` attribute can't be ungroupped.
            if let Some(&AValue::FuncLink(_)) = node.attributes().get_value(AId::Mask) {
                continue;
            }

            // Groups with a `filter` attribute can't be ungroupped.
            if let Some(&AValue::FuncLink(_)) = node.attributes().get_value(AId::Filter) {
                continue;
            }

            // We can ungroup group with opacity only when it has only one child.
            let opacity = node.attributes().get_number_or(AId::Opacity, 1.0);
            if opacity.fuzzy_ne(&1.0) {
                if node.children().count() != 1 {
                    continue;
                }
            }

            groups.push(node.clone());
        }
    }
}

fn ungroup_group(g: &mut Node) {
    for (aid, attr) in g.attributes().iter().svg() {
        for (_, mut child) in g.children().svg() {
            if aid.is_inheritable() {
                child.set_attribute_if_none((aid, attr.value.clone()));
            } else {
                copy_non_inheritable_attribute(g, &mut child, aid);
            }
        }
    }

    let is_single_child = g.children().count() == 1;

    while g.has_children() {
        let mut child = g.last_child().unwrap();
        child.detach();
        g.insert_after(child.clone());

        // Transfer the group ID to the child.
        if is_single_child && !child.has_id() {
            child.set_id(g.id().clone());
        }
    }
}

fn copy_non_inheritable_attribute(g_node: &Node, child_node: &mut Node, aid: AId) {
    match aid {
        AId::Opacity => {
            // We can't just replace 'opacity' attribute,
            // we should multiply it.
            let op1 = g_node.attributes().get_number_or(aid, 1.0);
            let op2 = child_node.attributes().get_number_or(aid, 1.0);
            child_node.set_attribute((aid, op1 * op2));
        }
        AId::Transform => {
            // We should multiply transform matrices.
            let ts = g_node.attributes().get_transform(aid).unwrap_or_default();
            child_node.prepend_transform(ts);
        }
        AId::Display => {
            // Display attribute has a priority during rendering, so we must
            // copy it even if a child has it already.
            g_node.copy_attribute_to(aid, child_node);
        }
        AId::Visibility => {
            if let Some(av) = g_node.attributes().get_value(aid) {
                child_node.set_attribute_if_none((aid, av.clone()));
            }
        }
        _ => {}
    }
}
