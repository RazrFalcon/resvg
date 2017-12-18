// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Cairo backend implementation.

use cairo::{
    self,
    MatrixTrait,
};

use dom;

use math::{
    Size,
    Rect,
};

use {
    ErrorKind,
    Options,
    Result,
};

use render_utils;


mod ext;
mod fill;
mod gradient;
mod image;
mod path;
mod stroke;
mod text;

use self::ext::*;


/// Renders SVG to image.
pub fn render_to_image(doc: &dom::Document, opt: &Options) -> Result<cairo::ImageSurface> {
    let img_size = render_utils::fit_to(&doc.size, opt.fit_to);

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
        let (dx, dy, sx, sy) = render_utils::view_box_transform(&doc.view_box, &img_view);
        cairo::Matrix::new(sx, 0.0, 0.0, sy, dx, dy)
    };
    cr.set_matrix(ts);

    render_group(doc, &doc.elements, &cr, &cr.get_matrix(), img_view.size());
}

fn render_group(
    doc: &dom::Document,
    elements: &[dom::Element],
    cr: &cairo::Context,
    matrix: &cairo::Matrix,
    img_size: Size,
) {
    for elem in elements {
        cr.apply_transform(&elem.transform);

        match elem.data {
            dom::Type::Path(ref path) => {
                path::draw(doc, path, cr);
            }
            dom::Type::Text(ref text) => {
                text::draw(doc, text, cr);
            }
            dom::Type::Image(ref img) => {
                image::draw(img, cr);
            }
            dom::Type::Group(ref g) => {
                let sub_surface = cairo::ImageSurface::create(
                    cairo::Format::ARgb32,
                    img_size.w as i32,
                    img_size.h as i32
                );

                let sub_surface = match sub_surface {
                    Ok(surf) => surf,
                    Err(_) => {
                        warn!("Subsurface creation failed.");
                        continue;
                    }
                };

                let sub_cr = cairo::Context::new(&sub_surface);
                sub_cr.set_matrix(cr.get_matrix());

                render_group(doc, &g.children, &sub_cr, &cr.get_matrix(), img_size);

                let curr_matrix = cr.get_matrix();
                cr.set_matrix(cairo::Matrix::identity());

                cr.set_source_surface(&sub_surface, 0.0, 0.0);

                if let Some(opacity) = g.opacity {
                    cr.paint_with_alpha(opacity);
                } else {
                    cr.paint();
                }

                cr.set_matrix(curr_matrix);
            }
        }

        cr.set_matrix(*matrix);
    }
}
