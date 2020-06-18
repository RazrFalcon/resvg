// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::render::prelude::*;

pub fn clip(
    node: &usvg::Node,
    cp: &usvg::ClipPath,
    opt: &Options,
    bbox: Rect,
    layers: &mut Layers,
    dt: &mut raqote::DrawTarget,
) {
    let clip_dt = layers.get();
    let mut clip_dt = clip_dt.borrow_mut();

    clip_dt.clear(raqote::SolidSource { r: 0, g: 0, b: 0, a: 255 });
    clip_dt.set_transform(dt.get_transform());
    clip_dt.transform(&cp.transform.to_native());

    if cp.units == usvg::Units::ObjectBoundingBox {
        clip_dt.transform(&usvg::Transform::from_bbox(bbox).to_native());
    }

    let ts = *clip_dt.get_transform();
    for node in node.children() {
        clip_dt.transform(&node.transform().to_native());

        match *node.borrow() {
            usvg::NodeKind::Path(ref p) => {
                let draw_opt = raqote::DrawOptions {
                    blend_mode: raqote::BlendMode::Clear,
                    ..Default::default()
                };

                crate::path::draw(&node.tree(), p, opt, draw_opt, &mut clip_dt);
            }
            usvg::NodeKind::Group(ref g) => {
                clip_group(&node, g, opt, bbox, layers, &mut clip_dt);
            }
            _ => {}
        }

        clip_dt.set_transform(&ts);
    }

    if let Some(ref id) = cp.clip_path {
        if let Some(ref clip_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::ClipPath(ref cp) = *clip_node.borrow() {
                clip(clip_node, cp, opt, bbox, layers, dt);
            }
        }
    }
    dt.blend_surface(&clip_dt,
        raqote::IntRect::new(raqote::IntPoint::new(0, 0),
                             raqote::IntPoint::new(clip_dt.width(), clip_dt.height())),
        raqote::IntPoint::new(0, 0),
        raqote::BlendMode::DstOut);
}

fn clip_group(
    node: &usvg::Node,
    g: &usvg::Group,
    opt: &Options,
    bbox: Rect,
    layers: &mut Layers,
    dt: &mut raqote::DrawTarget,
) {
    if let Some(ref id) = g.clip_path {
        if let Some(ref clip_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::ClipPath(ref cp) = *clip_node.borrow() {
                // If a `clipPath` child also has a `clip-path`
                // then we should render this child on a new canvas,
                // clip it, and only then draw it to the `clipPath`.

                let clip_dt = layers.get();
                let mut clip_dt = clip_dt.borrow_mut();
                clip_dt.set_transform(dt.get_transform());

                draw_group_child(&node, opt, raqote::DrawOptions::default(), &mut clip_dt);
                clip(clip_node, cp, opt, bbox, layers, &mut clip_dt);

                dt.set_transform(&raqote::Transform::identity());
                dt.draw_image_at(0.0, 0.0, &clip_dt.as_image(), &raqote::DrawOptions {
                    blend_mode: raqote::BlendMode::Xor,
                    ..Default::default()
                });
            }
        }
    }
}


fn draw_group_child(
    node: &usvg::Node,
    opt: &Options,
    draw_options: raqote::DrawOptions,
    dt: &mut raqote::DrawTarget,
) {
    if let Some(child) = node.first_child() {
        dt.transform(&child.transform().to_native());

        match *child.borrow() {
            usvg::NodeKind::Path(ref path_node) => {
                crate::path::draw(&child.tree(), path_node, opt, draw_options, dt);
            }
            _ => {}
        }
    }
}
