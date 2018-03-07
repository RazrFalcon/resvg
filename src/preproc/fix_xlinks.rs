// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom::{
    Document,
};

// self
use short::{
    AId,
    AValue,
    EId,
};


// Remove all `xlink:href` that is not `Link` type.
// Except `image` element.
pub fn fix_xlinks(doc: &Document) {
    for mut node in doc.descendants().filter(|n| !n.is_tag_name(EId::Image)) {
        let av = node.attributes().get_value(("xlink", AId::Href)).cloned();
        if let Some(av) = av {
            match av {
                AValue::Link(_) => {}
                _ => {
                    node.remove_attribute(("xlink", AId::Href));
                }
            }
        }
    }
}
