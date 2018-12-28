// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::f64;

use super::prelude::*;


pub fn fix_gradient_stops(doc: &mut Document) {
    let mut stops = Vec::new();
    for gradient in doc.root().descendants().filter(|n| n.is_gradient()) {
        // Remove any non-`stop` children, so we can skip tag name checks in the code below.
        {
            let mut stop_opt = gradient.first_child();
            while let Some(stop) = stop_opt {
                stop_opt = stop.next_sibling();

                if !stop.is_tag_name(EId::Stop) {
                    doc.remove_node(stop);
                }
            }
        }

        stops.clear();
        _fix_gradient_stops(&gradient, &mut stops, doc);
    }
}

fn _fix_gradient_stops(grad: &Node, stops: &mut Vec<Node>, doc: &mut Document) {
    // Resolve missing offsets.
    {
        let mut prev_offset = 0.0;
        for mut stop in grad.children() {
            let offset = stop.attributes().get_number(AId::Offset);
            match offset {
                Some(n) => prev_offset = n,
                None => stop.set_attribute((AId::Offset, prev_offset)),
            }
        }
    }

    // Clamp offsets.
    for mut stop in grad.children() {
        let mut attrs = stop.attributes_mut();
        let av = attrs.get_value_mut(AId::Offset);
        if let Some(&mut AValue::Number(ref mut offset)) = av {
            *offset = f64_bound(0.0, *offset, 1.0);
        }
    }

    // Remove stops with equal offset.
    //
    // Example:
    // offset="0.5"
    // offset="0.7"
    // offset="0.7" <-- this one should be removed
    // offset="0.7"
    // offset="0.9"
    {
        stops.clear();
        for stop in grad.children() {
            stops.push(stop);
        }

        if stops.len() >= 3 {
            let mut i = 0;
            while i < stops.len() - 2 {
                let offset1 = stops[i + 0].attributes().get_number_or(AId::Offset, 0.0);
                let offset2 = stops[i + 1].attributes().get_number_or(AId::Offset, 0.0);
                let offset3 = stops[i + 2].attributes().get_number_or(AId::Offset, 0.0);

                if offset1.fuzzy_eq(&offset2) && offset2.fuzzy_eq(&offset3) {
                    // Remove offset in the middle.
                    doc.remove_node(stops[i + 1].clone());
                    stops.remove(1);
                } else {
                    i += 1;
                }
            }
        }
    }

    // Remove zeros.
    //
    // From:
    // offset="0.0"
    // offset="0.0"
    // offset="0.7"
    //
    // To:
    // offset="0.0"
    // offset="0.00000001"
    // offset="0.7"
    {
        stops.clear();
        for stop in grad.children() {
            stops.push(stop);
        }

        while stops.len() >= 2 {
            let offset1 = stops[0].attributes().get_number_or(AId::Offset, 0.0);
            let offset2 = stops[1].attributes().get_number_or(AId::Offset, 0.0);

            if offset1.is_fuzzy_zero() && offset2.is_fuzzy_zero() {
                stops[1].set_attribute((AId::Offset, offset1 + f64::EPSILON));
            } else {
                break;
            }
        }
    }

    // Shift equal offsets.
    //
    // From:
    // offset="0.5"
    // offset="0.7"
    // offset="0.7"
    //
    // To:
    // offset="0.5"
    // offset="0.699999999"
    // offset="0.7"
    {
        let mut prev_offset = 0.0;
        for mut stop in grad.children() {
            let mut offset = stop.attributes().get_number_or(AId::Offset, 0.0);

            // Next offset must be smaller then previous.
            if offset < prev_offset || offset.fuzzy_eq(&prev_offset) {
                if let Some(mut prev_stop) = stop.previous_sibling() {
                    // Make previous offset a bit smaller.
                    let new_offset = prev_offset - f64::EPSILON;
                    prev_stop.set_attribute((AId::Offset, new_offset));
                }

                offset = prev_offset;
            }

            stop.set_attribute((AId::Offset, offset));
            prev_offset = offset;
        }
    }
}
