// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use cairo::{
    self,
    MatrixTrait,
};
use usvg;

// self
use geom::*;
use traits::{
    TransformFromBBox,
};
use super::{
    CairoLayers,
};
use {
    utils,
    Options,
};


pub fn apply(
    node: &usvg::Node,
    mask: &usvg::Mask,
    opt: &Options,
    bbox: Rect,
    opacity: Option<usvg::Opacity>,
    layers: &mut CairoLayers,
    cr: &cairo::Context,
) {
    let mask_surface = try_opt!(layers.get(), ());
    let mut mask_surface = mask_surface.borrow_mut();

    {
        let mask_cr = cairo::Context::new(&*mask_surface);
        mask_cr.set_matrix(cr.get_matrix());

        let r = if mask.units == usvg::Units::ObjectBoundingBox {
            mask.rect.transform(usvg::Transform::from_bbox(bbox))
        } else {
            mask.rect
        };

        mask_cr.rectangle(r.x(), r.y(), r.width(), r.height());
        mask_cr.clip();

        if mask.content_units == usvg::Units::ObjectBoundingBox {
            mask_cr.transform(cairo::Matrix::from_bbox(bbox));
        }

        super::render_group(node, opt, layers, &mask_cr);
    }

    {
        let mut data = try_opt_warn!(mask_surface.get_data().ok(),
                                     { layers.release(); () },
                                     "Failed to borrow a surface for mask: {:?}.", mask.id);
        utils::image_to_mask(&mut data, layers.image_size(), opacity);
    }

    let patt = cairo::SurfacePattern::create(&*mask_surface);
    cr.set_matrix(cairo::Matrix::identity());
    cr.mask(&patt);

    layers.release();
}
