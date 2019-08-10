// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Cairo backend implementation.

use usvg::try_opt;
use log::warn;

use crate::prelude::*;
use crate::layers::{self, Layer, Layers};
use crate::{FlatRender, ConvTransform, BlendMode};


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


mod filter;
mod image;
mod path;
mod style;


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
    ) -> Option<Box<dyn OutputImage>> {
        let img = render_to_image(tree, opt)?;
        Some(Box::new(img))
    }

    fn render_node_to_image(
        &self,
        node: &usvg::Node,
        opt: &Options,
    ) -> Option<Box<dyn OutputImage>> {
        let img = render_node_to_image(node, opt)?;
        Some(Box::new(img))
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
    let node_bbox = if let Some(bbox) = node.calculate_bbox() {
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
    let tree = node.tree();
    let mut render = CairoFlatRender::new(&tree, opt, img_size, cr);

    let mut ts = node.abs_transform();
    ts.append(&node.transform());

    render.apply_viewbox(view_box, img_size);
    render.apply_transform(ts);
    render.render_node(node);
}

fn create_surface(
    size: ScreenSize,
    opt: &Options,
) -> Option<(cairo::ImageSurface, ScreenSize)> {
    let img_size = utils::fit_to(size, opt.fit_to)?;

    let surface = try_create_surface!(img_size, None);

    Some((surface, img_size))
}

impl Into<cairo::Operator> for BlendMode {
    fn into(self) -> cairo::Operator {
        match self {
            BlendMode::SourceOver => cairo::Operator::Over,
            BlendMode::Clear => cairo::Operator::Clear,
            BlendMode::DestinationIn => cairo::Operator::DestIn,
            BlendMode::DestinationOut => cairo::Operator::DestOut,
            BlendMode::Xor => cairo::Operator::Xor,
        }
    }
}

impl layers::Image for cairo::ImageSurface {
    fn new(size: ScreenSize) -> Option<Self> {
        Some(try_create_surface!(size, None))
    }

    fn clear(&mut self) {
        let cr = cairo::Context::new(self);
        cr.set_operator(cairo::Operator::Clear);
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        cr.paint();
    }
}

struct CairoFlatRender<'a> {
    tree: &'a usvg::Tree,
    opt: &'a Options,
    cr: &'a cairo::Context,
    layers: Layers<cairo::ImageSurface>,
}

impl<'a> CairoFlatRender<'a> {
    fn new(
        tree: &'a usvg::Tree,
        opt: &'a Options,
        img_size: ScreenSize,
        cr: &'a cairo::Context,
    ) -> Self {
        CairoFlatRender {
            tree,
            opt,
            cr,
            layers: Layers::new(img_size),
        }
    }

    fn new_painter(layer: &mut Layer<cairo::ImageSurface>) -> cairo::Context {
        let cr = cairo::Context::new(&mut layer.img);
        cr.set_matrix(layer.ts.to_native());
        cr.set_operator(layer.blend_mode.into());

        if let Some(rect) = layer.clip_rect {
            cr.rectangle(rect.x(), rect.y(), rect.width(), rect.height());
            cr.clip();
        }

        cr
    }

    fn paint<F>(&mut self, f: F)
        where F: FnOnce(&usvg::Tree, &Options, &cairo::Context)
    {
        match self.layers.current_mut() {
            Some(layer) => {
                let cr = Self::new_painter(layer);
                f(self.tree, self.opt, &cr);
            }
            None => {
                f(self.tree, self.opt, self.cr);
            }
        }
    }
}

impl<'a> FlatRender for CairoFlatRender<'a> {
    fn draw_path(&mut self, path: &usvg::Path, bbox: Option<Rect>) {
        self.paint(|tree, opt, cr| {
            path::draw(tree, path, opt, bbox, cr);
        });
    }

    fn draw_svg_image(&mut self, image: &usvg::Image) {
        self.paint(|_, opt, cr| {
            image::draw_svg(&image.data, image.view_box, opt, cr);
        });
    }

    fn draw_raster_image(&mut self, image: &usvg::Image) {
        self.paint(|_, opt, cr| {
            image::draw_raster(
                image.format, &image.data, image.view_box, image.rendering_mode, opt, cr,
            );
        });
    }

    fn filter(&mut self, filter: &usvg::Filter, bbox: Option<Rect>, ts: usvg::Transform) {
        if let Some(layer) = self.layers.current_mut() {
            filter::apply(filter, bbox, &ts, &self.opt, &mut layer.img);
        }
    }

    fn fill_layer(&mut self, r: u8, g: u8, b: u8, a: u8) {
        if let Some(layer) = self.layers.current_mut() {
            let clip_cr = cairo::Context::new(&layer.img);
            clip_cr.set_source_rgba(
                r as f64 / 255.0,
                g as f64 / 255.0,
                b as f64 / 255.0,
                a as f64 / 255.0,
            );
            clip_cr.paint();
        }
    }

    fn push_layer(&mut self) -> Option<()> {
        self.layers.push()
    }

    fn pop_layer(&mut self, opacity: usvg::Opacity, mode: BlendMode) {
        let last = try_opt!(self.layers.pop());
        match self.layers.current_mut() {
            Some(prev) => {
                let cr = cairo::Context::new(&prev.img);
                cr.set_source_surface(&last.img, 0.0, 0.0);
                cr.set_operator(mode.into());
                if !opacity.is_default() {
                    cr.paint_with_alpha(opacity.value());
                } else {
                    cr.paint();
                }
            }
            None => {
                let curr_matrix = self.cr.get_matrix();
                self.reset_transform();
                self.cr.set_source_surface(&last.img, 0.0, 0.0);
                self.cr.set_operator(mode.into());
                if !opacity.is_default() {
                    self.cr.paint_with_alpha(opacity.value());
                } else {
                    self.cr.paint();
                }

                self.cr.set_matrix(curr_matrix);
                self.cr.set_operator(cairo::Operator::Over);

                // All layers must be unlinked from the main context/cr after used.
                self.cr.reset_source_rgba();
            }
        }

        self.layers.push_back(last);
    }

    fn apply_mask(&mut self) {
        let img_size = self.layers.img_size();
        if let Some(layer) = self.layers.current_mut() {
            let mut data = try_opt!(layer.img.get_data().ok());
            crate::image_to_mask(&mut data, img_size);
        }
    }

    fn set_composition_mode(&mut self, mode: BlendMode) {
        match self.layers.current_mut() {
            Some(layer) => layer.blend_mode = mode,
            None => self.cr.set_operator(mode.into()),
        }
    }

    fn set_clip_rect(&mut self, rect: Rect) {
        match self.layers.current_mut() {
            Some(layer) => layer.clip_rect = Some(rect),
            None => {
                self.cr.rectangle(rect.x(), rect.y(), rect.width(), rect.height());
                self.cr.clip();
            }
        }
    }

    fn get_transform(&self) -> usvg::Transform {
        match self.layers.current() {
            Some(layer) => layer.ts,
            None => usvg::Transform::from_native(&self.cr.get_matrix()),
        }
    }

    fn set_transform(&mut self, ts: usvg::Transform) {
        match self.layers.current_mut() {
            Some(layer) => layer.ts = ts,
            None => self.cr.set_matrix(ts.to_native()),
        }
    }
}
