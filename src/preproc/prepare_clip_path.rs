// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom::{
    Document,
    ValueId,
};

use svgdom::types::{
    Color,
};

use short::{
    AId,
    EId,
};

use traits::{
    GetValue,
};


// The clipPath is implemented using a 1bit-like mask.
// So to create it we have to draw all clipPath children with a black fill and without a stroke.
pub fn prepare_clip_path(doc: &mut Document) {
    for mut node in doc.descendants().filter(|n| n.is_tag_name(EId::ClipPath)) {
        let units = node.attributes().get_predef(AId::ClipPathUnits)
                        .unwrap_or(ValueId::UserSpaceOnUse);
        node.set_attribute((AId::ClipPathUnits, units));

        for (_, mut child) in node.descendants().svg() {
            // Set fill to black.
            child.set_attribute((AId::Fill, Color::new(0, 0, 0)));

            // Remove stroke.
            child.set_attribute((AId::Stroke, ValueId::None));

            // Disable opacity.
            child.set_attribute((AId::Opacity, 1.0));

            // We don't have a separate fill-rule for clipPath, so use an existing property.
            //
            // Note that in the SVG dump it will be converted back to clip-path.
            let clip_rule = child.attributes().get_predef(AId::ClipRule).unwrap_or(ValueId::Nonzero);
            child.set_attribute((AId::FillRule, clip_rule));
        }
    }
}
