use std::collections::HashMap;

use roxmltree::Error;

use crate::{
    Attribute, AttributeId, Document, ElementId, Node, NodeData, NodeId, NodeKind, ShortRange,
};

const SVG_NS: &str = "http://www.w3.org/2000/svg";
const XLINK_NS: &str = "http://www.w3.org/1999/xlink";
const XML_NAMESPACE_NS: &str = "http://www.w3.org/XML/1998/namespace";

impl<'input> Document<'input> {
    /// Parses a [`Document`] from a string.
    pub fn parse_str(text: &'input str) -> Result<Document<'input>, Error> {
        let xml = roxmltree::Document::parse(text)?;
        parse(&xml)
    }

    /// Parses a [`Document`] from a [`roxmltree::Document`].
    pub fn parse_tree(xml: &roxmltree::Document<'input>) -> Result<Document<'input>, Error> {
        parse(xml)
    }

    pub(crate) fn append(&mut self, parent_id: NodeId, kind: NodeKind) -> NodeId {
        let new_child_id = NodeId::from(self.nodes.len());
        self.nodes.push(NodeData {
            parent: Some(parent_id),
            next_sibling: None,
            children: None,
            kind,
        });

        let last_child_id = self.nodes[parent_id.get_usize()].children.map(|(_, id)| id);

        if let Some(id) = last_child_id {
            self.nodes[id.get_usize()].next_sibling = Some(new_child_id);
        }

        self.nodes[parent_id.get_usize()].children = Some(
            if let Some((first_child_id, _)) = self.nodes[parent_id.get_usize()].children {
                (first_child_id, new_child_id)
            } else {
                (new_child_id, new_child_id)
            },
        );

        new_child_id
    }

    fn append_attribute(&mut self, name: AttributeId, value: roxmltree::StringStorage<'input>) {
        self.attrs.push(Attribute { name, value });
    }
}

fn parse<'input>(xml: &roxmltree::Document<'input>) -> Result<Document<'input>, Error> {
    let mut doc = Document {
        nodes: Vec::new(),
        attrs: Vec::new(),
        links: HashMap::new(),
    };

    // Add a root node.
    doc.nodes.push(NodeData {
        parent: None,
        next_sibling: None,
        children: None,
        kind: NodeKind::Root,
    });

    let style_sheet = resolve_css(xml);

    parse_xml_node_children(
        xml.root(),
        xml.root(),
        doc.root().id,
        &style_sheet,
        false,
        0,
        &mut doc,
    )?;

    // Check that the root element is `svg`.
    match doc.root().first_element_child() {
        Some(child) => {
            if child.tag_name() != Some(ElementId::Svg) {
                return Err(roxmltree::Error::NoRootNode.into());
            }
        }
        None => return Err(roxmltree::Error::NoRootNode.into()),
    }

    // Collect all elements with `id` attribute.
    let mut links = HashMap::new();
    for node in doc.descendants() {
        if let Some(id) = node.attribute::<&str>(AttributeId::Id) {
            links.insert(id.to_string(), node.id);
        }
    }
    doc.links = links;

    fix_recursive_patterns(&mut doc);
    fix_recursive_links(ElementId::ClipPath, AttributeId::ClipPath, &mut doc);
    fix_recursive_links(ElementId::Mask, AttributeId::Mask, &mut doc);
    fix_recursive_links(ElementId::Filter, AttributeId::Filter, &mut doc);
    fix_recursive_fe_image(&mut doc);

    Ok(doc)
}

pub(crate) fn parse_tag_name(node: roxmltree::Node) -> Option<ElementId> {
    if !node.is_element() {
        return None;
    }

    if node.tag_name().namespace() != Some(SVG_NS) {
        return None;
    }

    ElementId::from_str(node.tag_name().name())
}

fn parse_xml_node_children<'input>(
    parent: roxmltree::Node<'_, 'input>,
    origin: roxmltree::Node,
    parent_id: NodeId,
    style_sheet: &simplecss::StyleSheet,
    ignore_ids: bool,
    depth: u32,
    doc: &mut Document<'input>,
) -> Result<(), Error> {
    for node in parent.children() {
        parse_xml_node(node, origin, parent_id, style_sheet, ignore_ids, depth, doc)?;
    }

    Ok(())
}

fn parse_xml_node<'input>(
    node: roxmltree::Node<'_, 'input>,
    origin: roxmltree::Node,
    parent_id: NodeId,
    style_sheet: &simplecss::StyleSheet,
    ignore_ids: bool,
    depth: u32,
    doc: &mut Document<'input>,
) -> Result<(), Error> {
    if depth > 1024 {
        return Err(Error::NodesLimitReached);
    }

    let mut tag_name = match parse_tag_name(node) {
        Some(id) => id,
        None => return Ok(()),
    };

    if tag_name == ElementId::Style {
        return Ok(());
    }

    // TODO: remove?
    // Treat links as groups.
    if tag_name == ElementId::A {
        tag_name = ElementId::G;
    }

    let node_id = parse_svg_element(node, parent_id, tag_name, style_sheet, ignore_ids, doc)?;
    if tag_name == ElementId::Text {
        crate::text::parse_svg_text_element(node, node_id, style_sheet, doc)?;
    } else if tag_name == ElementId::Use {
        parse_svg_use_element(node, origin, node_id, style_sheet, depth + 1, doc)?;
    } else {
        parse_xml_node_children(
            node,
            origin,
            node_id,
            style_sheet,
            ignore_ids,
            depth + 1,
            doc,
        )?;
    }

    Ok(())
}

pub(crate) fn parse_svg_element<'input>(
    xml_node: roxmltree::Node<'_, 'input>,
    parent_id: NodeId,
    tag_name: ElementId,
    style_sheet: &simplecss::StyleSheet,
    ignore_ids: bool,
    doc: &mut Document<'input>,
) -> Result<NodeId, Error> {
    let attrs_start_idx = doc.attrs.len();

    // Copy presentational attributes first.
    for attr in xml_node.attributes() {
        match attr.namespace() {
            None | Some(SVG_NS) | Some(XLINK_NS) | Some(XML_NAMESPACE_NS) => {}
            _ => continue,
        }

        let aid = match AttributeId::from_str(attr.name()) {
            Some(v) => v,
            None => continue,
        };

        // During a `use` resolving, all `id` attributes must be ignored.
        // Otherwise we will get elements with duplicated id's.
        if ignore_ids && aid == AttributeId::Id {
            continue;
        }

        // For some reason those properties are allowed only inside a `style` attribute and CSS.
        if matches!(
            aid,
            AttributeId::MixBlendMode | AttributeId::Isolation | AttributeId::FontKerning
        ) {
            continue;
        }

        append_attribute(parent_id, tag_name, aid, attr.value_storage().clone(), doc);
    }

    let mut insert_attribute = |aid, value: &str| {
        // Check that attribute already exists.
        let idx = doc.attrs[attrs_start_idx..]
            .iter_mut()
            .position(|a| a.name == aid);

        // Append an attribute as usual.
        let added = append_attribute(
            parent_id,
            tag_name,
            aid,
            roxmltree::StringStorage::new_owned(value),
            doc,
        );

        // Check that attribute was actually added, because it could be skipped.
        if added {
            if let Some(idx) = idx {
                // Swap the last attribute with an existing one.
                let last_idx = doc.attrs.len() - 1;
                doc.attrs.swap(attrs_start_idx + idx, last_idx);
                // Remove last.
                doc.attrs.pop();
            }
        }
    };

    // Apply CSS.
    for rule in &style_sheet.rules {
        if rule.selector.matches(&XmlNode(xml_node)) {
            for declaration in &rule.declarations {
                // TODO: perform XML attribute normalization
                if let Some(aid) = AttributeId::from_str(declaration.name) {
                    // Parse only the presentation attributes.
                    if aid.is_presentation() {
                        insert_attribute(aid, declaration.value);
                    }
                } else if declaration.name == "marker" {
                    insert_attribute(AttributeId::MarkerStart, declaration.value);
                    insert_attribute(AttributeId::MarkerMid, declaration.value);
                    insert_attribute(AttributeId::MarkerEnd, declaration.value);
                }
            }
        }
    }

    // Split a `style` attribute.
    if let Some(value) = xml_node.attribute("style") {
        for declaration in simplecss::DeclarationTokenizer::from(value) {
            // TODO: preform XML attribute normalization
            if let Some(aid) = AttributeId::from_str(declaration.name) {
                // Parse only the presentation attributes.
                if aid.is_presentation() {
                    insert_attribute(aid, declaration.value);
                }
            }
        }
    }

    if doc.nodes.len() > 1_000_000 {
        return Err(Error::NodesLimitReached);
    }

    let node_id = doc.append(
        parent_id,
        NodeKind::Element {
            tag_name,
            attributes: ShortRange::new(attrs_start_idx as u32, doc.attrs.len() as u32),
        },
    );

    Ok(node_id)
}

fn append_attribute<'input>(
    parent_id: NodeId,
    tag_name: ElementId,
    aid: AttributeId,
    value: roxmltree::StringStorage<'input>,
    doc: &mut Document<'input>,
) -> bool {
    match aid {
        // The `style` attribute will be split into attributes, so we don't need it.
        AttributeId::Style |
        // No need to copy a `class` attribute since CSS were already resolved.
        AttributeId::Class => return false,
        _ => {}
    }

    // Ignore `xlink:href` on `tspan` (which was originally `tref` or `a`),
    // because we will convert `tref` into `tspan` anyway.
    if tag_name == ElementId::Tspan && aid == AttributeId::Href {
        return false;
    }

    if aid.allows_inherit_value() && &*value == "inherit" {
        return resolve_inherit(parent_id, aid, doc);
    }

    doc.append_attribute(aid, value);
    true
}

fn resolve_inherit<'input>(
    parent_id: NodeId,
    aid: AttributeId,
    doc: &mut Document<'input>,
) -> bool {
    if aid.is_inheritable() {
        // Inheritable attributes can inherit a value from an any ancestor.
        let node_id = doc
            .get(parent_id)
            .ancestors()
            .find(|n| n.has_attribute(aid))
            .map(|n| n.id);
        if let Some(node_id) = node_id {
            if let Some(attr) = doc
                .get(node_id)
                .attributes()
                .iter()
                .find(|a| a.name == aid)
                .cloned()
            {
                doc.attrs.push(Attribute {
                    name: aid,
                    value: attr.value,
                });

                return true;
            }
        }
    } else {
        // Non-inheritable attributes can inherit a value only from a direct parent.
        if let Some(attr) = doc
            .get(parent_id)
            .attributes()
            .iter()
            .find(|a| a.name == aid)
            .cloned()
        {
            doc.attrs.push(Attribute {
                name: aid,
                value: attr.value,
            });

            return true;
        }
    }

    // Fallback to a default value if possible.
    let value = match aid {
        AttributeId::ImageRendering | AttributeId::ShapeRendering | AttributeId::TextRendering => {
            "auto"
        }

        AttributeId::ClipPath
        | AttributeId::Filter
        | AttributeId::MarkerEnd
        | AttributeId::MarkerMid
        | AttributeId::MarkerStart
        | AttributeId::Mask
        | AttributeId::Stroke
        | AttributeId::StrokeDasharray
        | AttributeId::TextDecoration => "none",

        AttributeId::FontStretch
        | AttributeId::FontStyle
        | AttributeId::FontVariant
        | AttributeId::FontWeight
        | AttributeId::LetterSpacing
        | AttributeId::WordSpacing => "normal",

        AttributeId::Fill | AttributeId::FloodColor | AttributeId::StopColor => "black",

        AttributeId::FillOpacity
        | AttributeId::FloodOpacity
        | AttributeId::Opacity
        | AttributeId::StopOpacity
        | AttributeId::StrokeOpacity => "1",

        AttributeId::ClipRule | AttributeId::FillRule => "nonzero",

        AttributeId::BaselineShift => "baseline",
        AttributeId::ColorInterpolationFilters => "linearRGB",
        AttributeId::Direction => "ltr",
        AttributeId::Display => "inline",
        AttributeId::FontSize => "medium",
        AttributeId::Overflow => "visible",
        AttributeId::StrokeDashoffset => "0",
        AttributeId::StrokeLinecap => "butt",
        AttributeId::StrokeLinejoin => "miter",
        AttributeId::StrokeMiterlimit => "4",
        AttributeId::StrokeWidth => "1",
        AttributeId::TextAnchor => "start",
        AttributeId::Visibility => "visible",
        AttributeId::WritingMode => "lr-tb",
        _ => return false,
    };

    doc.append_attribute(aid, roxmltree::StringStorage::Borrowed(value));
    true
}

fn resolve_href<'a, 'input: 'a>(
    node: roxmltree::Node<'a, 'input>,
) -> Option<roxmltree::Node<'a, 'input>> {
    let link_value = node
        .attribute((XLINK_NS, "href"))
        .or_else(|| node.attribute("href"))?;

    let link_id = svgtypes::IRI::from_str(link_value).ok()?.0;

    // We're using `descendants` each time instead of HashTable because
    // we have to preserve the original elements order.
    // See tests/svg/e-use-024.svg
    //
    // Technically we can use https://crates.io/crates/hashlink,
    // but this is an additional dependency.
    // And performance even on huge files is still good enough.
    node.document()
        .descendants()
        .find(|n| n.attribute("id") == Some(link_id))
}

fn parse_svg_use_element<'input>(
    node: roxmltree::Node<'_, 'input>,
    origin: roxmltree::Node,
    parent_id: NodeId,
    style_sheet: &simplecss::StyleSheet,
    depth: u32,
    doc: &mut Document<'input>,
) -> Result<(), Error> {
    let link = match resolve_href(node) {
        Some(v) => v,
        None => return Ok(()),
    };

    if link == node || link == origin {
        log::warn!(
            "Recursive 'use' detected. '{}' will be skipped.",
            node.attribute((SVG_NS, "id")).unwrap_or_default()
        );
        return Ok(());
    }

    // Make sure we're linked to an SVG element.
    if parse_tag_name(link).is_none() {
        return Ok(());
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
    for link_child in link
        .descendants()
        .skip(1)
        .filter(|n| n.has_tag_name((SVG_NS, "use")))
    {
        if let Some(link2) = resolve_href(link_child) {
            if link2 == node || link2 == link {
                is_recursive = true;
                break;
            }
        }
    }

    if is_recursive {
        log::warn!(
            "Recursive 'use' detected. '{}' will be skipped.",
            node.attribute((SVG_NS, "id")).unwrap_or_default()
        );
        return Ok(());
    }

    parse_xml_node(link, node, parent_id, style_sheet, true, depth + 1, doc)
}

fn resolve_css<'a>(xml: &'a roxmltree::Document<'a>) -> simplecss::StyleSheet<'a> {
    let mut sheet = simplecss::StyleSheet::new();

    for node in xml.descendants().filter(|n| n.has_tag_name("style")) {
        match node.attribute("type") {
            Some("text/css") => {}
            Some(_) => continue,
            None => {}
        }

        let text = match node.text() {
            Some(v) => v,
            None => continue,
        };

        sheet.parse_more(text);
    }

    sheet
}

struct XmlNode<'a, 'input: 'a>(roxmltree::Node<'a, 'input>);

impl simplecss::Element for XmlNode<'_, '_> {
    fn parent_element(&self) -> Option<Self> {
        self.0.parent_element().map(XmlNode)
    }

    fn prev_sibling_element(&self) -> Option<Self> {
        self.0.prev_sibling_element().map(XmlNode)
    }

    fn has_local_name(&self, local_name: &str) -> bool {
        self.0.tag_name().name() == local_name
    }

    fn attribute_matches(&self, local_name: &str, operator: simplecss::AttributeOperator) -> bool {
        match self.0.attribute(local_name) {
            Some(value) => operator.matches(value),
            None => false,
        }
    }

    fn pseudo_class_matches(&self, class: simplecss::PseudoClass) -> bool {
        match class {
            simplecss::PseudoClass::FirstChild => self.prev_sibling_element().is_none(),
            // TODO: lang
            _ => false, // Since we are querying a static SVG we can ignore other pseudo-classes.
        }
    }
}

fn fix_recursive_patterns(doc: &mut Document) {
    while let Some(node_id) = find_recursive_pattern(AttributeId::Fill, doc) {
        let idx = doc.get(node_id).attribute_id(AttributeId::Fill).unwrap();
        doc.attrs[idx].value = roxmltree::StringStorage::Borrowed("none");
    }

    while let Some(node_id) = find_recursive_pattern(AttributeId::Stroke, doc) {
        let idx = doc.get(node_id).attribute_id(AttributeId::Stroke).unwrap();
        doc.attrs[idx].value = roxmltree::StringStorage::Borrowed("none");
    }
}

fn find_recursive_pattern(aid: AttributeId, doc: &mut Document) -> Option<NodeId> {
    for pattern_node in doc
        .root()
        .descendants()
        .filter(|n| n.tag_name() == Some(ElementId::Pattern))
    {
        for node in pattern_node.descendants() {
            if let Some(svgtypes::Paint::FuncIRI(link_id, _)) = node.attribute(aid) {
                if link_id == pattern_node.element_id() {
                    // If a pattern child has a link to the pattern itself
                    // then we have to replace it with `none`.
                    // Otherwise we will get endless loop/recursion and stack overflow.
                    return Some(node.id);
                } else {
                    // Check that linked node children doesn't link this pattern.
                    if let Some(linked_node) = doc.element_by_id(link_id) {
                        for node2 in linked_node.descendants() {
                            if let Some(svgtypes::Paint::FuncIRI(link_id2, _)) =
                                node2.attribute(aid)
                            {
                                if link_id2 == pattern_node.element_id() {
                                    return Some(node2.id);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

fn fix_recursive_links(eid: ElementId, aid: AttributeId, doc: &mut Document) {
    while let Some(node_id) = find_recursive_link(eid, aid, doc) {
        let idx = doc.get(node_id).attribute_id(aid).unwrap();
        doc.attrs[idx].value = roxmltree::StringStorage::Borrowed("none");
    }
}

fn find_recursive_link(eid: ElementId, aid: AttributeId, doc: &Document) -> Option<NodeId> {
    for node in doc
        .root()
        .descendants()
        .filter(|n| n.tag_name() == Some(eid))
    {
        for child in node.descendants() {
            if let Some(link) = child.attribute::<Node>(aid) {
                if link == node {
                    // If an element child has a link to the element itself
                    // then we have to replace it with `none`.
                    // Otherwise we will get endless loop/recursion and stack overflow.
                    return Some(child.id);
                } else {
                    // Check that linked node children doesn't link this element.
                    for node2 in link.descendants() {
                        if let Some(link2) = node2.attribute::<Node>(aid) {
                            if link2 == node {
                                return Some(node2.id);
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Detects cases like:
///
/// ```xml
/// <filter id="filter1">
///   <feImage xlink:href="#rect1"/>
/// </filter>
/// <rect id="rect1" x="36" y="36" width="120" height="120" fill="green" filter="url(#filter1)"/>
/// ```
fn fix_recursive_fe_image(doc: &mut Document) {
    let mut ids = Vec::new();
    for fe_node in doc
        .root()
        .descendants()
        .filter(|n| n.tag_name() == Some(ElementId::FeImage))
    {
        if let Some(link) = fe_node.attribute::<Node>(AttributeId::Href) {
            if let Some(filter_uri) = link.attribute::<&str>(AttributeId::Filter) {
                let filter_id = fe_node.parent().unwrap().element_id().to_string();
                for func in svgtypes::FilterValueListParser::from(filter_uri) {
                    if let Ok(func) = func {
                        if let svgtypes::FilterValue::Url(url) = func {
                            if url == filter_id {
                                ids.push(link.id);
                            }
                        }
                    }
                }
            }
        }
    }

    for id in ids {
        let idx = doc.get(id).attribute_id(AttributeId::Filter).unwrap();
        doc.attrs[idx].value = roxmltree::StringStorage::Borrowed("none");
    }
}
