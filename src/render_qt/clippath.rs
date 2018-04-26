// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use qt;
use usvg::tree;
use usvg::tree::prelude::*;

// self
use geom::*;
use traits::{
    ConvTransform,
    TransformFromBBox,
};
use super::{
    path,
    text,
    QtLayers,
};
use {
    Options,
};


pub fn apply(
    node: &tree::Node,
    cp: &tree::ClipPath,
    opt: &Options,
    bbox: Rect,
    layers: &mut QtLayers,
    p: &qt::Painter,
) {
    let clip_img = try_opt!(layers.get(), ());
    let mut clip_img = clip_img.borrow_mut();
    clip_img.fill(0, 0, 0, 255);

    let clip_p = qt::Painter::new(&clip_img);
    clip_p.set_transform(&p.get_transform());
    clip_p.apply_transform(&cp.transform.to_native());

    if cp.units == tree::Units::ObjectBoundingBox {
        clip_p.apply_transform(&qt::Transform::from_bbox(bbox));
    }

    clip_p.set_composition_mode(qt::CompositionMode::CompositionMode_Clear);

    let ts = clip_p.get_transform();
    for node in node.children() {
        clip_p.apply_transform(&node.transform().to_native());

        match *node.kind() {
            tree::NodeKind::Path(ref path_node) => {
                path::draw(&node.tree(), path_node, opt, &clip_p);
            }
            tree::NodeKind::Text(_) => {
                text::draw(&node, opt, &clip_p);
            }
            _ => {}
        }

        clip_p.set_transform(&ts);
    }

    clip_p.end();

    p.set_transform(&qt::Transform::default());
    p.set_composition_mode(qt::CompositionMode::CompositionMode_DestinationOut);
    p.draw_image(0.0, 0.0, &clip_img);

    layers.release();
}
