// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use qt;

// self
use tree;
use math::*;
use traits::{
    ConvTransform,
    TransformFromBBox,
};
use render_utils;


pub fn apply(
    rtree: &tree::RenderTree,
    global_ts: qt::Transform,
    bbox: Rect,
    pattern_node: tree::DefsNodeRef,
    pattern: &tree::Pattern,
    brush: &mut qt::Brush,
) {
    let r = if pattern.units == tree::Units::ObjectBoundingBox {
        pattern.rect.transform(tree::Transform::from_bbox(bbox))
    } else {
        pattern.rect
    };

    let global_ts = tree::Transform::from_native(&global_ts);
    let (sx, sy) = global_ts.get_scale();

    let img_size = Size::new(r.width() * sx, r.height() * sy);
    let img = qt::Image::new(img_size.width as u32, img_size.height as u32);
    let mut img = match img {
        Some(img) => img,
        None => {
            // TODO: add expected image size
            warn!("Subimage creation failed.");
            return;
        }
    };

    img.set_dpi(rtree.svg_node().dpi);
    img.fill(0, 0, 0, 0);

    let p = qt::Painter::new(&img);

    p.apply_transform(&qt::Transform::new(sx, 0.0, 0.0, sy, 0.0, 0.0));
    if let Some(vbox) = pattern.view_box {
        let img_view = Rect::from_xywh(0.0, 0.0, r.width(), r.height());
        let (dx, dy, sx2, sy2) = render_utils::view_box_transform(vbox, img_view);
        p.apply_transform(&qt::Transform::new(sx2, 0.0, 0.0, sy2, dx, dy));
    }
    if pattern.content_units == tree::Units::ObjectBoundingBox {
        p.apply_transform(&qt::Transform::from_bbox(bbox));
    }

    super::render_group(rtree, pattern_node.to_node_ref(), &p, &p.get_transform(), img_size);
    p.end();

    brush.set_pattern(img);

    let mut ts = tree::Transform::default();
    ts.append(&pattern.transform);
    ts.translate(r.x(), r.y());
    ts.scale(1.0 / sx, 1.0 / sy);
    brush.set_transform(ts.to_native());
}
