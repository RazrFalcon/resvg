// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use crate::qt;

// self
use super::prelude::*;
use super::path;


pub fn apply(
    node: &usvg::Node,
    cp: &usvg::ClipPath,
    opt: &Options,
    bbox: Rect,
    layers: &mut QtLayers,
    p: &mut qt::Painter,
) {
    let clip_img = try_opt!(layers.get(), ());
    let mut clip_img = clip_img.borrow_mut();
    clip_img.fill(0, 0, 0, 255);

    let mut clip_p = qt::Painter::new(&mut clip_img);
    clip_p.set_transform(&p.get_transform());
    clip_p.apply_transform(&cp.transform.to_native());

    if cp.units == usvg::Units::ObjectBoundingBox {
        clip_p.apply_transform(&qt::Transform::from_bbox(bbox));
    }

    clip_p.set_composition_mode(qt::CompositionMode::Clear);

    let ts = clip_p.get_transform();
    for node in node.children() {
        clip_p.apply_transform(&node.transform().to_native());

        match *node.borrow() {
            usvg::NodeKind::Path(ref path_node) => {
                path::draw(&node.tree(), path_node, opt, &mut clip_p);
            }
            usvg::NodeKind::Group(ref g) => {
                clip_group(&node, g, opt, bbox, layers, &mut clip_p);
            }
            _ => {}
        }

        clip_p.set_transform(&ts);
    }

    clip_p.end();

    if let Some(ref id) = cp.clip_path {
        if let Some(ref clip_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::ClipPath(ref cp) = *clip_node.borrow() {
                apply(clip_node, cp, opt, bbox, layers, p);
            }
        }
    }

    p.set_transform(&qt::Transform::default());
    p.set_composition_mode(qt::CompositionMode::DestinationOut);
    p.draw_image(0.0, 0.0, &clip_img);
}

fn clip_group(
    node: &usvg::Node,
    g: &usvg::Group,
    opt: &Options,
    bbox: Rect,
    layers: &mut QtLayers,
    p: &mut qt::Painter,
) {
    if let Some(ref id) = g.clip_path {
        if let Some(ref clip_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::ClipPath(ref cp) = *clip_node.borrow() {
                // If a `clipPath` child also has a `clip-path`
                // then we should render this child on a new canvas,
                // clip it, and only then draw it to the `clipPath`.

                let clip_img = try_opt!(layers.get(), ());
                let mut clip_img = clip_img.borrow_mut();

                let mut clip_p = qt::Painter::new(&mut clip_img);
                clip_p.set_transform(&p.get_transform());
                draw_group_child(&node, opt, &mut clip_p);

                apply(clip_node, cp, opt, bbox, layers, &mut clip_p);
                clip_p.end();

                p.set_transform(&qt::Transform::default());
                p.set_composition_mode(qt::CompositionMode::Xor);
                p.draw_image(0.0, 0.0, &clip_img);
            }
        }
    }
}

fn draw_group_child(
    node: &usvg::Node,
    opt: &Options,
    p: &mut qt::Painter,
) {
    if let Some(child) = node.first_child() {
        p.apply_transform(&child.transform().to_native());

        match *child.borrow() {
            usvg::NodeKind::Path(ref path_node) => {
                path::draw(&child.tree(), path_node, opt, p);
            }
            _ => {}
        }
    }
}
