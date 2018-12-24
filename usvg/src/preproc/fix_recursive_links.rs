// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use super::prelude::*;


pub fn fix_recursive_links(doc: &Document) {
    fix_pattern(doc);
    fix_marker(doc);
    fix_func_iri(doc, EId::ClipPath, AId::ClipPath);
    fix_func_iri(doc, EId::Mask, AId::Mask);
    fix_func_iri(doc, EId::Filter, AId::Filter);
}

fn fix_pattern(doc: &Document) {
    for pattern_node in doc.root().descendants().filter(|n| n.is_tag_name(EId::Pattern)) {
        for mut node in pattern_node.descendants() {
            let mut check_attr = |aid: AId| {
                let av = node.attributes().get_value(aid).cloned();
                if let Some(AValue::Paint(link, _)) = av {
                    if link == pattern_node {
                        // If a pattern child has a link to the pattern itself
                        // then we have to replace it with `none`.
                        // Otherwise we will get endless loop/recursion and stack overflow.
                        node.set_attribute((aid, AValue::None));
                    } else {
                        // Check that linked node children doesn't link this pattern.
                        for node2 in link.descendants() {
                            let av2 = node2.attributes().get_value(aid).cloned();
                            if let Some(AValue::Paint(link2, _)) = av2 {
                                if link2 == pattern_node {
                                    node.set_attribute((aid, AValue::None));
                                }
                            }
                        }
                    }
                }
            };

            check_attr(AId::Fill);
            check_attr(AId::Stroke);
        }
    }
}

fn fix_marker(doc: &Document) {
    for marker_node in doc.root().descendants().filter(|n| n.is_tag_name(EId::Marker)) {
        for mut node in marker_node.descendants() {
            let mut check_attr = |aid: AId| {
                let av = node.attributes().get_value(aid).cloned();
                if let Some(AValue::FuncLink(link)) = av {
                    if link == marker_node {
                        // If a marker child has a link to the marker itself
                        // then we have to remove it.
                        // Otherwise we will get endless loop/recursion and stack overflow.
                        node.remove_attribute(aid);
                    } else {
                        // Check that linked node children doesn't link this marker.
                        for node2 in link.descendants() {
                            let av2 = node2.attributes().get_value(aid).cloned();
                            if let Some(AValue::FuncLink(link2)) = av2 {
                                if link2 == marker_node {
                                    node.remove_attribute(aid);
                                }
                            }
                        }
                    }
                }
            };

            check_attr(AId::MarkerStart);
            check_attr(AId::MarkerMid);
            check_attr(AId::MarkerEnd);
        }
    }
}

fn fix_func_iri(doc: &Document, eid: EId, aid: AId) {
    for node in doc.root().descendants().filter(|n| n.is_tag_name(eid)) {
        for mut child in node.descendants() {
            let av = child.attributes().get_value(aid).cloned();
            if let Some(AValue::FuncLink(link)) = av {
                if link == node {
                    // If a mask child has a link to the mask itself
                    // then we have to replace it with `none`.
                    // Otherwise we will get endless loop/recursion and stack overflow.
                    child.remove_attribute(aid);
                } else {
                    // Check that linked node children doesn't link this mask.
                    for mut node2 in link.descendants() {
                        let av2 = node2.attributes().get_value(aid).cloned();
                        if let Some(AValue::FuncLink(link2)) = av2 {
                            if link2 == node {
                                node2.remove_attribute(aid);
                            }
                        }
                    }
                }
            }
        }
    }
}
