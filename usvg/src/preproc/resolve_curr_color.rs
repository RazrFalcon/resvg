// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom::{
    Color,
    PaintFallback,
};

use super::prelude::*;


/// Resolves the `currentColor` attribute.
///
/// The function will fallback to a default value when possible.
pub fn resolve_current_color(doc: &Document) {
    let mut ids = Vec::new();

    for (_, mut node) in doc.root().descendants().svg() {
        ids.clear();

        {
            let attrs = node.attributes();
            for (aid, attr) in attrs.iter().svg() {
                match attr.value {
                    AValue::CurrentColor => {
                        ids.push(aid);
                    }
                    AValue::Paint(_, fallback) => {
                        if let Some(PaintFallback::CurrentColor) = fallback {
                            ids.push(aid);
                        }
                    }
                    _ => {}
                }
            }
        }

        for id in &ids {
            match resolve_color(&node, *id) {
                Some(v) => {
                    let av = node.attributes().get_value(*id).cloned().unwrap();
                    match av {
                        AValue::CurrentColor => {
                            node.set_attribute((*id, v));
                        }
                        AValue::Paint(link, _) => {
                            let fallback = Some(PaintFallback::Color(v));
                            node.set_attribute((*id, (link.clone(), fallback)));
                        }
                        _ => {}
                    }
                }
                None => {
                    warn!("Failed to resolve currentColor for '{}'. Removing it.", id);
                    node.remove_attribute(*id);
                }
            }
        }
    }
}

fn resolve_color(node: &Node, aid: AId) -> Option<Color> {
    if let Some(n) = node.ancestors().find(|n| n.has_attribute(AId::Color)) {
        n.attributes().get_color(AId::Color)
    } else {
        match aid {
              AId::Fill
            | AId::FloodColor
            | AId::StopColor => Some(Color::black()),
            AId::LightingColor => Some(Color::white()),
            _ => None,
        }
    }
}
