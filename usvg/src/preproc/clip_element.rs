// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use super::prelude::*;

// Emulate a new viewport via clipPath.
//
// From:
// <defs/>
// <elem/>
//
// To:
// <defs>
//   <clipPath id="clipPath1">
//     <rect/>
//   </clipPath>
// </defs>
// <g clip-path="ulr(#clipPath1)">
//   <elem/>
// </g>
pub fn clip_element(doc: &mut Document, target_node: &mut Node) -> Option<Node> {
    let mut defs_node = try_opt!(doc.defs_element(), None);

    // No need to clip elements with overflow:visible.
    {
        let attrs = target_node.attributes();
        let overflow = attrs.get_str_or(AId::Overflow, "hidden");
        if overflow != "hidden" && overflow != "scroll" {
            return None;
        }
    }

    if let Some(clip_rect) = get_clip_rect(doc, target_node) {
        // We can't set `clip-path` on the element itself,
        // because it will be affected by a possible transform.
        // So we have to create an additional group.
        let mut g_node = doc.create_element(EId::G);
        target_node.insert_before(g_node.clone());
        target_node.detach();
        g_node.append(target_node.clone());

        let mut clip_node = doc.create_element(EId::ClipPath);
        clip_node.set_id(gen_clip_path_id(doc));
        clip_node.set_attribute((AId::ClipPathUnits, "userSpaceOnUse"));
        defs_node.append(clip_node.clone());

        let mut rect_node = doc.create_element(EId::Rect);

        rect_node.set_attribute((AId::X, clip_rect.x));
        rect_node.set_attribute((AId::Y, clip_rect.y));
        rect_node.set_attribute((AId::Width, clip_rect.width));
        rect_node.set_attribute((AId::Height, clip_rect.height));
        clip_node.append(rect_node);

        g_node.set_attribute((AId::ClipPath, clip_node.clone()));

        Some(g_node)
    } else {
        None
    }
}

fn get_clip_rect(doc: &Document, node: &Node) -> Option<Rect> {
    let (x, y, w, h) = {
        let attrs = node.attributes();
        let x = attrs.get_number(AId::X)?;
        let y = attrs.get_number(AId::Y)?;
        let w = attrs.get_number(AId::Width)?;
        let h = attrs.get_number(AId::Height)?;
        (x, y, w, h)
    };

    let svg = doc.svg_element()?;
    let svg_w = svg.attributes().get_number(AId::Width)?;
    let svg_h = svg.attributes().get_number(AId::Height)?;

    // Clip rect is not needed when it has the same size as a whole image.
    if w.fuzzy_eq(&svg_w) && h.fuzzy_eq(&svg_h) {
        return None;
    }

    Some((x, y, w, h).into())
}

/// Creates a free id for `clipPath`.
fn gen_clip_path_id(doc: &Document) -> String {
    // TODO: speedup

    let mut idx = 1;
    let mut id = format!("clipPath{}", idx);
    while doc.root().descendants().any(|n| *n.id() == id) {
        idx += 1;
        id = format!("clipPath{}", idx);
    }

    id
}
