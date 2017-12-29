// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom::{
    Document,
    Node,
    NodeType,
    ValueId,
};
use svgdom::types::{
    Length,
};

use short::{
    AId,
    AValue,
    Unit,
};

use traits::{
    FindAttribute,
};

use super::{
    DEFAULT_FONT_SIZE,
};


pub fn resolve_font_size(doc: &Document) {
    _resolve_font_size(&doc.root());
}

pub fn _resolve_font_size(parent: &Node) {
    for (_, mut node) in parent.children().svg() {
        // We have to resolve 'font-size' for all elements
        // and not only for 'text content' based,
        // because it will be used during 'em'/'ex' units conversion.
        //
        // https://www.w3.org/TR/2008/REC-CSS2-20080411/fonts.html#propdef-font-size

        let font_size = match node.attributes().get(AId::FontSize) {
            Some(v) => {
                v.value.clone()
            }
            None => {
                // If not set - lookup in parent nodes or use default.
                let len = node.find_attribute(AId::FontSize)
                              .unwrap_or(Length::new_number(DEFAULT_FONT_SIZE));

                AValue::Length(len)
            }
        };

        let font_size = match font_size {
            AValue::Length(len) => {
                if len.unit == Unit::Percent {
                    process_percent_font_size(parent, len)
                } else {
                    len
                }
            }
            AValue::PredefValue(id) => {
                process_named_font_size(id, &font_size)
            }
            _ => {
                // Technically unreachable, because 'svgparser' should validate it.
                warn!("Invalid 'font-size' value: {}.", font_size);
                Length::new(DEFAULT_FONT_SIZE, Unit::None)
            }
        };

        let had_attr = node.has_attribute(AId::FontSize);

        node.set_attribute((AId::FontSize, font_size));

        // We have to mark this attribute as invisible,
        // otherwise it will break the 'use' resolving.
        if !had_attr {
            if let Some(ref mut attr) = node.attributes_mut().get_mut(AId::FontSize) {
                attr.visible = false;
            }
        }

        if node.has_children() {
            _resolve_font_size(&mut node);
        }
    }
}

// If 'font-size' has percent units that it's value
// is relative to the parent node 'font-size'.
fn process_percent_font_size(parent: &Node, len: Length) -> Length {
    if parent.node_type() == NodeType::Root {
        Length::new(DEFAULT_FONT_SIZE, Unit::None)
    } else {
        let parent_len = parent.find_attribute(AId::FontSize)
                               .unwrap_or(Length::new_number(DEFAULT_FONT_SIZE));

        let n = len.num * parent_len.num * 0.01;
        Length::new(n, Unit::None)
    }
}

fn process_named_font_size(id: ValueId, font_size: &AValue) -> Length {
    let factor = match id {
        ValueId::XxSmall => -3,
        ValueId::XSmall => -2,
        ValueId::Small => -1,
        ValueId::Medium => 0,
        ValueId::Large => 1,
        ValueId::XLarge => 2,
        ValueId::XxLarge => 3,
        ValueId::Smaller => -1,
        ValueId::Larger => 1,
        _ => {
            // Technically unreachable, because 'svgparser' should validate it.
            warn!("Invalid 'font-size' value: {}.", font_size);
            0
        }
    };

    // 'On a computer screen a scaling factor of 1.2
    // is suggested between adjacent indexes'
    let n = DEFAULT_FONT_SIZE * 1.2f64.powi(factor);
    Length::new(n, Unit::None)
}
