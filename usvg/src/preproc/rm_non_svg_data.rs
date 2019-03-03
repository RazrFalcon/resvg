// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use super::prelude::*;


/// Removes non-SVG elements and attributes.
pub fn remove_non_svg_data(doc: &mut Document) {
    // Keep only SVG elements and text nodes.
    let root = doc.root().clone();
    doc.drain(root, |n| !n.is_svg_element() && !n.is_text());
}

/// Removes descriptive elements.
///
/// Such elements do not impact rendering, so we can skip them.
/// But we should remove them before preprocessing and not during the conversion to the `Tree`.
///
/// The problem is that we are often checking if a node has any children by simply running the
/// `svgdom::Node::has_children` method. And it will return `true` for structures like this:
///
/// ```text
/// <g>
///     <desc>Text</desc>
/// </g>
/// ```
///
/// Which is not what we want. So by removing such elements,
/// we can simplify the preprocessing a bit.
///
/// <https://www.w3.org/TR/SVG11/intro.html#TermDescriptiveElement>
pub fn remove_descriptive_elements(doc: &mut Document) {
    let root = doc.root().clone();
    doc.drain(root, |n|    n.is_tag_name(EId::Title)
                        || n.is_tag_name(EId::Desc)
                        || n.is_tag_name(EId::Metadata));
}

/// Removes all text nodes that are not inside the `text` element.
pub fn remove_useless_text(doc: &mut Document) {
    let root = doc.root().clone();
    doc.drain(root, |n| {
        n.is_text() && !n.ancestors().any(|p| p.is_tag_name(EId::Text))
    });
}
