// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom::{
    Document,
    FuzzyEq,
    Node,
};

// self
use short::{
    AId,
    AValue,
};


/// Removes ill-defined elements.
pub fn remove_invalid_transform(doc: &mut Document) {
    doc.drain(|n| is_invalid_transform(n));
}

/// Transform with 0 scale makes element invisible.
///
/// Also, `cairo` will crash if we pass such transform.
fn is_invalid_transform(node: &Node) -> bool {
    if let Some(&AValue::Transform(ts)) = node.attributes().get_value(AId::Transform) {
        let (sx, sy) = ts.get_scale();

        if sx.fuzzy_eq(&0.0) || sy.fuzzy_eq(&0.0) {
            return true;
        }
    }

    false
}
