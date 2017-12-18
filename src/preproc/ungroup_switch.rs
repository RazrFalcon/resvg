// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom::{
    Document,
    Node,
};

use short::{
    AId,
    EId,
    AValue,
};


static FEATURES: &[&str] = &[
    // "http://www.w3.org/TR/SVG11/feature#SVG", // not yet
    // "http://www.w3.org/TR/SVG11/feature#SVG-static", // not yet
    "http://www.w3.org/TR/SVG11/feature#CoreAttribute", // no xml:base and xml:lang
    "http://www.w3.org/TR/SVG11/feature#Structure",
    "http://www.w3.org/TR/SVG11/feature#BasicStructure",
    // "http://www.w3.org/TR/SVG11/feature#ContainerAttribute", // not yet
    "http://www.w3.org/TR/SVG11/feature#ConditionalProcessing", // no systemLanguage
    "http://www.w3.org/TR/SVG11/feature#Image",
    "http://www.w3.org/TR/SVG11/feature#Style",
    // "http://www.w3.org/TR/SVG11/feature#ViewportAttribute", // not yet
    "http://www.w3.org/TR/SVG11/feature#Shape",
    "http://www.w3.org/TR/SVG11/feature#Text", // partial
    "http://www.w3.org/TR/SVG11/feature#BasicText",
    "http://www.w3.org/TR/SVG11/feature#PaintAttribute", // no color-interpolation and color-rendering
    "http://www.w3.org/TR/SVG11/feature#BasicPaintAttribute", // no color-interpolation
    "http://www.w3.org/TR/SVG11/feature#OpacityAttribute",
    // "http://www.w3.org/TR/SVG11/feature#GraphicsAttribute", // not yet
    "http://www.w3.org/TR/SVG11/feature#BasicGraphicsAttribute",
    // "http://www.w3.org/TR/SVG11/feature#Marker", // not yet
    // "http://www.w3.org/TR/SVG11/feature#ColorProfile", // not yet
    "http://www.w3.org/TR/SVG11/feature#Gradient",
    // "http://www.w3.org/TR/SVG11/feature#Pattern", // not yet
    // "http://www.w3.org/TR/SVG11/feature#Clip", // not yet
    // "http://www.w3.org/TR/SVG11/feature#Mask", // not yet
    // "http://www.w3.org/TR/SVG11/feature#Filter", // not yet
    // "http://www.w3.org/TR/SVG11/feature#BasicFilter", // not yet
    "http://www.w3.org/TR/SVG11/feature#Hyperlinking", // kinda
    "http://www.w3.org/TR/SVG11/feature#XlinkAttribute", // only xlink:href
];

pub fn ungroup_switch(doc: &Document) {
    loop {
        if let Some(mut node) = doc.descendants().find(|n| n.is_tag_name(EId::Switch)) {
            for (_, mut child) in node.children().svg() {
                if is_valid_child(&child) {
                    child.detach();
                    node.insert_after(&child);
                    node.remove();

                    break;
                }
            }
        } else {
            break;
        }
    }
}

fn is_valid_child(node: &Node) -> bool {
    let attrs = node.attributes();

    if attrs.contains(AId::RequiredExtensions) {
        return false;
    }

    // TODO: systemLanguage

    // 'The value is a list of feature strings, with the individual values separated by white space.
    // Determines whether all of the named features are supported by the user agent.
    // Only feature strings defined in the Feature String appendix are allowed.
    // If all of the given features are supported, then the attribute evaluates to true;
    // otherwise, the current element and its children are skipped and thus will not be rendered.'
    if let Some(features) = attrs.get_value(AId::RequiredFeatures) {
        if let &AValue::String(ref features) = features {
            for feature in features.split(' ') {
                if !FEATURES.contains(&feature) {
                    return false;
                }
            }
        }
    }

    true
}
