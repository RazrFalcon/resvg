// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use super::prelude::*;

pub fn resolve_display(doc: &mut Document) {
    let root = doc.root();
    doc.drain(root, |n| {
        if let Some(&AValue::None) = n.attributes().get_value(AId::Display) {
            let flag =    n.is_graphic()
                       || n.is_text_content_child()
                       || n.is_tag_name(EId::Svg)
                       || n.is_tag_name(EId::G)
                       || n.is_tag_name(EId::Switch)
                       || n.is_tag_name(EId::A)
                       || n.is_tag_name(EId::ForeignObject);

            return flag;
        }

        false
    });
}
