// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Qt backend implementation.

use qt;

use dom;

use {
    ErrorKind,
    Options,
    Result,
};

use math::{
    Size,
    Rect,
};

use render_utils;


mod ext;
mod fill;
mod gradient;
mod image;
mod path;
mod stroke;
mod text;

use self::ext::{
    TransformToMatrix,
};


/// Renders SVG to image.
pub fn render_to_image(doc: &dom::Document, opt: &Options) -> Result<qt::Image> {
    let _app = qt::GuiApp::new("resvg");

    let img_size = render_utils::fit_to(&doc.size, opt.fit_to);

    debug_assert!(img_size.w as i32 > 0 && img_size.h as i32 > 0);

    let img = qt::Image::new(img_size.w as u32, img_size.h as u32);

    let mut img = match img {
        Some(v) => v,
        None => {
            return Err(ErrorKind::NoCanvas.into());
        }
    };

    // Fill background.
    if let Some(c) = opt.background {
        img.fill(c.red, c.green, c.blue, 255);
    } else {
        img.fill(0, 0, 0, 0);
    }
    img.set_dpi(opt.dpi);

    let img_view = Rect::new(0.0, 0.0, img_size.w, img_size.h);
    let painter = qt::Painter::new(&img);

    render_to_canvas(&painter, img_view, doc);

    painter.end();

    Ok(img)
}

/// Renders SVG to canvas.
pub fn render_to_canvas(painter: &qt::Painter, img_view: Rect, doc: &dom::Document) {
    // Apply viewBox.
    let ts = {
        let (dx, dy, sx, sy) = render_utils::view_box_transform(&doc.view_box, &img_view);
        qt::Transform::new(sx, 0.0, 0.0, sy, dx, dy)
    };
    painter.set_transform(&ts);

    render_group(doc, &doc.elements, &painter, &painter.get_transform(), img_view.size());
}

// TODO: render groups backward to reduce memory usage
//       current implementation keeps parent canvas until all children are rendered
fn render_group(
    doc: &dom::Document,
    elements: &[dom::Element],
    p: &qt::Painter,
    ts: &qt::Transform,
    img_size: Size,
) {
    for elem in elements {
        // Apply transform.
        p.apply_transform(&elem.transform.to_qtransform());

        match elem.data {
            dom::Type::Path(ref path) => {
                path::draw(doc, path, p);
            }
            dom::Type::Text(ref text) => {
                text::draw(doc, text, p);
            }
            dom::Type::Image(ref img) => {
                image::draw(img, p);
            }
            dom::Type::Group(ref g) => {
                let sub_img = qt::Image::new(
                    img_size.w as u32,
                    img_size.h as u32
                );

                let mut sub_img = match sub_img {
                    Some(img) => img,
                    None => {
                        warn!("Subimage creation failed.");
                        continue;
                    }
                };

                sub_img.fill(0, 0, 0, 0);
                sub_img.set_dpi(doc.dpi);

                let sub_p = qt::Painter::new(&sub_img);
                sub_p.set_transform(&p.get_transform());

                render_group(doc, &g.children, &sub_p, &p.get_transform(), img_size);

                sub_p.end();

                let curr_ts = p.get_transform();
                p.set_transform(&qt::Transform::default());
                if let Some(opacity) = g.opacity {
                    p.set_opacity(opacity);
                }

                p.draw_image(0.0, 0.0, &sub_img);

                p.set_opacity(1.0);
                p.set_transform(&curr_ts);
            }
        }

        // Revert transform.
        p.set_transform(ts);
    }
}
