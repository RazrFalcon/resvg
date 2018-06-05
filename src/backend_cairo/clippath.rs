// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use cairo::{
    self,
    MatrixTrait,
};
use usvg;
use usvg::prelude::*;

// self
use super::prelude::*;
use super::{
    path,
    text,
};


pub fn apply(
    node: &usvg::Node,
    cp: &usvg::ClipPath,
    opt: &Options,
    bbox: Rect,
    layers: &mut CairoLayers,
    cr: &cairo::Context,
) {
    // a-clip-path-001.svg
    // e-clipPath-001.svg

    let clip_surface = try_opt!(layers.get(), ());
    let clip_surface = clip_surface.borrow_mut();

    let clip_cr = cairo::Context::new(&*clip_surface);
    clip_cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
    clip_cr.paint();
    // e-clipPath-006.svg
    // e-clipPath-007.svg
    clip_cr.set_matrix(cr.get_matrix());
    // e-clipPath-008.svg
    clip_cr.transform(cp.transform.to_native());

    // e-clipPath-005.svg
    if cp.units == usvg::Units::ObjectBoundingBox {
        let m = cairo::Matrix::from_bbox(bbox);
        clip_cr.transform(m);
    }

    clip_cr.set_operator(cairo::Operator::Clear);

    let matrix = clip_cr.get_matrix();
    // e-clipPath-015.svg
    // e-clipPath-017.svg
    for node in node.children() {
        clip_cr.transform(node.transform().to_native());

        match *node.borrow() {
            usvg::NodeKind::Path(ref p) => {
                path::draw(&node.tree(), p, opt, &clip_cr);
            }
            usvg::NodeKind::Text(ref text) => {
                // e-clipPath-009.svg
                // e-clipPath-010.svg
                // e-clipPath-011.svg
                // e-clipPath-012.svg
                text::draw(&node.tree(), text, opt, &clip_cr);
            }
            _ => {}
        }

        clip_cr.set_matrix(matrix);
    }

    cr.set_matrix(cairo::Matrix::identity());
    cr.set_source_surface(&*clip_surface, 0.0, 0.0);
    cr.set_operator(cairo::Operator::DestOut);
    cr.paint();

    // Reset operator.
    cr.set_operator(cairo::Operator::Over);

    // Reset source to unborrow the `clip_surface` from the `Context`.
    cr.reset_source_rgba();
}
