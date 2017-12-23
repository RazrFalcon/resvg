// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom::{
    Node,
};
use svgdom::types::{
    Transform,
};

use short::{
    AId,
    AValue,
    EId,
};

use traits::{
    GetValue,
};


pub fn ungroup_groups(svg: &Node) {
    let mut groups = Vec::with_capacity(16);

    loop {
        _ungroup_groups(&svg, &mut groups);

        if groups.is_empty() {
            break;
        }

        while let Some(mut g) = groups.pop() {
            ungroup_group(&mut g);
            g.remove();
        }
    }
}

fn _ungroup_groups(parent: &Node, groups: &mut Vec<Node>) {
    for node in parent.children() {
        if node.has_children() {
            _ungroup_groups(&node, groups);
        }

        if node.is_tag_name(EId::G) {
            if !node.has_children() {
                groups.push(node.clone());
                continue;
            }

            // Groups with `clip-path` attribute can't be ungroupped.
            if let Some(&AValue::FuncLink(_)) = node.attributes().get_type(AId::ClipPath) {
                continue;
            }

            // We can ungroup group with opacity only when it has only one child.
            if node.has_attribute(AId::Opacity) {
                if node.children().count() != 1 {
                    continue;
                }
            }

            groups.push(node.clone());
        }
    }
}

fn ungroup_group(g: &mut Node) {
    for (aid, attr) in g.attributes().iter_svg() {
        for (_, mut child) in g.children().svg() {
            if aid == AId::Opacity {
                if child.has_attribute(aid) {
                    // We can't just replace 'opacity' attribute,
                    // we should multiply it.
                    let op1 = if let AValue::Number(n) = attr.value { n } else { 1.0 };
                    let op2 = child.attributes().get_number(aid).unwrap_or(1.0);
                    child.set_attribute((aid, op1 * op2));
                    continue;
                }
            }

            if aid == AId::Transform {
                if child.has_attribute(aid) {
                    // We should multiply transform matrices.
                    let mut t1 = if let AValue::Transform(n) = attr.value {
                        n
                    } else {
                        Transform::default()
                    };
                    let t2 = child.attributes().get_transform(aid).unwrap_or_default();

                    t1.append(&t2);
                    child.set_attribute((aid, t1));
                    continue;
                }
            }

            if aid == AId::Display {
                // Display attribute has a priority during rendering, so we must
                // copy it even if a child has it already.
                child.set_attribute((aid, attr.value.clone()));
                continue;
            }

            child.set_attribute_if_none(aid, &attr.value);
        }
    }

    while g.has_children() {
        let mut c = g.last_child().unwrap();
        c.detach();
        g.insert_after(&c);
    }
}
