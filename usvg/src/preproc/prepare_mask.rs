// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom::{
    Length,
};

use super::prelude::*;


/// Resolves default `mask` attributes.
pub fn resolve_mask_attributes(doc: &mut Document) {
    for mut node in doc.root().descendants().filter(|n| n.is_tag_name(EId::Mask)) {
        let units = node.attributes().get_str(AId::MaskUnits)
                        .unwrap_or("objectBoundingBox").to_string();

        if units == "objectBoundingBox" {
            node.set_attribute_if_none((AId::X, -0.1));
            node.set_attribute_if_none((AId::Y, -0.1));
            node.set_attribute_if_none((AId::Width, 1.2));
            node.set_attribute_if_none((AId::Height, 1.2));
        } else {
            node.set_attribute_if_none((AId::X, Length::new(-10.0, Unit::Percent)));
            node.set_attribute_if_none((AId::Y, Length::new(-10.0, Unit::Percent)));
            node.set_attribute_if_none((AId::Width, Length::new(120.0, Unit::Percent)));
            node.set_attribute_if_none((AId::Height, Length::new(120.0, Unit::Percent)));
        }

        node.set_attribute((AId::MaskUnits, units));

        let c_units = node.attributes().get_str(AId::MaskContentUnits)
                          .unwrap_or("userSpaceOnUse").to_string();
        node.set_attribute((AId::MaskContentUnits, c_units));
    }
}
