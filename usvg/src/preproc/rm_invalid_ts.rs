// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use super::prelude::*;


/// Transform with 0 scale makes an element invisible.
///
/// Also, `cairo` will crash if we pass such transform.
pub fn remove_invalid_transform(doc: &mut Document) {
    let root = doc.root();
    doc.drain(root, |n| is_invalid_transform(n));
}

fn is_invalid_transform(node: &Node) -> bool {
    if let Some(&AValue::Transform(ts)) = node.attributes().get_value(AId::Transform) {
        let (sx, sy) = ts.get_scale();
        if sx.fuzzy_eq(&0.0) || sy.fuzzy_eq(&0.0) {
            return true;
        }
    }

    false
}
