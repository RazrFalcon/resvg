// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use super::prelude::*;


/// Removes marker attributes from unsupported elements.
///
/// `marker-*` attributes can only be set on `path`, `line`, `polyline` and `polygon`.
///
/// Also, `marker-*` attributes cannot be set on shapes inside the `clipPath`.
pub fn rm_marker_attributes(doc: &Document) {
    for mut node in doc.root().descendants() {
        let is_valid_elem =
               node.is_tag_name(EId::Path)
            || node.is_tag_name(EId::Line)
            || node.is_tag_name(EId::Polyline)
            || node.is_tag_name(EId::Polygon);

        if !is_valid_elem {
            node.remove_attribute(AId::MarkerStart);
            node.remove_attribute(AId::MarkerMid);
            node.remove_attribute(AId::MarkerEnd);
        }
    }

    for node in doc.root().descendants().filter(|n| n.is_tag_name(EId::ClipPath)) {
        for mut child in node.descendants() {
            child.remove_attribute(AId::MarkerStart);
            child.remove_attribute(AId::MarkerMid);
            child.remove_attribute(AId::MarkerEnd);
        }
    }
}
