// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom::Attributes;

use super::prelude::*;


// Full list can be found here: https://www.w3.org/TR/SVG11/feature.html

static FEATURES: &[&str] = &[
    "http://www.w3.org/TR/SVG11/feature#SVGDOM-static",
    "http://www.w3.org/TR/SVG11/feature#SVG-static",
    "http://www.w3.org/TR/SVG11/feature#CoreAttribute", // no xml:base and xml:lang
    "http://www.w3.org/TR/SVG11/feature#Structure",
    "http://www.w3.org/TR/SVG11/feature#BasicStructure",
    // "http://www.w3.org/TR/SVG11/feature#ContainerAttribute", // `enable-background`, not yet
    "http://www.w3.org/TR/SVG11/feature#ConditionalProcessing",
    "http://www.w3.org/TR/SVG11/feature#Image",
    "http://www.w3.org/TR/SVG11/feature#Style",
    // "http://www.w3.org/TR/SVG11/feature#ViewportAttribute", // `clip` and `overflow`, not yet
    "http://www.w3.org/TR/SVG11/feature#Shape",
    "http://www.w3.org/TR/SVG11/feature#Text", // partial
    "http://www.w3.org/TR/SVG11/feature#BasicText",
    "http://www.w3.org/TR/SVG11/feature#PaintAttribute", // no color-interpolation and color-rendering
    "http://www.w3.org/TR/SVG11/feature#BasicPaintAttribute", // no color-interpolation
    "http://www.w3.org/TR/SVG11/feature#OpacityAttribute",
    // "http://www.w3.org/TR/SVG11/feature#GraphicsAttribute",
    "http://www.w3.org/TR/SVG11/feature#BasicGraphicsAttribute",
    // "http://www.w3.org/TR/SVG11/feature#Marker", // not yet
    // "http://www.w3.org/TR/SVG11/feature#ColorProfile", // not yet
    "http://www.w3.org/TR/SVG11/feature#Gradient",
    "http://www.w3.org/TR/SVG11/feature#Pattern",
    "http://www.w3.org/TR/SVG11/feature#Clip",
    "http://www.w3.org/TR/SVG11/feature#BasicClip",
    "http://www.w3.org/TR/SVG11/feature#Mask",
    // "http://www.w3.org/TR/SVG11/feature#Filter", // not yet
    "http://www.w3.org/TR/SVG11/feature#BasicFilter",
    "http://www.w3.org/TR/SVG11/feature#XlinkAttribute", // only xlink:href
    // "http://www.w3.org/TR/SVG11/feature#Font",
    // "http://www.w3.org/TR/SVG11/feature#BasicFont",
];

pub fn resolve_conditional(doc: &mut Document, opt: &Options) {
    resolve_conditional_attrs(doc, opt);
    resolve_switch(doc, opt);
}

fn resolve_conditional_attrs(doc: &mut Document, opt: &Options) {
    let root = doc.root();
    doc.drain(root, |node| {
        // Process `switch` separately.
        if node.is_tag_name(EId::Switch) {
            return false;
        }
        if node.parent().unwrap().is_tag_name(EId::Switch) {
            return false;
        }

        // Technically, conditional attributes can be set on any element,
        // but they can affect only one that will be rendered.
        // Just like `display`.
        let flag =    node.is_graphic()
                   || node.is_text_content_child()
                   || node.is_tag_name(EId::Svg)
                   || node.is_tag_name(EId::G)
                   || node.is_tag_name(EId::Switch)
                   || node.is_tag_name(EId::A)
                   || node.is_tag_name(EId::ForeignObject);

        if !flag {
            return false;
        }

        !is_valid_child(node, opt)
    });
}

fn resolve_switch(doc: &mut Document, opt: &Options) {
    let mut rm_nodes = Vec::with_capacity(16);

    for mut node in doc.root().descendants().filter(|n| n.is_tag_name(EId::Switch)) {
        let mut valid_child = None;

        // Find first valid node.
        for (_, child) in node.children().svg() {
            if is_valid_child(&child, opt) {
                valid_child = Some(child.clone());
                break;
            }
        }

        let valid_child = match valid_child {
            Some(v) => v,
            None => continue,
        };

        // Remove all invalid nodes.
        for child in node.children().filter(|n| *n != valid_child) {
            rm_nodes.push(child.clone());
        }
        rm_nodes.iter_mut().for_each(|n| doc.remove_node(n.clone()));
        rm_nodes.clear();

        // 'switch' -> 'g'
        node.set_tag_name(EId::G);

        // Remember that this group was 'switch' before.
        node.set_attribute(("usvg-group", 1));
    }
}

fn is_valid_child(node: &Node, opt: &Options) -> bool {
    let ref attrs = node.attributes();

    if attrs.contains(AId::RequiredExtensions) {
        return false;
    }

    // 'The value is a list of feature strings, with the individual values separated by white space.
    // Determines whether all of the named features are supported by the user agent.
    // Only feature strings defined in the Feature String appendix are allowed.
    // If all of the given features are supported, then the attribute evaluates to true;
    // otherwise, the current element and its children are skipped and thus will not be rendered.'
    if let Some(features) = attrs.get_str(AId::RequiredFeatures) {
        for feature in features.split(' ') {
            if !FEATURES.contains(&feature) {
                return false;
            }
        }
    }

    if !is_valid_sys_lang(attrs, opt) {
        return false;
    }

    true
}

/// SVG spec 5.8.5
fn is_valid_sys_lang(attrs: &Attributes, opt: &Options) -> bool {
    // 'The attribute value is a comma-separated list of language names
    // as defined in BCP 47.'
    //
    // But we support only simple cases like `en` or `en-US`.
    // No one really uses this, especially with complex BCP 47 values.
    if let Some(langs) = attrs.get_str(AId::SystemLanguage) {
        let mut has_match = false;
        for lang in langs.split(',') {
            let lang = lang.trim();

            // 'Evaluates to `true` if one of the languages indicated by user preferences exactly
            // equals one of the languages given in the value of this parameter.'
            if opt.languages.iter().any(|v| v == lang) {
                has_match = true;
                break;
            }

            // 'If one of the languages indicated by user preferences exactly equals a prefix
            // of one of the languages given in the value of this parameter such that
            // the first tag character following the prefix is `-`.'
            if let Some(idx) = lang.bytes().position(|c| c == b'-') {
                let lang_prefix = &lang[..idx];
                if opt.languages.iter().any(|v| v == lang_prefix) {
                    has_match = true;
                    break;
                }
            }
        }

        return has_match;
    }

    true
}
