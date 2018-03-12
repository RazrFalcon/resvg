// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom;

// self
use tree::prelude::*;
use short::{
    AId,
};
use traits::{
    GetValue,
    GetViewBox,
};


pub fn convert(
    node: &svgdom::Node,
    rtree: &mut tree::RenderTree,
) -> tree::NodeId {
    let ref attrs = node.attributes();

    let view_box = node.get_viewbox().map(|vb|
        tree::ViewBox {
            rect: vb,
            aspect: super::convert_aspect(attrs),
        }
    );

    let rect = super::convert_rect(attrs);

    rtree.append_to_defs(tree::NodeKind::Pattern(tree::Pattern {
        id: node.id().clone(),
        units: super::convert_element_units(&attrs, AId::PatternUnits),
        content_units: super::convert_element_units(&attrs, AId::PatternContentUnits),
        transform: attrs.get_transform(AId::PatternTransform).unwrap_or_default(),
        rect,
        view_box,
    }))
}
