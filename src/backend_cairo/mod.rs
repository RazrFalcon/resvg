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
use usvg;
use usvg::prelude::*;

// self
use prelude::*;
use {
    backend_utils,
    layers,
    OutputImage,
    Render,
};
use self::ext::*;


macro_rules! try_create_surface {
    ($size:expr, $ret:expr) => {
        try_opt_warn!(
            cairo::ImageSurface::create(cairo::Format::ARgb32,
                $size.width as i32, $size.height as i32,
            ).ok(),
            $ret,
            "Failed to create a {}x{} surface.", $size.width, $size.height
        );
    };
}


mod clippath;
mod ext;
mod fill;
mod filter;
mod gradient;
mod image;
mod mask;
mod path;
mod pattern;
mod stroke;
mod text;

mod prelude {
    pub use super::super::prelude::*;
    pub type CairoLayers = super::layers::Layers<super::cairo::ImageSurface>;
    pub use super::ext::*;
}


type CairoLayers = layers::Layers<cairo::ImageSurface>;


impl ConvTransform<cairo::Matrix> for usvg::Transform {
    fn to_native(&self) -> cairo::Matrix {
        cairo::Matrix::new(self.a, self.b, self.c, self.d, self.e, self.f)
    }

    fn from_native(ts: &cairo::Matrix) -> Self {
        Self::new(ts.xx, ts.yx, ts.xy, ts.yy, ts.x0, ts.y0)
    }
}

impl TransformFromBBox for cairo::Matrix {
    fn from_bbox(bbox: Rect) -> Self {
        debug_assert!(!bbox.width.is_fuzzy_zero());
        debug_assert!(!bbox.height.is_fuzzy_zero());

        Self::new(bbox.width, 0.0, 0.0, bbox.height, bbox.x, bbox.y)
    }
}


/// Cairo backend handle.
#[derive(Clone, Copy)]
pub struct Backend;

impl Render for Backend {
    fn render_to_image(
        &self,
        tree: &usvg::Tree,
        opt: &Options,
    ) -> Option<Box<OutputImage>> {
        let img = render_to_image(tree, opt)?;
        Some(Box::new(img))
    }

    fn render_node_to_image(
        &self,
        node: &usvg::Node,
        opt: &Options,
    ) -> Option<Box<OutputImage>> {
        let img = render_node_to_image(node, opt)?;
        Some(Box::new(img))
    }

    fn calc_node_bbox(
        &self,
        node: &usvg::Node,
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
    tree: &usvg::Tree,
    opt: &Options,
) -> Option<cairo::ImageSurface> {
    let (surface, img_view) = create_surface(
        tree.svg_node().size.to_screen_size(),
        opt,
    )?;

    let cr = cairo::Context::new(&surface);

    // Fill background.
    if let Some(color) = opt.background {
        cr.set_source_color(color, 1.0.into());
        cr.paint();
    }

    render_to_canvas(tree, opt, img_view, &cr);

    Some(surface)
}

/// Renders SVG to image.
pub fn render_node_to_image(
    node: &usvg::Node,
    opt: &Options,
) -> Option<cairo::ImageSurface> {
    let node_bbox = if let Some(bbox) = calc_node_bbox(node, opt) {
        bbox
    } else {
        warn!("Node {:?} has zero size.", node.id());
        return None;
    };

    let (surface, img_size) = create_surface(node_bbox.to_screen_size(), opt)?;

    let vbox = usvg::ViewBox {
        rect: node_bbox,
        aspect: usvg::AspectRatio::default(),
    };

    let cr = cairo::Context::new(&surface);

    // Fill background.
    if let Some(color) = opt.background {
        cr.set_source_color(color, 1.0.into());
        cr.paint();
    }

    render_node_to_canvas(node, opt, vbox, img_size, &cr);

    Some(surface)
}

/// Renders SVG to canvas.
pub fn render_to_canvas(
    tree: &usvg::Tree,
    opt: &Options,
    img_size: ScreenSize,
    cr: &cairo::Context,
) {
    render_node_to_canvas(&tree.root(), opt, tree.svg_node().view_box,
                          img_size, cr);
}

/// Renders SVG node to canvas.
pub fn render_node_to_canvas(
    node: &usvg::Node,
    opt: &Options,
    view_box: usvg::ViewBox,
    img_size: ScreenSize,
    cr: &cairo::Context,
) {
    let mut layers = create_layers(img_size, opt);

    apply_viewbox_transform(view_box, img_size, &cr);

    let curr_ts = cr.get_matrix();
    let mut ts = utils::abs_transform(node);
    ts.append(&node.transform());

    cr.transform(ts.to_native());
    render_node(node, opt, &mut layers, cr);
    cr.set_matrix(curr_ts);
}

fn create_surface(
    size: ScreenSize,
    opt: &Options,
) -> Option<(cairo::ImageSurface, ScreenSize)> {
    let img_size = utils::fit_to(size, opt.fit_to);

    debug_assert_ne!(img_size.width, 0);
    debug_assert_ne!(img_size.height, 0);
    let surface = try_create_surface!(img_size, None);

    Some((surface, img_size))
}

/// Applies viewbox transformation to the painter.
fn apply_viewbox_transform(
    view_box: usvg::ViewBox,
    img_size: ScreenSize,
    cr: &cairo::Context,
) {
    let ts = utils::view_box_to_transform(view_box.rect, view_box.aspect, img_size.to_size());
    cr.transform(ts.to_native());
}

fn render_node(
    node: &usvg::Node,
    opt: &Options,
    layers: &mut CairoLayers,
    cr: &cairo::Context,
) -> Option<Rect> {
    match *node.borrow() {
        usvg::NodeKind::Svg(_) => {
            Some(render_group(node, opt, layers, cr))
        }
        usvg::NodeKind::Path(ref path) => {
            Some(path::draw(&node.tree(), path, opt, cr))
        }
        usvg::NodeKind::Text(ref text) => {
            Some(text::draw(&node.tree(), text, opt, cr))
        }
        usvg::NodeKind::Image(ref img) => {
            Some(image::draw(img, opt, cr))
        }
        usvg::NodeKind::Group(ref g) => {
            render_group_impl(node, g, opt, layers, cr)
        }
        _ => None,
    }
}

fn render_group(
    parent: &usvg::Node,
    opt: &Options,
    layers: &mut CairoLayers,
    cr: &cairo::Context,
) -> Rect {
    let curr_ts = cr.get_matrix();
    let mut g_bbox = Rect::new_bbox();

    for node in parent.children() {
        cr.transform(node.transform().to_native());

        let bbox = render_node(&node, opt, layers, cr);

        if let Some(bbox) = bbox {
            g_bbox.expand(bbox);
        }

        // Revert transform.
        cr.set_matrix(curr_ts);
    }

    g_bbox
}

fn render_group_impl(
    node: &usvg::Node,
    g: &usvg::Group,
    opt: &Options,
    layers: &mut CairoLayers,
    cr: &cairo::Context,
) -> Option<Rect> {
    let sub_surface = layers.get()?;
    let mut sub_surface = sub_surface.borrow_mut();

    let curr_ts = cr.get_matrix();

    let bbox = {
        let sub_cr = cairo::Context::new(&*sub_surface);
        sub_cr.set_matrix(curr_ts);

        render_group(node, opt, layers, &sub_cr)
    };

    if let Some(ref id) = g.filter {
        if let Some(filter_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::Filter(ref filter) = *filter_node.borrow() {
                let ts = usvg::Transform::from_native(&curr_ts);
                filter::apply(filter, bbox, &ts, &mut *sub_surface);
            }
        }
    }

    if let Some(ref id) = g.clip_path {
        if let Some(clip_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::ClipPath(ref cp) = *clip_node.borrow() {
                let sub_cr = cairo::Context::new(&*sub_surface);
                sub_cr.set_matrix(curr_ts);

                clippath::apply(&clip_node, cp, opt, bbox, layers, &sub_cr);
            }
        }
    }

    if let Some(ref id) = g.mask {
        if let Some(mask_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::Mask(ref mask) = *mask_node.borrow() {
                let sub_cr = cairo::Context::new(&*sub_surface);
                sub_cr.set_matrix(curr_ts);

                mask::apply(&mask_node, mask, opt, bbox, layers, &sub_cr);
            }
        }
    }

    let curr_matrix = cr.get_matrix();
    cr.set_matrix(cairo::Matrix::identity());
    cr.set_source_surface(&*sub_surface, 0.0, 0.0);
    if let Some(opacity) = g.opacity {
        cr.paint_with_alpha(*opacity);
    } else {
        cr.paint();
    }

    cr.set_matrix(curr_matrix);

    // All layers must be unlinked from the main context/cr after used.
    // TODO: find a way to automate this
    cr.reset_source_rgba();

    Some(bbox)
}

/// Calculates node's absolute bounding box.
///
/// Note: this method can be pretty expensive.
pub fn calc_node_bbox(
    node: &usvg::Node,
    opt: &Options,
) -> Option<Rect> {
    let tree = node.tree();

    // We can't use 1x1 image, like in Qt backend because otherwise
    // text layouts will be truncated.
    let (surface, img_view) = create_surface(
        tree.svg_node().size.to_screen_size(),
        opt,
    ).unwrap();
    let cr = cairo::Context::new(&surface);

    // We also have to apply the viewbox transform,
    // otherwise text hinting will be different and bbox will be different too.
    apply_viewbox_transform(tree.svg_node().view_box, img_view, &cr);

    let abs_ts = utils::abs_transform(node);
    _calc_node_bbox(node, opt, abs_ts, &cr)
}

fn _calc_node_bbox(
    node: &usvg::Node,
    opt: &Options,
    ts: usvg::Transform,
    cr: &cairo::Context,
) -> Option<Rect> {
    let mut ts2 = ts;
    ts2.append(&node.transform());

    match *node.borrow() {
        usvg::NodeKind::Path(ref path) => {
            Some(utils::path_bbox(&path.segments, path.stroke.as_ref(), &ts2))
        }
        usvg::NodeKind::Text(ref text) => {
            let mut bbox = Rect::new_bbox();
            let mut fm = text::PangoFontMetrics::new(opt, cr);
            let blocks = backend_utils::text::prepare_blocks(text, &mut fm);
            backend_utils::text::draw_blocks(blocks, |block| {
                cr.new_path();

                let context = text::init_pango_context(opt, cr);
                let layout = text::init_pango_layout(&block.text, &block.font, &context);

                pc::layout_path(cr, &layout);
                let path = cr.copy_path();
                let segments = from_cairo_path(&path);

                let mut t = ts2;
                if !block.rotate.is_fuzzy_zero() {
                    t.rotate_at(block.rotate, block.bbox.x, block.bbox.y + block.font_ascent);
                }
                t.translate(block.bbox.x, block.bbox.y);

                if !segments.is_empty() {
                    let c_bbox = utils::path_bbox(&segments, block.stroke.as_ref(), &t);
                    bbox.expand(c_bbox);
                }
            });

            Some(bbox)
        }
        usvg::NodeKind::Image(ref img) => {
            let segments = utils::rect_to_path(img.view_box.rect);
            Some(utils::path_bbox(&segments, None, &ts2))
        }
        usvg::NodeKind::Group(_) => {
            let mut bbox = Rect::new_bbox();

            for child in node.children() {
                if let Some(c_bbox) = _calc_node_bbox(&child, opt, ts2, cr) {
                    bbox.expand(c_bbox);
                }
            }

            Some(bbox)
        }
        _ => None
    }
}

fn from_cairo_path(path: &cairo::Path) -> Vec<usvg::PathSegment> {
    let mut segments = Vec::new();
    for seg in path.iter() {
        match seg {
            cairo::PathSegment::MoveTo((x, y)) => {
                segments.push(usvg::PathSegment::MoveTo { x, y });
            }
            cairo::PathSegment::LineTo((x, y)) => {
                segments.push(usvg::PathSegment::LineTo { x, y });
            }
            cairo::PathSegment::CurveTo((x1, y1), (x2, y2), (x, y)) => {
                segments.push(usvg::PathSegment::CurveTo { x1, y1, x2, y2, x, y });
            }
            cairo::PathSegment::ClosePath => {
                segments.push(usvg::PathSegment::ClosePath);
            }
        }
    }

    if segments.len() < 2 {
        segments.clear();
    }

    segments
}

fn create_layers(img_size: ScreenSize, opt: &Options) -> CairoLayers {
    layers::Layers::new(img_size, opt.usvg.dpi, create_subsurface, clear_subsurface)
}

fn create_subsurface(
    size: ScreenSize,
    _: f64,
) -> Option<cairo::ImageSurface> {
    Some(try_create_surface!(size, None))
}

fn clear_subsurface(surface: &mut cairo::ImageSurface) {
    let cr = cairo::Context::new(&surface);
    cr.set_operator(cairo::Operator::Clear);
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
    cr.paint();
}
