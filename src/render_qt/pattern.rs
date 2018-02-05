// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use qt;

use dom;
use math::{
    Size,
    Rect,
};

use traits::{
    ConvTransform,
};

use render_utils;


pub fn apply(
    doc: &dom::Document,
    global_ts: qt::Transform,
    bbox: &Rect,
    pattern_node: dom::DefsNodeRef,
    pattern: &dom::Pattern,
    brush: &mut qt::Brush,
) {
    let r = if pattern.units == dom::Units::ObjectBoundingBox {
        let mut pr = pattern.rect;
        let ts = dom::Transform::new(bbox.w, 0.0, 0.0, bbox.h, bbox.x, bbox.y);
        ts.apply_ref(&mut pr.x, &mut pr.y);
        ts.apply_ref(&mut pr.w, &mut pr.h);
        pr
    } else {
        pattern.rect
    };

    let global_ts = dom::Transform::from_native(&global_ts);
    let (sx, sy) = global_ts.get_scale();

    let img_size = Size::new(r.w * sx, r.h * sy);
    let img = qt::Image::new(img_size.w as u32, img_size.h as u32);
    let mut img = match img {
        Some(img) => img,
        None => {
            // TODO: add expected image size
            warn!("Subimage creation failed.");
            return;
        }
    };

    img.set_dpi(doc.svg_node().dpi);
    img.fill(0, 0, 0, 0);

    let p = qt::Painter::new(&img);

    p.apply_transform(&qt::Transform::new(sx, 0.0, 0.0, sy, 0.0, 0.0));
    if let Some(vbox) = pattern.view_box {
        let img_view = Rect::new(0.0, 0.0, r.w, r.h);
        let (dx, dy, sx2, sy2) = render_utils::view_box_transform(&vbox, &img_view);
        p.apply_transform(&qt::Transform::new(sx2, 0.0, 0.0, sy2, dx, dy));
    }
    if pattern.content_units == dom::Units::ObjectBoundingBox {
        p.apply_transform(&qt::Transform::new(bbox.w, 0.0, 0.0, bbox.h, bbox.x, bbox.y));
    }

    super::render_group(doc, pattern_node.to_node_ref(), &p, &p.get_transform(), img_size);
    p.end();

    brush.set_pattern(img);

    let mut ts = dom::Transform::default();
    ts.append(&pattern.transform);
    ts.translate(r.x, r.y);
    ts.scale(1.0 / sx, 1.0 / sy);
    brush.set_transform(ts.to_native());
}
