// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Skia backend implementation.

use crate::skia;
use log::warn;
use usvg::try_opt;

use crate::prelude::*;
use crate::layers::{self, Layers};
use crate::backend_utils::{self, FlatRender, ConvTransform, BlendMode};

macro_rules! try_create_surface {
    ($size:expr, $ret:expr) => {
        usvg::try_opt_warn_or!(
            skia::Surface::new_rgba_premultiplied($size.width(), $size.height()),
            $ret,
            "Failed to create a {}x{} surface.", $size.width(), $size.height()
        );
    };
}

mod filter;
mod image;
mod path;
mod style;

impl ConvTransform<skia::Matrix> for usvg::Transform {
    fn to_native(&self) -> skia::Matrix {
        skia::Matrix::new_from(self.a, self.b, self.c, self.d, self.e, self.f)
    }

    fn from_native(mat: &skia::Matrix) -> Self {
        let d = mat.data();
        Self::new(d.0, d.1, d.2, d.3, d.4, d.5)
    }
}


/// Skia backend handle.
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
}

impl OutputImage for skia::Surface {
    fn save(
        &self,
        path: &std::path::Path,
    ) -> bool {
        self.save(path.to_str().unwrap())
    }
}

/// Renders SVG to image.
pub fn render_to_image(
    tree: &usvg::Tree,
    opt: &Options,
) -> Option<skia::Surface> {

    let (mut surface, img_size) = create_root_surface(tree.svg_node().size.to_screen_size(), opt)?;
    render_to_canvas(tree, opt, img_size, &mut surface);
    surface.canvas_mut().flush();

    Some(surface)
}

/// Renders SVG node to image.
pub fn render_node_to_image(
    node: &usvg::Node,
    opt: &Options,
) -> Option<skia::Surface> {
    let node_bbox = if let Some(bbox) = node.calculate_bbox() {
        bbox
    } else {
        warn!("Node '{}' has zero size.", node.id());
        return None;
    };

    let vbox = usvg::ViewBox {
        rect: node_bbox,
        aspect: usvg::AspectRatio::default(),
    };

    let (mut surface, img_size) = create_root_surface(node_bbox.size().to_screen_size(), opt)?;
    render_node_to_canvas(node, opt, vbox, img_size, &mut surface);
    surface.canvas_mut().flush();

    Some(surface)
}

/// Renders SVG to canvas.
pub fn render_to_canvas(
    tree: &usvg::Tree,
    opt: &Options,
    img_size: ScreenSize,
    surface: &mut skia::Surface,
) {
    render_node_to_canvas(&tree.root(), opt, tree.svg_node().view_box, img_size, surface);
}

/// Renders SVG node to canvas.
pub fn render_node_to_canvas(
    node: &usvg::Node,
    opt: &Options,
    view_box: usvg::ViewBox,
    img_size: ScreenSize,
    surface: &mut skia::Surface,
) {
    let tree = node.tree();
    let mut render = SkiaFlatRender::new(&tree, opt, img_size, surface);

    let mut ts = node.abs_transform();
    ts.append(&node.transform());

    render.apply_viewbox(view_box, img_size);
    render.apply_transform(ts);
    render.render_node(node);
}

fn create_root_surface(
    size: ScreenSize,
    opt: &Options,
) -> Option<(skia::Surface, ScreenSize)> {
    let img_size = utils::fit_to(size, opt.fit_to)?;

    let mut surface = try_create_surface!(img_size, None);
    let canvas = surface.canvas_mut();

    // Fill background.
    if let Some(c) = opt.background {
        canvas.fill(c.red, c.green, c.blue, 255);
    } else {
        canvas.clear();
    }

    Some((surface, img_size))
}

impl Into<skia::BlendMode> for BlendMode {
    fn into(self) -> skia::BlendMode {
        match self {
            BlendMode::SourceOver => skia::BlendMode::SourceOver,
            BlendMode::Clear => skia::BlendMode::Clear,
            BlendMode::DestinationIn => skia::BlendMode::DestinationIn,
            BlendMode::DestinationOut => skia::BlendMode::DestinationOut,
            BlendMode::Xor => skia::BlendMode::Xor,
        }
    }
}

impl layers::Image for skia::Surface {
    fn new(size: ScreenSize) -> Option<Self> {
        let mut surface = try_create_surface!(size, None);

        let canvas = surface.canvas_mut();
        canvas.clear();

        Some(surface)
    }

    fn clear(&mut self) {
        self.canvas_mut().clear();
    }
}

struct SkiaFlatRender<'a> {
    tree: &'a usvg::Tree,
    opt: &'a Options,
    blend_mode: BlendMode,
    clip_rect: Option<Rect>,
    surface: &'a mut skia::Surface,
    layers: Layers<skia::Surface>,
}

impl<'a> SkiaFlatRender<'a> {
    fn new(
        tree: &'a usvg::Tree,
        opt: &'a Options,
        img_size: ScreenSize,
        surface: &'a mut skia::Surface,
    ) -> Self {
        SkiaFlatRender {
            tree,
            opt,
            blend_mode: BlendMode::default(),
            clip_rect: None,
            surface,
            layers: Layers::new(img_size),
        }
    }

    fn paint<F>(&mut self, f: F)
        where F: FnOnce(&usvg::Tree, &Options, BlendMode, &mut skia::Surface)
    {
        match self.layers.current_mut() {
            Some(layer) => {
                let mut canvas = layer.img.canvas_mut();
                canvas.save();
                canvas.set_matrix(&layer.ts.to_native());

                if let Some(r) = layer.clip_rect {
                    canvas.set_clip_rect(r.x(), r.y(), r.width(), r.height());
                }

                f(self.tree, self.opt, layer.blend_mode, &mut layer.img);

                canvas.restore();
            }
            None => {
                let mut canvas = self.surface.canvas_mut();
                canvas.save();

                if let Some(r) = self.clip_rect {
                    canvas.set_clip_rect(r.x(), r.y(), r.width(), r.height());
                }

                f(self.tree, self.opt, self.blend_mode, self.surface);

                canvas.restore();
            }
        }
    }
}

impl<'a> FlatRender for SkiaFlatRender<'a> {
    fn draw_path(&mut self, path: &usvg::Path, bbox: Option<Rect>) {
        self.paint(|tree, opt, blend_mode, surface| {
            path::draw(tree, path, opt, bbox, surface, blend_mode.into());
        });
    }

    fn draw_svg_image(&mut self, image: &usvg::Image) {
        self.paint(|_, opt, _, surface| {
            image::draw_svg(&image.data, image.view_box, opt, surface);
        });
    }

    fn draw_raster_image(&mut self, image: &usvg::Image) {
        self.paint(|_, opt, _, surface| {
            image::draw_raster(
                image.format, &image.data, image.view_box, image.rendering_mode, opt, surface,
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
            layer.img.canvas_mut().fill(r, g, b, a);
        }
    }

    fn push_layer(&mut self) -> Option<()> {
        self.layers.push()
    }

    fn pop_layer(&mut self, opacity: usvg::Opacity, mode: BlendMode) {
        let a = if !opacity.is_default() {
            (opacity.value() * 255.0) as u8
        } else {
            255
        };

        let last = try_opt!(self.layers.pop());
        match self.layers.current_mut() {
            Some(prev) => {
                let mut canvas = prev.img.canvas_mut();
                canvas.draw_surface(&last.img, 0.0, 0.0, a, mode.into());
            }
            None => {
                let mut canvas = self.surface.canvas_mut();

                let curr_ts = canvas.get_matrix();
                canvas.reset_matrix();
                canvas.draw_surface(&last.img, 0.0, 0.0, a, mode.into());

                // Reset.
                canvas.set_matrix(&curr_ts);
                self.blend_mode = BlendMode::default();
            }
        }

        self.layers.push_back(last);
    }

    fn apply_mask(&mut self) {
        let img_size = self.layers.img_size();
        if let Some(layer) = self.layers.current_mut() {
            backend_utils::image_to_mask(&mut layer.img.data_mut(), img_size);
        }
    }

    fn set_composition_mode(&mut self, mode: BlendMode) {
        match self.layers.current_mut() {
            Some(layer) => layer.blend_mode = mode,
            None => self.blend_mode = mode,
        }
    }

    fn set_clip_rect(&mut self, rect: Rect) {
        match self.layers.current_mut() {
            Some(layer) => layer.clip_rect = Some(rect),
            None => self.surface.canvas_mut().set_clip_rect(rect.x(), rect.y(), rect.width(), rect.height()),
        }
    }

    fn get_transform(&self) -> usvg::Transform {
        match self.layers.current() {
            Some(layer) => layer.ts,
            None => usvg::Transform::from_native(&self.surface.canvas().get_matrix()),
        }
    }

    fn set_transform(&mut self, ts: usvg::Transform) {
        match self.layers.current_mut() {
            Some(layer) => layer.ts = ts,
            None => self.surface.canvas_mut().set_matrix(&ts.to_native()),
        }
    }

    fn finish(&mut self) {
        if self.layers.is_empty() {
            self.surface.canvas_mut().flush();
        }
    }
}
