// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use cairo::{
    self,
    MatrixTrait,
    Pattern,
};

use tree;
use math::{
    Size,
    Rect,
};

use traits::{
    ConvTransform,
};

use render_utils;


pub fn apply(
    doc: &tree::RenderTree,
    node: tree::DefsNodeRef,
    pattern: &tree::Pattern,
    bbox: &Rect,
    cr: &cairo::Context,
) {
    let r = if pattern.units == tree::Units::ObjectBoundingBox {
        let mut pr = pattern.rect;
        let ts = tree::Transform::new(bbox.w, 0.0, 0.0, bbox.h, bbox.x, bbox.y);
        ts.apply_ref(&mut pr.x, &mut pr.y);
        ts.apply_ref(&mut pr.w, &mut pr.h);
        pr
    } else {
        pattern.rect
    };

    let global_ts = tree::Transform::from_native(&cr.get_matrix());
    let (sx, sy) = global_ts.get_scale();

    let img_size = Size::new(r.w * sx, r.h * sy);
    let surface = cairo::ImageSurface::create(
        cairo::Format::ARgb32,
        img_size.w as i32,
        img_size.h as i32
    );

    let surface = match surface {
        Ok(surf) => surf,
        Err(_) => {
            warn!("Subsurface creation failed.");
            return;
        }
    };

    let sub_cr = cairo::Context::new(&surface);
    sub_cr.transform(cairo::Matrix::new(sx, 0.0, 0.0, sy, 0.0, 0.0));

    if let Some(vbox) = pattern.view_box {
        let img_view = Rect::new(0.0, 0.0, r.w, r.h);
        let (dx, dy, sx2, sy2) = render_utils::view_box_transform(&vbox, &img_view);
        sub_cr.transform(cairo::Matrix::new(sx2, 0.0, 0.0, sy2, dx, dy));
    }
    if pattern.content_units == tree::Units::ObjectBoundingBox {
        sub_cr.transform(cairo::Matrix::new(bbox.w, 0.0, 0.0, bbox.h, bbox.x, bbox.y));
    }

    super::render_group(doc, node.to_node_ref(), &sub_cr, &sub_cr.get_matrix(), img_size);

    let mut ts = tree::Transform::default();
    ts.append(&pattern.transform);
    ts.translate(r.x, r.y);
    ts.scale(1.0 / sx, 1.0 / sy);

    let patt = cairo::SurfacePattern::create(&surface);
    patt.set_extend(cairo::Extend::Repeat);
    patt.set_filter(cairo::Filter::Best);

    let mut m: cairo::Matrix = ts.to_native();
    m.invert();
    patt.set_matrix(m);

    cr.set_source(&patt);
}
