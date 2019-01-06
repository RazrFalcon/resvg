// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use super::prelude::*;


/// Removes invalid IRI links.
///
/// `svgdom` will store unresolved links as strings.
/// So we have to remove attributes that should be FuncIRI and not string.
pub fn fix_xlinks(doc: &Document) {
    // Remove all `xlink:href` that is not a `Link` type.
    // Except `image` element.
    let iter = doc.root().descendants()
                  .filter(|n| !n.is_tag_name(EId::Image) && !n.is_tag_name(EId::FeImage));
    for mut node in iter {
        let av = node.attributes().get_value(AId::Href).cloned();
        if let Some(av) = av {
            match av {
                AValue::Link(_) => {}
                _ => {
                    node.remove_attribute(AId::Href);
                }
            }
        }
    }


    // Check that `xlink:href` reference a proper element type.
    for (eid, mut node) in doc.root().descendants().svg() {
        let av = node.attributes().get_value(AId::Href).cloned();
        if let Some(AValue::Link(link)) = av {
            let is_valid = match eid {
                EId::LinearGradient | EId::RadialGradient => link.is_gradient(),
                EId::Pattern => link.is_tag_name(EId::Pattern),
                EId::Filter => link.is_tag_name(EId::Filter),
                _ => true,
            };

            if !is_valid {
                node.remove_attribute(AId::Href);
            }
        }
    }
}

// Removes all `xlink:href` attributes because we already resolved everything.
pub fn remove_xlinks(doc: &Document) {
    let iter = doc.root().descendants()
                  .filter(|n| !n.is_tag_name(EId::Image) && !n.is_tag_name(EId::FeImage))
                  .filter(|n| n.has_attribute(AId::Href));

    for mut node in iter {
        node.remove_attribute(AId::Href);
    }
}

/// Removes invalid FuncIRI links.
///
/// `svgdom` will store unresolved links as strings.
/// So we have to remove attributes that should be FuncIRI and not string.
pub fn fix_links(doc: &mut Document) {
    for mut node in doc.root().descendants() {
        if !is_valid_func_link(&node, AId::ClipPath) {
            node.remove_attribute(AId::ClipPath);
        }

        if !is_valid_func_link(&node, AId::Mask) {
            node.remove_attribute(AId::Mask);
        }
    }

    // Unlike `clip-path` and `mask`, when `filter` is invalid
    // than the whole element should be removed.
    let root = doc.root().clone();
    doc.drain(root, |n| !is_valid_func_link(n, AId::Filter));
}

fn is_valid_func_link(node: &Node, aid: AId) -> bool {
    match node.attributes().get_value(aid) {
        Some(AValue::FuncLink(_)) | Some(AValue::None) | None => true,
        _ => false,
    }
}
