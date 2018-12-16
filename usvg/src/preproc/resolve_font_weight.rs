// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use super::prelude::*;


pub fn resolve_font_weight(doc: &Document) {
    for (_, mut node) in doc.root().descendants().svg() {
        let parent = match node.parent() {
            Some(p) => p,
            None => continue,
        };

        let av = node.attributes().get_value(AId::FontWeight).cloned();
        if let Some(AValue::String(name)) = av {
            match name.as_str() {
                "bolder" => {
                    // By the CSS2 spec the default value should be 400
                    // so `bolder` will result in 500.
                    // But Chrome and Inkscape will give us 700.
                    // Have no idea is it a bug or something, but
                    // we will follow such behavior for now.
                    let weight = find_font_weight(&parent, 600);
                    let weight = bound(100, weight + 100, 900);
                    node.set_attribute((AId::FontWeight, weight.to_string()));
                }
                "lighter" => {
                    // By the CSS2 spec the default value should be 400
                    // so `lighter` will result in 300.
                    // But Chrome and Inkscape will give us 200.
                    // Have no idea is it a bug or something, but
                    // we will follow such behavior for now.
                    let weight = find_font_weight(&parent, 300);
                    let weight = bound(100, weight - 100, 900);
                    node.set_attribute((AId::FontWeight, weight.to_string()));
                }
                _ => {}
            }
        }
    }
}

fn bound<T: ::std::cmp::Ord>(min: T, val: T, max: T) -> T {
    use std::cmp;

    cmp::max(min, cmp::min(max, val))
}

fn find_font_weight(node: &Node, default: i32) -> i32 {
    for n in node.ancestors() {
        if let Some(v) = n.attributes().get_str(AId::FontWeight) {
            return v.parse().unwrap_or(default);
        }
    }

    default
}
