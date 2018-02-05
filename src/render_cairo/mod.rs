// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Cairo backend implementation.

use std::f64;

use cairo::{
    self,
    MatrixTrait,
};

use dom;

use math::{
    Size,
    Rect,
};

use traits::{
    ConvTransform,
};

use {
    ErrorKind,
    Options,
    Result,
};

use render_utils;

use self::ext::{
    ReCairoContextExt,
};


mod clippath;
mod ext;
mod fill;
mod gradient;
mod image;
mod path;
mod pattern;
mod stroke;
mod text;


impl ConvTransform<cairo::Matrix> for dom::Transform {
    fn to_native(&self) -> cairo::Matrix {
        cairo::Matrix::new(self.a, self.b, self.c, self.d, self.e, self.f)
    }

    fn from_native(ts: &cairo::Matrix) -> Self {
        Self::new(ts.xx, ts.yx, ts.xy, ts.yy, ts.x0, ts.y0)
    }
}


/// Renders SVG to image.
pub fn render_to_image(doc: &dom::Document, opt: &Options) -> Result<cairo::ImageSurface> {
    let img_size = render_utils::fit_to(&doc.svg_node().size, opt.fit_to);

    debug_assert!(img_size.w as i32 > 0 && img_size.h as i32 > 0);

    let surface = cairo::ImageSurface::create(
        cairo::Format::ARgb32,
        img_size.w as i32,
        img_size.h as i32
    );

    let surface = match surface {
        Ok(v) => v,
        Err(_) => {
            return Err(ErrorKind::NoCanvas.into());
        }
    };

    let img_view = Rect::new(0.0, 0.0, img_size.w, img_size.h);
    let cr = cairo::Context::new(&surface);

    // Fill background.
    if let Some(color) = opt.background {
        cr.set_source_color(&color, 1.0);
        cr.paint();
    }

    render_to_canvas(&cr, img_view, doc);

    Ok(surface)
}

/// Renders SVG to canvas.
pub fn render_to_canvas(cr: &cairo::Context, img_view: Rect, doc: &dom::Document) {
    // Apply viewBox.
    let ts = {
        let (dx, dy, sx, sy) = render_utils::view_box_transform(&doc.svg_node().view_box, &img_view);
        cairo::Matrix::new(sx, 0.0, 0.0, sy, dx, dy)
    };
    cr.transform(ts);

    render_group(doc, doc.root(), &cr, &cr.get_matrix(), img_view.size());
}

fn render_group(
    doc: &dom::Document,
    node: dom::NodeRef,
    cr: &cairo::Context,
    matrix: &cairo::Matrix,
    img_size: Size,
) -> Rect {
    let mut g_bbox = Rect::new(f64::MAX, f64::MAX, 0.0, 0.0);
    for node in node.children() {
        cr.transform(node.kind().transform().to_native());

        let bbox = match node.kind() {
            dom::NodeKindRef::Path(ref path) => {
                Some(path::draw(doc, path, cr))
            }
            dom::NodeKindRef::Text(_) => {
                Some(text::draw(doc, node, cr))
            }
            dom::NodeKindRef::Image(ref img) => {
                Some(image::draw(img, cr))
            }
            dom::NodeKindRef::Group(ref g) => {
                render_group_impl(doc, node, g, cr, img_size)
            }
        };

        if let Some(bbox) = bbox {
            g_bbox.expand_from_rect(&bbox);
        }

        cr.set_matrix(*matrix);
    }

    g_bbox
}

fn render_group_impl(
    doc: &dom::Document,
    node: dom::NodeRef,
    g: &dom::Group,
    cr: &cairo::Context,
    img_size: Size,
) -> Option<Rect> {
    let sub_surface = cairo::ImageSurface::create(
        cairo::Format::ARgb32,
        img_size.w as i32,
        img_size.h as i32
    );

    let sub_surface = match sub_surface {
        Ok(surf) => surf,
        Err(_) => {
            warn!("Subsurface creation failed.");
            return None;
        }
    };

    let sub_cr = cairo::Context::new(&sub_surface);
    sub_cr.set_matrix(cr.get_matrix());

    let bbox = render_group(doc, node, &sub_cr, &cr.get_matrix(), img_size);

    if let Some(idx) = g.clip_path {
        let clip_node = doc.defs_at(idx);
        if let dom::DefsNodeKindRef::ClipPath(ref cp) = clip_node.kind() {
            clippath::apply(doc, clip_node, cp, &sub_cr, &bbox, img_size);
        }
    }

    let curr_matrix = cr.get_matrix();
    cr.set_matrix(cairo::Matrix::identity());

    cr.set_source_surface(&sub_surface, 0.0, 0.0);

    if let Some(opacity) = g.opacity {
        cr.paint_with_alpha(opacity);
    } else {
        cr.paint();
    }

    cr.set_matrix(curr_matrix);

    Some(bbox)
}
