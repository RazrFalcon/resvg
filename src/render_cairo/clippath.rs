// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use cairo::{
    self,
    MatrixTrait,
};

// self
use tree;
use math::*;
use traits::{
    ConvTransform,
    TransformFromBBox,
};
use super::{
    path,
    text,
};


pub fn apply(
    rtree: &tree::RenderTree,
    node: tree::DefsNodeRef,
    cp: &tree::ClipPath,
    cr: &cairo::Context,
    bbox: Rect,
    img_size: Size,
) {
    let clip_surface = cairo::ImageSurface::create(
        cairo::Format::ARgb32,
        img_size.width as i32,
        img_size.height as i32
    ).unwrap();

    let clip_cr = cairo::Context::new(&clip_surface);
    clip_cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
    clip_cr.paint();
    clip_cr.set_matrix(cr.get_matrix());
    clip_cr.transform(cp.transform.to_native());

    if cp.units == tree::Units::ObjectBoundingBox {
        let m = cairo::Matrix::from_bbox(bbox);
        clip_cr.transform(m);
    }

    clip_cr.set_operator(cairo::Operator::Clear);

    let matrix = clip_cr.get_matrix();
    for node in node.children() {
        clip_cr.transform(node.kind().transform().to_native());

        match node.kind() {
            tree::NodeKindRef::Path(ref path_elem) => {
                path::draw(rtree, path_elem, &clip_cr);
            }
            tree::NodeKindRef::Text(_) => {
                text::draw(rtree, node, &clip_cr);
            }
            _ => {}
        }

        clip_cr.set_matrix(matrix);
    }

    clip_cr.set_operator(cairo::Operator::Over);

    cr.set_matrix(cairo::Matrix::identity());
    cr.set_source_surface(&clip_surface, 0.0, 0.0);
    cr.set_operator(cairo::Operator::DestOut);
    cr.paint();
}
