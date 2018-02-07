// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Qt backend implementation.

use std::f64;

// external
use qt;

// self
use tree::{
    self,
    NodeExt,
};
use math::*;
use traits::{
    ConvTransform,
    TransformFromBBox,
};
use {
    ErrorKind,
    Options,
    Result,
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


impl ConvTransform<qt::Transform> for tree::Transform {
    fn to_native(&self) -> qt::Transform {
        qt::Transform::new(self.a, self.b, self.c, self.d, self.e, self.f)
    }

    fn from_native(ts: &qt::Transform) -> Self {
        let d = ts.data();
        Self::new(d.0, d.1, d.2, d.3, d.4, d.5)
    }
}

impl TransformFromBBox for qt::Transform {
    fn from_bbox(bbox: Rect) -> Self {
        Self::new(bbox.width(), 0.0, 0.0, bbox.height(), bbox.x(), bbox.y())
    }
}


/// Renders SVG to image.
pub fn render_to_image(
    rtree: &tree::RenderTree,
    opt: &Options,
) -> Result<qt::Image> {
    let _app = qt::GuiApp::new("resvg");

    let svg = rtree.svg_node();

    let img_size = render_utils::fit_to(svg.size, opt.fit_to);

    debug_assert!(!img_size.is_empty_or_negative());

    let img = qt::Image::new(img_size.width as u32, img_size.height as u32);

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

    let img_view = Rect::new(Point::new(0.0, 0.0), img_size);
    let painter = qt::Painter::new(&img);

    render_to_canvas(&painter, img_view, rtree);

    painter.end();

    Ok(img)
}

/// Renders SVG to canvas.
pub fn render_to_canvas(
    painter: &qt::Painter,
    img_view: Rect,
    rtree: &tree::RenderTree,
) {
    let svg = rtree.svg_node();

    // Apply viewBox.
    let ts = {
        let (dx, dy, sx, sy) = render_utils::view_box_transform(svg.view_box, img_view);
        qt::Transform::new(sx, 0.0, 0.0, sy, dx, dy)
    };
    painter.apply_transform(&ts);

    render_group(rtree, rtree.root(), &painter, &painter.get_transform(), img_view.size);
}

// TODO: render groups backward to reduce memory usage
//       current implementation keeps parent canvas until all children are rendered
fn render_group(
    rtree: &tree::RenderTree,
    node: tree::NodeRef,
    p: &qt::Painter,
    ts: &qt::Transform,
    img_size: Size,
) -> Rect {
    let mut g_bbox = Rect::from_xywh(f64::MAX, f64::MAX, 0.0, 0.0);
    for node in node.children() {
        // Apply transform.
        p.apply_transform(&node.transform().to_native());

        let bbox = match *node.value() {
            tree::NodeKind::Path(ref path) => {
                Some(path::draw(rtree, path, p))
            }
            tree::NodeKind::Text(_) => {
                Some(text::draw(rtree, node, p))
            }
            tree::NodeKind::Image(ref img) => {
                Some(image::draw(img, p))
            }
            tree::NodeKind::Group(ref g) => {
                render_group_impl(rtree, node, g, p, img_size)
            }
            _ => None,
        };

        if let Some(bbox) = bbox {
            g_bbox.expand_from_rect(bbox);
        }

        // Revert transform.
        p.set_transform(ts);
    }

    g_bbox
}

fn render_group_impl(
    rtree: &tree::RenderTree,
    node: tree::NodeRef,
    g: &tree::Group,
    p: &qt::Painter,
    img_size: Size,
) -> Option<Rect> {
    let sub_img = qt::Image::new(
        img_size.width as u32,
        img_size.height as u32,
    );

    let mut sub_img = match sub_img {
        Some(img) => img,
        None => {
            warn!("Subimage creation failed.");
            return None;
        }
    };

    sub_img.fill(0, 0, 0, 0);
    sub_img.set_dpi(rtree.svg_node().dpi);

    let sub_p = qt::Painter::new(&sub_img);
    sub_p.set_transform(&p.get_transform());
    let bbox = render_group(rtree, node, &sub_p, &p.get_transform(), img_size);

    if let Some(idx) = g.clip_path {
        let clip_node = rtree.defs_at(idx);
        if let tree::NodeKind::ClipPath(ref cp) = *clip_node.value() {
            clippath::apply(rtree, clip_node, cp, &sub_p, bbox, img_size);
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
