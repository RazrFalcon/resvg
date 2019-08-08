// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use super::prelude::*;


/// Prepares an input `Document`.
///
/// # Errors
///
/// - If `Document` doesn't have an SVG node - clears the `doc`.
///
/// Basically, any error, even a critical one, should be recoverable.
/// In worst case scenario clear the `doc`.
pub fn prepare_doc(
    doc: &mut svgdom::Document,
) {
    let mut svg = if let Some(svg) = doc.svg_element() {
        svg
    } else {
        // Technically unreachable, because svgdom will return a parser error
        // if input SVG doesn't have an `svg` node.
        warn!("Invalid SVG structure. The Document will be cleared.");
        *doc = svgdom::Document::new();
        return;
    };

    let svg = &mut svg;

    resolve_root_style_attributes(doc, svg);
    resolve_use(doc);
    resolve_inherit(doc);
    resolve_current_color(doc);
    fix_recursive_links(doc);
    ungroup_a(doc);
    resolve_tref(doc);
    prepare_clip_path(doc);
    regroup_elements(doc, svg);
    prepare_text(doc);
}

fn fix_recursive_links(
    doc: &svgdom::Document,
) {
    fix_patterns(doc);
    fix_func_iri(doc, EId::ClipPath, AId::ClipPath);
    fix_func_iri(doc, EId::Mask, AId::Mask);
    fix_func_iri(doc, EId::Filter, AId::Filter);
}

fn fix_patterns(
    doc: &svgdom::Document,
) {
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

fn fix_func_iri(
    doc: &svgdom::Document,
    eid: EId,
    aid: AId,
) {
    for node in doc.root().descendants().filter(|n| n.is_tag_name(eid)) {
        for mut child in node.descendants() {
            let av = child.attributes().get_value(aid).cloned();
            if let Some(AValue::FuncLink(link)) = av {
                if link == node {
                    // If an element child has a link to the element itself
                    // then we have to replace it with `none`.
                    // Otherwise we will get endless loop/recursion and stack overflow.
                    child.remove_attribute(aid);
                } else {
                    // Check that linked node children doesn't link this element.
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

/// `clipPath` can have only shapes and `text` children and not groups.
/// So instead of creating a separate svgdom::Node to usvg::Node converter
/// just for `clipPath` we will remove invalid children beforehand.
fn prepare_clip_path(
    doc: &mut svgdom::Document,
) {
    fn is_valid_child(eid: EId) -> bool {
        // `line` doesn't impact rendering because stroke is always disabled
        // for `clipPath` children. So we should remove it too.
        match eid {
              EId::Rect
            | EId::Circle
            | EId::Ellipse
            | EId::Polyline
            | EId::Polygon
            | EId::Path
            | EId::Text => true,
            _ => false,
        }
    }

    // Remove invalid children.
    for node in doc.root().descendants().filter(|n| n.is_tag_name(EId::ClipPath)) {
        let mut curr = node.first_child();
        while let Some(n) = curr {
            curr = n.next_sibling();

            let eid = match n.tag_id() {
                Some(eid) => eid,
                None => {
                    // Not an SVG element? Remove it.
                    doc.remove_node(n);
                    continue;
                }
            };

            // Keep groups generated during `use` resolving.
            if eid == EId::G && n.has_attribute("usvg-use") {
                // Remove `use` elements that reference elements not supported by `clipPath`.
                if let Some(child_eid) = n.first_child().and_then(|n| n.tag_id()) {
                    if !is_valid_child(child_eid) {
                        doc.remove_node(n);
                    }
                }

                continue;
            }

            if !is_valid_child(eid) {
                doc.remove_node(n);
            }
        }
    }
}

fn prepare_text(
    doc: &mut svgdom::Document,
) {
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
    }

    sanitize_text(doc.root(), doc);
}

fn regroup_elements(
    doc: &mut svgdom::Document,
    parent: &svgdom::Node,
) {
    fn has_links(node: &svgdom::Node) -> bool {
           node.has_attribute(AId::ClipPath)
        || node.has_attribute(AId::Mask)
        || node.has_attribute(AId::Filter)
    }

    let g_attrs = [AId::ClipPath, AId::Mask, AId::Filter, AId::Opacity];

    let mut ids = Vec::new();
    let mut curr_node = parent.first_child();
    while let Some(mut node) = curr_node {
        curr_node = node.next_sibling();
        ids.clear();

        if node.has_children() {
            regroup_elements(doc, &node);
        }

        if !node.is_graphic() {
            continue;
        }

        let opacity = {
            let opacity = match node.attributes().get_value(AId::Opacity) {
                Some(AValue::Number(n)) => *n,
                _ => 1.0,
            };

            f64_bound(0.0, opacity, 1.0)
        };

        if opacity.fuzzy_eq(&1.0) && !has_links(&node) {
            continue;
        }

        let mut g_node = doc.create_element(EId::G);

        {
            let attrs = node.attributes();
            for aid in &g_attrs {
                if *aid == AId::Opacity && opacity.fuzzy_eq(&1.0) {
                    continue;
                }

                if let Some(attr) = attrs.get(*aid) {
                    g_node.set_attribute(attr.clone());
                    ids.push(*aid);
                }
            }

            if let Some(ts) = attrs.get(AId::Transform) {
                g_node.set_attribute(ts.clone());
                ids.push(AId::Transform);
            }
        }

        for id in &ids {
            node.remove_attribute(*id);
        }

        node.insert_before(g_node.clone());
        node.detach();
        g_node.append(node.clone());
    }
}

/// Resolves the `currentColor` attribute.
///
/// The function will fallback to a default value when possible.
fn resolve_current_color(
    doc: &svgdom::Document,
) {
    fn resolve_color(node: &svgdom::Node, aid: AId) -> Option<svgdom::Color> {
        if let Some(n) = node.ancestors().find(|n| n.has_attribute(AId::Color)) {
            n.attributes().get_color(AId::Color)
        } else {
            match aid {
                  AId::Fill
                | AId::Stroke
                | AId::FloodColor
                | AId::StopColor
                | AId::LightingColor => Some(svgdom::Color::black()),
                _ => None,
            }
        }
    }

    let mut ids = Vec::new();

    for (_, mut node) in doc.root().descendants().svg() {
        ids.clear();

        {
            let attrs = node.attributes();
            for (aid, attr) in attrs.iter().svg() {
                match attr.value {
                    AValue::CurrentColor => {
                        ids.push(aid);
                    }
                    AValue::Paint(_, fallback) => {
                        if let Some(svgdom::PaintFallback::CurrentColor) = fallback {
                            ids.push(aid);
                        }
                    }
                    _ => {}
                }
            }
        }

        for id in &ids {
            match resolve_color(&node, *id) {
                Some(v) => {
                    let av = node.attributes().get_value(*id).cloned().unwrap();
                    match av {
                        AValue::CurrentColor => {
                            node.set_attribute((*id, v));
                        }
                        AValue::Paint(link, _) => {
                            let fallback = Some(svgdom::PaintFallback::Color(v));
                            node.set_attribute((*id, (link.clone(), fallback)));
                        }
                        _ => {}
                    }
                }
                None => {
                    warn!("Failed to resolve currentColor for '{}'. Removing it.", id);
                    node.remove_attribute(*id);
                }
            }
        }
    }
}

/// Resolves the `inherit` attribute value.
///
/// The function will fallback to a default value when possible.
fn resolve_inherit(
    doc: &svgdom::Document,
) {
    let mut ids = Vec::new();
    for (_, mut node) in doc.root().descendants().svg() {
        ids.clear();

        for (aid, attr) in node.attributes().iter().svg() {
            if let AValue::Inherit = attr.value {
                ids.push(aid);
            }
        }

        for id in &ids {
            _resolve_inherit(&mut node, *id);
        }
    }
}

fn _resolve_inherit(
    node: &mut svgdom::Node,
    aid: AId,
) {
    if aid.is_inheritable() {
        if let Some(n) = node.ancestors().skip(1).find(|n| n.has_attribute(aid)) {
            let attrs = n.attributes();
            if let Some(attr) = attrs.get(aid) {
                node.try_set_attribute(attr);
                return;
            }
        }
    } else {
        if let Some(parent) = node.parent() {
            let attrs = parent.attributes();
            if let Some(attr) = attrs.get(aid) {
                node.try_set_attribute(attr);
                return;
            }
        }
    }

    match svgdom::Attribute::new_default(aid) {
        Some(a) => node.set_attribute((aid, a.value)),
        None => {
            warn!("Failed to resolve attribute: {}. Removing it.",
                  node.attributes().get(aid).unwrap());
            node.remove_attribute(aid);
        }
    }
}

/// Resolves the root `svg` element attributes.
///
/// In the `usvg`, the root `svg` element can't have any style attributes,
/// so we have to create a new root group and move all non-inheritable attributes into it.
fn resolve_root_style_attributes(
    doc: &mut svgdom::Document,
    svg: &mut svgdom::Node,
) {
    // Create a new group only when needed.
    let has_any =
           svg.has_attribute(AId::ClipPath)
        || svg.has_attribute(AId::Filter)
        || svg.has_attribute(AId::Mask)
        || svg.has_attribute(AId::Opacity)
        || svg.has_attribute(AId::Transform);

    if !has_any {
        return;
    }

    let mut g = doc.create_element(EId::G);

    let children: Vec<_> = svg.children().collect();
    for child in children {
        g.append(child);
    }

    svg.append(g.clone());

    svg.move_attribute_to(AId::ClipPath, &mut g);
    svg.move_attribute_to(AId::Filter, &mut g);
    svg.move_attribute_to(AId::Mask, &mut g);
    svg.move_attribute_to(AId::Opacity, &mut g);
    svg.move_attribute_to(AId::Transform, &mut g);
}

fn resolve_tref(
    doc: &mut svgdom::Document,
) {
    for mut tref in doc.root().descendants().filter(|n| n.is_tag_name(EId::Tref)) {
        let av = tref.attributes().get_value(AId::Href).cloned();
        let text_elem = if let Some(AValue::Link(ref link)) = av {
            link.clone()
        } else {
            continue;
        };

        // 'All character data within the referenced element, including character data enclosed
        // within additional markup, will be rendered.'
        //
        // So we don't care about attributes and everything. Just collecting text nodes data.
        let mut text = String::new();
        for node in text_elem.descendants().filter(|n| n.is_text()) {
            text.push_str(&node.text());
        }

        // `tref` must not have any children, so we have to remove all of them.
        doc.drain(tref.clone(), |_| true);

        let text_node = doc.create_node(svgdom::NodeType::Text, text);
        tref.append(text_node);

        tref.set_tag_name(EId::Tspan);
        tref.remove_attribute(AId::Href);
    }
}

fn resolve_use(
    doc: &mut svgdom::Document,
) {
    let mut rm_nodes = Vec::new();

    // 'use' elements can be linked in any order,
    // so we have to process the tree until all 'use' are solved.
    let mut is_any_resolved = true;
    while is_any_resolved {
        rm_nodes.clear();

        let root = doc.root().clone();
        is_any_resolved = _resolve_use(doc, root, &mut rm_nodes);

        // Remove unresolved 'use' elements, since there is not need
        // to keep them around and they will be skipped anyway.
        for node in &mut rm_nodes {
            doc.remove_node(node.clone());
        }
    }

    remove_invalid_use(doc);
}

fn _resolve_use(
    doc: &mut svgdom::Document,
    parent: svgdom::Node,
    rm_nodes: &mut Vec<svgdom::Node>,
) -> bool {
    let mut is_any_resolved = false;

    for mut node in parent.children() {
        if node.is_tag_name(EId::Use) {
            let av = node.attributes().get_value(AId::Href).cloned();
            if let Some(AValue::Link(mut link)) = av {
                // Ignore 'use' elements linked to other 'use' elements.
                if link.is_tag_name(EId::Use) {
                    continue;
                }

                // TODO: this
                // We don't support 'use' elements linked to 'svg' element.
                if link.is_tag_name(EId::Svg) {
                    warn!("'use' element linked to an 'svg' element is not supported. Skipped.");
                    rm_nodes.push(node.clone());
                    continue;
                }

                // Check that none of the linked node's children reference current `use` node
                // via other `use` node.
                //
                // Example:
                // <g id="g1">
                //     <use xlink:href="#use1" id="use2"/>
                // </g>
                // <use xlink:href="#g1" id="use1"/>
                //
                // `use2` should be removed.
                //
                // Also, child should not reference its parent:
                // <g id="g1">
                //     <use xlink:href="#g1" id="use1"/>
                // </g>
                //
                // `use1` should be removed.
                let mut is_recursive = false;
                for link_child in link.descendants().skip(1).filter(|n| n.is_tag_name(EId::Use)) {
                    let av = link_child.attributes().get_value(AId::Href).cloned();
                    if let Some(AValue::Link(link2)) = av {
                        if link2 == node || link2 == link {
                            is_recursive = true;
                            break;
                        }
                    }
                }

                if is_recursive {
                    warn!("Recursive 'use' detected. '{}' will be deleted.", node.id());
                    rm_nodes.push(node.clone());
                    continue;
                }

                __resolve_use(doc, &mut node, &mut link);
                is_any_resolved = true;
            }
        }

        if _resolve_use(doc, node, rm_nodes) {
            is_any_resolved = true;
        }
    }

    is_any_resolved
}

fn __resolve_use(
    doc: &mut svgdom::Document,
    use_node: &mut svgdom::Node,
    linked_node: &mut svgdom::Node,
) {
    use_node.set_tag_name(EId::G);

    // Remember that this group was 'use' before.
    use_node.set_attribute(("usvg-use", 1));

    if linked_node.is_tag_name(EId::Symbol) {
        use_node.set_attribute(("usvg-symbol", 1));

        let new_node = doc.copy_node_deep(linked_node.clone());
        for child in new_node.children() {
            use_node.append(child);
        }
    } else {
        let new_node = doc.copy_node_deep(linked_node.clone());
        use_node.append(new_node);
    }
}

fn remove_invalid_use(
    doc: &mut svgdom::Document,
) {
    fn _rm(doc: &mut svgdom::Document) -> usize {
        let root = doc.root();
        doc.drain(root, |n| {
            if n.is_tag_name(EId::Use) {
                if !n.has_attribute(AId::Href) {
                    // Remove 'use' elements without an 'xlink:href' attribute.
                    return true;
                } else {
                    // Remove 'use' elements with an invalid 'xlink:href' attribute.
                    let attrs = n.attributes();
                    if let Some(&AValue::Link(_)) = attrs.get_value(AId::Href) {
                        // Nothing.
                    } else {
                        // NOTE: actually, an attribute with 'String' type is valid
                        // if it contain a path to an external file, like '../img.svg#rect1',
                        // but we don't support external SVG, so we treat them as invalid.
                        return true;
                    }
                }
            }

            false
        })
    }

    // 'use' can be linked to another 'use' and if it was removed
    // the first one will became invalid, so we need to check DOM again.
    // Loop until there are no drained elements.
    while _rm(doc) > 0 {}
}

/// We don't care about `a` elements, but we can't just remove them.
/// So, if an `a` element is inside a `text` - change the tag name to `tspan`.
/// Otherwise, to `g`.
fn ungroup_a(
    doc: &svgdom::Document,
) {
    for (id, mut node) in doc.root().descendants().svg() {
        if id != EId::A {
            continue;
        }

        node.remove_attribute(AId::Href);

        if node.ancestors().skip(1).any(|n| n.is_tag_name(EId::Text)) {
            node.set_tag_name(EId::Tspan);
        } else {
            node.set_tag_name(EId::G);
        }
    }
}
