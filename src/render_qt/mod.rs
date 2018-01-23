// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Qt backend implementation.

use std::f64;

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

use traits::{
    ConvTransform,
};

use render_utils;


mod clippath;
mod fill;
mod gradient;
mod image;
mod path;
mod pattern;
mod stroke;
mod text;


impl ConvTransform<qt::Transform> for dom::Transform {
    fn to_native(&self) -> qt::Transform {
        qt::Transform::new(self.a, self.b, self.c, self.d, self.e, self.f)
    }

    fn from_native(ts: &qt::Transform) -> Self {
        let d = ts.data();
        Self::new(d.0, d.1, d.2, d.3, d.4, d.5)
    }
}


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
    painter.apply_transform(&ts);

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
) -> Rect {
    let mut g_bbox = Rect::new(f64::MAX, f64::MAX, 0.0, 0.0);
    for elem in elements {
        // Apply transform.
        p.apply_transform(&elem.transform.to_native());

        let bbox = match elem.kind {
            dom::ElementKind::Path(ref path) => {
                Some(path::draw(doc, path, p))
            }
            dom::ElementKind::Text(ref text) => {
                Some(text::draw(doc, text, p))
            }
            dom::ElementKind::Image(ref img) => {
                Some(image::draw(img, p))
            }
            dom::ElementKind::Group(ref g) => {
                render_group_impl(doc, g, p, img_size)
            }
        };

        if let Some(bbox) = bbox {
            g_bbox.expand_from_rect(&bbox);
        }

        // Revert transform.
        p.set_transform(ts);
    }

    g_bbox
}

fn render_group_impl(
    doc: &dom::Document,
    g: &dom::Group,
    p: &qt::Painter,
    img_size: Size,
) -> Option<Rect> {
    let sub_img = qt::Image::new(
        img_size.w as u32,
        img_size.h as u32,
    );

    let mut sub_img = match sub_img {
        Some(img) => img,
        None => {
            warn!("Subimage creation failed.");
            return None;
        }
    };

    sub_img.fill(0, 0, 0, 0);
    sub_img.set_dpi(doc.dpi);

    let sub_p = qt::Painter::new(&sub_img);
    sub_p.set_transform(&p.get_transform());
    let bbox = render_group(doc, &g.children, &sub_p, &p.get_transform(), img_size);

    if let Some(idx) = g.clip_path {
        if let dom::RefElementKind::ClipPath(ref cp) = doc.get_defs(idx).kind {
            clippath::apply(doc, cp, &sub_p, &bbox, img_size);
        }
    }

    sub_p.end();

    if let Some(opacity) = g.opacity {
        p.set_opacity(opacity);
    }

    let curr_ts = p.get_transform();
    p.set_transform(&qt::Transform::default());

    p.draw_image(0.0, 0.0, &sub_img);

    p.set_opacity(1.0);
    p.set_transform(&curr_ts);

    Some(bbox)
}
