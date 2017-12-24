// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom;

use dom;

use short::{
    AId,
    EId,
};

use traits::{
    GetValue,
};

use super::{
    path,
    text,
    shapes,
};


pub fn convert(node: &svgdom::Node) -> Option<dom::RefElement> {
    let attrs = node.attributes();

    if let Some(children) = convert_children(node) {
        let elem = dom::RefElement {
            id: node.id().clone(),
            kind: dom::RefElementKind::ClipPath(dom::ClipPath {
                units: super::convert_element_units(&attrs, AId::ClipPathUnits),
                transform: attrs.get_transform(AId::Transform).unwrap_or_default(),
                children,
            }),
        };

        Some(elem)
    } else {
        warn!("The '{}' clipPath has no valid children. Skipped.", node.id());
        None
    }
}

fn convert_children(node: &svgdom::Node) -> Option<Vec<dom::Element>> {
    let mut nodes: Vec<dom::Element> = Vec::new();

    for (id, node) in node.children().svg() {
        match id {
              EId::Line
            | EId::Rect
            | EId::Polyline
            | EId::Polygon
            | EId::Circle
            | EId::Ellipse => {
                if let Some(d) = shapes::convert(&node) {
                    if let Ok(elem) = path::convert(&[], &node, d) {
                        nodes.push(elem);
                    }
                }
            }
            EId::Path => {
                let attrs = node.attributes();
                if let Some(d) = attrs.get_path(AId::D) {
                    if let Ok(elem) = path::convert(&[], &node, d.clone()) {
                        nodes.push(elem);
                    }
                }
            }
            EId::Text => {
                if let Some(elem) = text::convert(&[], &node) {
                    nodes.push(elem);
                }
            }
            _ => {
                warn!("Skipping the '{}' clipPath invalid child element '{}'.", node.id(), id);
                continue;
            }
        }
    }

    if nodes.is_empty() {
        return None;
    }

    Some(nodes)
}
