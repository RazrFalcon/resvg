// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom::{
    Length,
    Transform,
};

use super::prelude::*;


/// Resolves default `svg` attributes.
pub fn resolve_svg_attributes(doc: &Document) {
    for mut node in doc.root().descendants().filter(|n| n.is_tag_name(EId::Svg)) {
        node.set_attribute_if_none((AId::X, 0.0));
        node.set_attribute_if_none((AId::Y, 0.0));
        node.set_attribute_if_none((AId::Width,  Length::new(100.0, Unit::Percent)));
        node.set_attribute_if_none((AId::Height, Length::new(100.0, Unit::Percent)));
    }
}

pub fn prepare_nested_svg(doc: &mut Document, svg: &Node) {
    for mut node in svg.descendants().skip(1).filter(|n| n.is_tag_name(EId::Svg)) {
        let x = node.attributes().get_number_or(AId::X, 0.0);
        let y = node.attributes().get_number_or(AId::Y, 0.0);
        node.append_transform(Transform::new_translate(x, y));

        if let Some(ts) = node.get_viewbox_transform() {
            node.append_transform(ts);
        }

        node.set_tag_name(EId::G);

        super::clip_element::clip_element(doc, &mut node);
    }
}
