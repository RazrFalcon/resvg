// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Cairo backend implementation.

// external
use cairo::{
    self,
    MatrixTrait,
};
use pangocairo::functions as pc;

// self
use tree::prelude::*;
use math::*;
use traits::{
    ConvTransform,
    TransformFromBBox,
};
use {
    ErrorKind,
    Options,
    OutputImage,
    Render,
    Result,
};
use utils;
use self::ext::*;


mod clippath;
mod ext;
mod fill;
mod gradient;
mod image;
mod path;
mod pattern;
mod stroke;
mod text;


impl ConvTransform<cairo::Matrix> for tree::Transform {
    fn to_native(&self) -> cairo::Matrix {
        cairo::Matrix::new(self.a, self.b, self.c, self.d, self.e, self.f)
    }

    fn from_native(ts: &cairo::Matrix) -> Self {
        Self::new(ts.xx, ts.yx, ts.xy, ts.yy, ts.x0, ts.y0)
    }
}

impl TransformFromBBox for cairo::Matrix {
    fn from_bbox(bbox: Rect) -> Self {
        debug_assert!(!bbox.width().is_fuzzy_zero());
        debug_assert!(!bbox.height().is_fuzzy_zero());

        Self::new(bbox.width(), 0.0, 0.0, bbox.height(), bbox.x(), bbox.y())
    }
}

/// Cairo backend handle.
#[derive(Clone, Copy)]
pub struct Backend;

impl Render for Backend {
    fn render_to_image(
        &self,
        rtree: &tree::RenderTree,
        opt: &Options,
    ) -> Result<Box<OutputImage>> {
        let img = render_to_image(rtree, opt)?;
        Ok(Box::new(img))
    }

    fn render_node_to_image(
        &self,
        node: tree::NodeRef,
        opt: &Options,
    ) -> Result<Box<OutputImage>> {
        let img = render_node_to_image(node, opt)?;
        Ok(Box::new(img))
    }

    fn calc_node_bbox(
        &self,
        node: tree::NodeRef,
        opt: &Options,
    ) -> Option<Rect> {
        calc_node_bbox(node, opt)
    }
}

impl OutputImage for cairo::ImageSurface {
    fn save(&self, path: &::std::path::Path) -> bool {
        use std::fs;

        if let Ok(mut buffer) = fs::File::create(path) {
            if let Ok(_) = self.write_to_png(&mut buffer) {
                return true;
            }
        }

        false
    }
}


/// Renders SVG to image.
pub fn render_to_image(
    rtree: &tree::RenderTree,
    opt: &Options,
) -> Result<cairo::ImageSurface> {
    let (surface, img_view) = create_surface(
        rtree.svg_node().size.to_screen_size(),
        opt,
    )?;

    let cr = cairo::Context::new(&surface);

    // Fill background.
    if let Some(color) = opt.background {
        cr.set_source_color(&color, 1.0);
        cr.paint();
    }

    render_to_canvas(rtree, opt, img_view, &cr);

    Ok(surface)
}

/// Renders SVG to image.
pub fn render_node_to_image(
    node: tree::NodeRef,
    opt: &Options,
) -> Result<cairo::ImageSurface> {
    let node_bbox = if let Some(bbox) = calc_node_bbox(node, opt) {
        bbox
    } else {
        warn!("Node {:?} has zero size.", node.svg_id());
        return Err(ErrorKind::NoCanvas.into());
    };

    let (surface, img_view) = create_surface(node_bbox.to_screen_size(), opt)?;

    let cr = cairo::Context::new(&surface);

    // Fill background.
    if let Some(color) = opt.background {
        cr.set_source_color(&color, 1.0);
        cr.paint();
    }

    apply_viewbox_transform(node_bbox, img_view, &cr);
    render_node_to_canvas(node, opt, img_view.to_screen_size(), &cr);

    Ok(surface)
}

/// Renders SVG to canvas.
pub fn render_to_canvas(
    rtree: &tree::RenderTree,
    opt: &Options,
    img_view: Rect,
    cr: &cairo::Context,
) {
    apply_viewbox_transform(rtree.svg_node().view_box, img_view, cr);
    render_group(rtree.root(), opt, img_view.to_screen_size(), &cr);
}

/// Renders SVG node to canvas.
pub fn render_node_to_canvas(
    node: tree::NodeRef,
    opt: &Options,
    img_size: ScreenSize,
    cr: &cairo::Context,
) {
    let curr_ts = cr.get_matrix();
    let mut ts = utils::abs_transform(node);
    ts.append(&node.transform());

    cr.transform(ts.to_native());
    render_node(node, opt, img_size, cr);
    cr.set_matrix(curr_ts);
}

fn create_surface(
    size: ScreenSize,
    opt: &Options,
) -> Result<(cairo::ImageSurface, Rect)> {
    let img_size = utils::fit_to(size, opt.fit_to);

    debug_assert!(!img_size.is_empty_or_negative());

    let surface = cairo::ImageSurface::create(
        cairo::Format::ARgb32,
        img_size.width as i32,
        img_size.height as i32
    );

    let surface = match surface {
        Ok(v) => v,
        Err(_) => {
            return Err(ErrorKind::NoCanvas.into());
        }
    };

    let img_view = Rect::new(Point::new(0.0, 0.0), img_size.to_f64());

    Ok((surface, img_view))
}

/// Applies viewbox transformation to the painter.
fn apply_viewbox_transform(
    view_box: Rect,
    img_view: Rect,
    cr: &cairo::Context,
) {
    let ts = {
        let (dx, dy, sx, sy) = utils::view_box_transform(view_box, img_view);
        cairo::Matrix::new(sx, 0.0, 0.0, sy, dx, dy)
    };
    cr.transform(ts);
}

fn render_group(
    parent: tree::NodeRef,
    opt: &Options,
    img_size: ScreenSize,
    cr: &cairo::Context,
) -> Rect {
    let curr_ts = cr.get_matrix();
    let mut g_bbox = Rect::new_bbox();

    for node in parent.children() {
        cr.transform(node.transform().to_native());

        let bbox = render_node(node, opt, img_size, cr);

        if let Some(bbox) = bbox {
            g_bbox.expand(bbox);
        }

        // Revert transform.
        cr.set_matrix(curr_ts);
    }

    g_bbox
}

fn render_group_impl(
    node: tree::NodeRef,
    g: &tree::Group,
    opt: &Options,
    img_size: ScreenSize,
    cr: &cairo::Context,
) -> Option<Rect> {
    let sub_surface = cairo::ImageSurface::create(
        cairo::Format::ARgb32,
        img_size.width as i32,
        img_size.height as i32
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

    let bbox = render_group(node, opt, img_size, &sub_cr);

    if let Some(idx) = g.clip_path {
        if let Some(clip_node) = node.tree().defs_at(idx) {
            if let tree::NodeKind::ClipPath(ref cp) = *clip_node.value() {
                clippath::apply(clip_node, cp, opt, bbox, img_size, &sub_cr);
            }
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

fn render_node(
    node: tree::NodeRef,
    opt: &Options,
    img_size: ScreenSize,
    cr: &cairo::Context,
) -> Option<Rect> {
    match *node.value() {
        tree::NodeKind::Path(ref path) => {
            Some(path::draw(node.tree(), path, opt, cr))
        }
        tree::NodeKind::Text(_) => {
            Some(text::draw(node, opt, cr))
        }
        tree::NodeKind::Image(ref img) => {
            Some(image::draw(img, cr))
        }
        tree::NodeKind::Group(ref g) => {
            render_group_impl(node, g, opt, img_size, cr)
        }
        _ => None,
    }
}

/// Calculates node's absolute bounding box.
///
/// Note: this method can be pretty expensive.
pub fn calc_node_bbox(
    node: tree::NodeRef,
    opt: &Options,
) -> Option<Rect> {
    // We can't use 1x1 image, like in Qt backend because otherwise
    // text layouts will be truncated.
    let (surface, img_view) = create_surface(
        node.tree().svg_node().size.to_screen_size(),
        opt,
    ).unwrap();
    let cr = cairo::Context::new(&surface);

    // We also have to apply the viewbox transform,
    // otherwise text hinting will be different and bbox will be different too.
    apply_viewbox_transform(node.tree().svg_node().view_box, img_view, &cr);

    let abs_ts = utils::abs_transform(node);
    _calc_node_bbox(node, opt, abs_ts, &cr)
}

fn _calc_node_bbox(
    node: tree::NodeRef,
    opt: &Options,
    ts: tree::Transform,
    cr: &cairo::Context,
) -> Option<Rect> {
    let mut ts2 = ts;
    ts2.append(&node.transform());

    match *node.value() {
        tree::NodeKind::Path(ref path) => {
            Some(utils::path_bbox(&path.segments, path.stroke.as_ref(), &ts2))
        }
        tree::NodeKind::Text(_) => {
            let mut bbox = Rect::new_bbox();

            text::draw_tspan(node, opt, cr, |tspan, x, y, _, pd| {
                cr.new_path();

                pc::layout_path(cr, &pd.layout);
                let path = cr.copy_path();
                let segments = from_cairo_path(&path);

                let mut t = ts2;
                t.append(&tree::Transform::new(1.0, 0.0, 0.0, 1.0, x, y));

                if !segments.is_empty() {
                    let c_bbox = utils::path_bbox(&segments, tspan.stroke.as_ref(), &t);
                    bbox.expand(c_bbox);
                }
            });

            Some(bbox)
        }
        tree::NodeKind::Image(ref img) => {
            let segments = utils::rect_to_path(img.rect);
            Some(utils::path_bbox(&segments, None, &ts2))
        }
        tree::NodeKind::Group(_) => {
            let mut bbox = Rect::new_bbox();

            for child in node.children() {
                if let Some(c_bbox) = _calc_node_bbox(child, opt, ts2, cr) {
                    bbox.expand(c_bbox);
                }
            }

            Some(bbox)
        }
        _ => None
    }
}

fn from_cairo_path(path: &cairo::Path) -> Vec<tree::PathSegment> {
    let mut segments = Vec::new();
    for seg in path.iter() {
        match seg {
            cairo::PathSegment::MoveTo((x, y)) => {
                segments.push(tree::PathSegment::MoveTo { x, y });
            }
            cairo::PathSegment::LineTo((x, y)) => {
                segments.push(tree::PathSegment::LineTo { x, y });
            }
            cairo::PathSegment::CurveTo((x1, y1), (x2, y2), (x, y)) => {
                segments.push(tree::PathSegment::CurveTo { x1, y1, x2, y2, x, y });
            }
            cairo::PathSegment::ClosePath => {
                segments.push(tree::PathSegment::ClosePath);
            }
        }
    }

    if segments.len() < 2 {
        segments.clear();
    }

    segments
}
