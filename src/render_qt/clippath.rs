// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use qt;

use tree;

use math::{
    Size,
    Rect,
};

use traits::{
    ConvTransform,
};

use super::{
    path,
    text,
};


pub fn apply(
    doc: &tree::RenderTree,
    node: tree::DefsNodeRef,
    cp: &tree::ClipPath,
    p: &qt::Painter,
    bbox: &Rect,
    img_size: Size,
) {
    let mut clip_img = qt::Image::new(
        img_size.w as u32,
        img_size.h as u32,
    ).unwrap(); // TODO: remove

    clip_img.fill(0, 0, 0, 255);
    clip_img.set_dpi(doc.svg_node().dpi);

    let clip_p = qt::Painter::new(&clip_img);
    clip_p.set_transform(&p.get_transform());
    clip_p.apply_transform(&cp.transform.to_native());

    if cp.units == tree::Units::ObjectBoundingBox {
        clip_p.apply_transform(&qt::Transform::new(bbox.w, 0.0, 0.0, bbox.h, bbox.x, bbox.y));
    }

    clip_p.set_composition_mode(qt::CompositionMode::CompositionMode_Clear);

    let ts = clip_p.get_transform();
    for node in node.children() {
        clip_p.apply_transform(&node.kind().transform().to_native());

        match node.kind() {
            tree::NodeKindRef::Path(ref path_elem) => {
                path::draw(doc, path_elem, &clip_p);
            }
            tree::NodeKindRef::Text(_) => {
                text::draw(doc, node, &clip_p);
            }
            _ => {}
        }

        clip_p.set_transform(&ts);
    }

    clip_p.end();

    p.set_transform(&qt::Transform::default());
    p.set_composition_mode(qt::CompositionMode::CompositionMode_DestinationOut);
    p.draw_image(0.0, 0.0, &clip_img);
}
