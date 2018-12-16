// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use super::prelude::*;


/// Removes well-defined, but invisible, elements.
pub fn remove_invisible_elements(doc: &mut Document) {
    // TODO: process Options::keep_invisible_shapes
    rm_display_none(doc);

    // TODO: to a separate module.
    // Since 'svgdom' automatically removes (Func)IRI attributes
    // from linked elements, 'use' elements may became obsolete, because
    // a 'use' element without 'xlink:href' is invalid.
    rm_use(doc);
}

fn rm_display_none(doc: &mut Document) {
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

fn rm_use(doc: &mut Document) {
    fn _rm(doc: &mut Document) -> usize {
        let root = doc.root();
        doc.drain(root, |n| {
            if n.is_tag_name(EId::Use) {
                if !n.has_attribute(AId::Href) {
                    // remove 'use' element without the 'xlink:href' attribute
                    return true;
                } else {
                    // remove 'use' element with invalid 'xlink:href' attribute value
                    let attrs = n.attributes();
                    if let Some(&AValue::Link(_)) = attrs.get_value(AId::Href) {
                        // nothing
                    } else {
                        // NOTE: actually, an attribute with 'String' type is valid
                        // if it contain a path to an external file, like '../img.svg#rect1',
                        // but we don't support external SVG, so we treat it like an invalid
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
