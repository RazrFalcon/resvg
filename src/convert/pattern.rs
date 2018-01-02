// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom::{
    self,
    FuzzyEq,
};

use dom;

use short::{
    AId,
};

use math::{
    Rect,
};

use traits::{
    GetValue,
    GetViewBox,
};


pub fn convert(
    node: &svgdom::Node,
) -> Option<dom::RefElement> {
    let ref attrs = node.attributes();

    let rect = convert_rect(attrs);
    debug_assert!(!rect.w.is_fuzzy_zero());
    debug_assert!(!rect.h.is_fuzzy_zero());

    let elem = dom::RefElement {
        id: node.id().clone(),
        kind: dom::RefElementKind::Pattern(dom::Pattern {
            units: super::convert_element_units(&attrs, AId::PatternUnits),
            content_units: super::convert_element_units(&attrs, AId::PatternContentUnits),
            transform: attrs.get_transform(AId::PatternTransform).unwrap_or_default(),
            rect,
            view_box: node.get_viewbox().ok(),
            children: Vec::new(), // children will be converted later
        }),
    };

    Some(elem)
}

pub fn convert_rect(attrs: &svgdom::Attributes) -> Rect {
    Rect {
        x: attrs.get_number(AId::X).unwrap_or(0.0),
        y: attrs.get_number(AId::Y).unwrap_or(0.0),
        w: attrs.get_number(AId::Width).unwrap_or(0.0),
        h: attrs.get_number(AId::Height).unwrap_or(0.0),
    }
}
