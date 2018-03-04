// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom::{
    Document,
    ValueId,
};

// self
use short::{
    AId,
    AValue,
};
use traits::{
    FindAttribute,
};


pub fn resolve_font_weight(doc: &Document) {
    for (_, mut node) in doc.descendants().svg() {
        let parent = match node.parent() {
            Some(p) => p,
            None => continue,
        };

        let av = node.attributes().get_value(AId::FontWeight).cloned();
        if let Some(AValue::PredefValue(id)) = av {
            match id {
                ValueId::Bolder => {
                    // By the CSS2 spec the default value should be 400
                    // so `bolder` will result in 500.
                    // But Chrome and Inkscape will give us 700.
                    // Have no idea is it a bug or something, but
                    // we will follow such behavior for now.
                    let parent_w = parent.find_attribute(AId::FontWeight)
                                         .unwrap_or(ValueId::N600);

                    let weight = match parent_w {
                        ValueId::N100 => ValueId::N200,
                        ValueId::N200 => ValueId::N300,
                        ValueId::N300 => ValueId::N400,
                        ValueId::N400 => ValueId::N500,
                        ValueId::N500 => ValueId::N600,
                        ValueId::N600 => ValueId::N700,
                        ValueId::N700 => ValueId::N800,
                        ValueId::N800 => ValueId::N900,
                        ValueId::N900 => ValueId::N900,
                        _ => ValueId::N700,
                    };

                    node.set_attribute((AId::FontWeight, weight));
                }
                ValueId::Lighter => {
                    // By the CSS2 spec the default value should be 400
                    // so `lighter` will result in 300.
                    // But Chrome and Inkscape will give us 200.
                    // Have no idea is it a bug or something, but
                    // we will follow such behavior for now.
                    let parent_w = parent.find_attribute(AId::FontWeight)
                                         .unwrap_or(ValueId::N300);

                    let weight = match parent_w {
                        ValueId::N100 => ValueId::N100,
                        ValueId::N200 => ValueId::N100,
                        ValueId::N300 => ValueId::N200,
                        ValueId::N400 => ValueId::N300,
                        ValueId::N500 => ValueId::N400,
                        ValueId::N600 => ValueId::N500,
                        ValueId::N700 => ValueId::N600,
                        ValueId::N800 => ValueId::N700,
                        ValueId::N900 => ValueId::N800,
                        _ => ValueId::N400,
                    };

                    node.set_attribute((AId::FontWeight, weight));
                }
                _ => {}
            }
        }
    }
}
