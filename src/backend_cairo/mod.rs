// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Cairo backend implementation.

// external
use cairo::{
    self,
    MatrixTrait,
};
use log::warn;

// self
use crate::prelude::*;
use crate::layers;
use crate::backend_utils::{
    ConvTransform,
};


macro_rules! try_create_surface {
    ($size:expr, $ret:expr) => {
        usvg::try_opt_warn_or!(
            cairo::ImageSurface::create(
                cairo::Format::ARgb32,
                $size.width() as i32,
                $size.height() as i32,
            ).ok(),
            $ret,
            "Failed to create a {}x{} surface.", $size.width(), $size.height()
        );
    };
}


mod clip_and_mask;
mod filter;
mod image;
mod path;
mod style;


type CairoLayers = layers::Layers<cairo::ImageSurface>;


impl ConvTransform<cairo::Matrix> for usvg::Transform {
    fn to_native(&self) -> cairo::Matrix {
        cairo::Matrix::new(self.a, self.b, self.c, self.d, self.e, self.f)
    }

    fn from_native(ts: &cairo::Matrix) -> Self {
        Self::new(ts.xx, ts.yx, ts.xy, ts.yy, ts.x0, ts.y0)
    }
}


pub(crate) trait ReCairoContextExt {
    fn set_source_color(&self, color: usvg::Color, opacity: usvg::Opacity);
    fn reset_source_rgba(&self);
}

impl ReCairoContextExt for cairo::Context {
    fn set_source_color(&self, color: usvg::Color, opacity: usvg::Opacity) {
        self.set_source_rgba(
            color.red as f64 / 255.0,
            color.green as f64 / 255.0,
            color.blue as f64 / 255.0,
            opacity.value(),
        );
    }

    fn reset_source_rgba(&self) {
        self.set_source_rgba(0.0, 0.0, 0.0, 0.0);
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
    fn save(
        &self,
        path: &std::path::Path,
    ) -> bool {
        use std::fs;

        if let Ok(mut buffer) = fs::File::create(path) {
            if self.write_to_png(&mut buffer).is_ok() {
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
        warn!("Node '{}' has a zero size.", node.id());
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
    render_node_to_canvas(&tree.root(), opt, tree.svg_node().view_box, img_size, cr);
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
    let img_size = utils::fit_to(size, opt.fit_to)?;

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
            path::draw(&node.tree(), path, opt, cr)
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
            let bbox = bbox.transform(&node.transform()).unwrap();
            g_bbox = g_bbox.expand(bbox);
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
                filter::apply(filter, bbox, &ts, opt, &mut *sub_surface);
            }
        }
    }

    if let Some(ref id) = g.clip_path {
        if let Some(clip_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::ClipPath(ref cp) = *clip_node.borrow() {
                let sub_cr = cairo::Context::new(&*sub_surface);
                sub_cr.set_matrix(curr_ts);

                clip_and_mask::clip(&clip_node, cp, opt, bbox, layers, &sub_cr);
            }
        }
    }

    if let Some(ref id) = g.mask {
        if let Some(mask_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::Mask(ref mask) = *mask_node.borrow() {
                let sub_cr = cairo::Context::new(&*sub_surface);
                sub_cr.set_matrix(curr_ts);

                clip_and_mask::mask(&mask_node, mask, opt, bbox, layers, &sub_cr);
            }
        }
    }

    let curr_matrix = cr.get_matrix();
    cr.set_matrix(cairo::Matrix::identity());
    cr.set_source_surface(&*sub_surface, 0.0, 0.0);
    if !g.opacity.is_default() {
        cr.paint_with_alpha(g.opacity.value());
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
    )?;
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
            utils::path_bbox(&path.segments, path.stroke.as_ref(), Some(ts2))
        }
        usvg::NodeKind::Image(ref img) => {
            let segments = utils::rect_to_path(img.view_box.rect);
            utils::path_bbox(&segments, None, Some(ts2))
        }
        usvg::NodeKind::Group(_) => {
            let mut bbox = Rect::new_bbox();

            for child in node.children() {
                if let Some(c_bbox) = _calc_node_bbox(&child, opt, ts2, cr) {
                    bbox = bbox.expand(c_bbox);
                }
            }

            Some(bbox)
        }
        _ => None
    }
}

fn create_layers(
    img_size: ScreenSize,
    opt: &Options,
) -> CairoLayers {
    layers::Layers::new(img_size, opt.usvg.dpi, create_subsurface, clear_subsurface)
}

fn create_subsurface(
    size: ScreenSize,
    _: f64,
) -> Option<cairo::ImageSurface> {
    Some(try_create_surface!(size, None))
}

fn clear_subsurface(
    surface: &mut cairo::ImageSurface,
) {
    let cr = cairo::Context::new(&surface);
    cr.set_operator(cairo::Operator::Clear);
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
    cr.paint();
}
