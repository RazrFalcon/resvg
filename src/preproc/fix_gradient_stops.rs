// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::f64;

// external
use svgdom::{
    Document,
    ElementType,
    FuzzyEq,
    Node,
};

// self
use math;
use short::{
    AId,
    EId,
};
use traits::{
    GetValue,
};


pub fn fix_gradient_stops(doc: &Document) {
    for node in doc.descendants().filter(|n| n.is_gradient()) {
        let mut prev_offset = 0.0;
        let mut prev_stop: Option<Node> = None;
        for mut stop in node.children().filter(|n| n.is_tag_name(EId::Stop)) {
            let mut offset = stop.attributes().get_number(AId::Offset).unwrap_or(0.0);
            offset = math::f64_bound(0.0, offset, 1.0);

            // Next offset must be smaller then previous.
            if offset < prev_offset || offset.fuzzy_eq(&prev_offset) {
                if let Some(mut prev_stop) = prev_stop {
                    // Make previous offset a bit smaller.
                    let new_offset = prev_offset - f64::EPSILON;
                    prev_stop.set_attribute((AId::Offset, new_offset));
                }

                offset = prev_offset;
            }

            stop.set_attribute((AId::Offset, offset));
            prev_offset = offset;
            prev_stop = Some(stop);
        }
    }
}
