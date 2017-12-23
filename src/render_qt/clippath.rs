// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use qt;

use dom;

use math::{
    Size,
    Rect,
};

use super::ext::{
    TransformToMatrix,
};
use super::{
    path,
    text,
};


pub fn apply(
    doc: &dom::Document,
    cp: &dom::ClipPath,
    p: &qt::Painter,
    bbox: &Rect,
    img_size: Size,
) {
    let mut clip_img = qt::Image::new(
        img_size.w as u32,
        img_size.h as u32,
    ).unwrap();

    clip_img.fill(0, 0, 0, 255);
    clip_img.set_dpi(doc.dpi);

    let clip_p = qt::Painter::new(&clip_img);
    clip_p.set_transform(&p.get_transform());
    clip_p.apply_transform(&cp.transform.to_qtransform());

    if cp.units == dom::Units::ObjectBoundingBox {
        clip_p.apply_transform(&qt::Transform::new(bbox.w, 0.0, 0.0, bbox.h, bbox.x, bbox.y));
    }

    clip_p.set_composition_mode(qt::CompositionMode::CompositionMode_Clear);

    let ts = clip_p.get_transform();
    for elem in &cp.children {
        clip_p.apply_transform(&elem.transform.to_qtransform());

        match elem.kind {
            dom::ElementKind::Path(ref path_elem) => {
                path::draw(doc, path_elem, &clip_p);
            }
            dom::ElementKind::Text(ref text_elem) => {
                text::draw(doc, text_elem, &clip_p);
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
