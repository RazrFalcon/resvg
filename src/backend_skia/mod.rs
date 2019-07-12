// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Skia backend implementation.

use crate::skia;
use usvg::try_opt;

use crate::prelude::*;
use crate::layers::{self, Layer, Layers};
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

    // TODO:  not implemented
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
    render_to_canvas(tree, opt, img_size, &mut surface.get_canvas());

    Some(surface)
}

/// Renders SVG node to image.
pub fn render_node_to_image(
    _node: &usvg::Node,
    _opt: &Options,
) -> Option<skia::Surface> {
    // TODO:  not implemented
    None
}

/// Renders SVG to canvas.
pub fn render_to_canvas(
    tree: &usvg::Tree,
    opt: &Options,
    img_size: ScreenSize,
    canvas: &mut skia::Canvas,
) {
    render_node_to_canvas(&tree.root(), opt, tree.svg_node().view_box, img_size, canvas);
}

/// Renders SVG node to canvas.
pub fn render_node_to_canvas(
    node: &usvg::Node,
    opt: &Options,
    view_box: usvg::ViewBox,
    img_size: ScreenSize,
    canvas: &mut skia::Canvas,
) {
    let tree = node.tree();
    let mut render = SkiaFlatRender::new(&tree, opt, img_size, canvas);

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
    let canvas = surface.get_canvas();

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

        let canvas = surface.get_canvas();
        canvas.clear();

        Some(surface)
    }

    fn clear(&mut self) {
        self.get_canvas().clear();
    }
}

struct SkiaFlatRender<'a> {
    tree: &'a usvg::Tree,
    opt: &'a Options,
    blend_mode: BlendMode,
    canvas: &'a mut skia::Canvas,
    layers: Layers<skia::Surface>,
}

impl<'a> SkiaFlatRender<'a> {
    fn new(
        tree: &'a usvg::Tree,
        opt: &'a Options,
        img_size: ScreenSize,
        painter: &'a mut skia::Canvas,
    ) -> Self {
        SkiaFlatRender {
            tree,
            opt,
            blend_mode: BlendMode::default(),
            canvas: painter,
            layers: Layers::new(img_size),
        }
    }

    fn new_painter(layer: &mut Layer<skia::Surface>) -> skia::Canvas {
        let mut p = layer.img.get_canvas();
        p.set_matrix(&layer.ts.to_native());
//        p.set_composition_mode(layer.blend_mode.into());

        if let Some(rect) = layer.clip_rect {
            p.clip_rect(rect.x(), rect.y(), rect.width(), rect.height());
        }

        p
    }

    fn paint<F>(&mut self, f: F)
        where F: FnOnce(&usvg::Tree, &Options, &mut skia::Canvas)
    {
        match self.layers.current_mut() {
            Some(layer) => {
                let mut p = Self::new_painter(layer);
                f(self.tree, self.opt, &mut p);
            }
            None => {
                f(self.tree, self.opt, self.canvas);
            }
        }
    }
}

impl<'a> FlatRender for SkiaFlatRender<'a> {
    fn draw_path(&mut self, path: &usvg::Path, bbox: Option<Rect>) {
        self.paint(|tree, opt, p| {
            path::draw(tree, path, opt, bbox, p, skia::BlendMode::SourceOver);
        });
    }

    fn draw_svg_image(&mut self, image: &usvg::Image) {
        self.paint(|_, opt, p| {
            image::draw_svg(&image.data, image.view_box, opt, p);
        });
    }

    fn draw_raster_image(&mut self, image: &usvg::Image) {
        self.paint(|_, opt, p| {
            image::draw_raster(
                image.format, &image.data, image.view_box, image.rendering_mode, opt, p,
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
            layer.img.get_canvas().fill(r, g, b, a);
        }
    }

    fn push_layer(&mut self) -> Option<()> {
        self.layers.push()
    }

    fn pop_layer(&mut self, opacity: usvg::Opacity, mode: BlendMode) {
        let last = try_opt!(self.layers.pop());
        match self.layers.current_mut() {
            Some(prev) => {
                let mut painter = prev.img.get_canvas();

//                if !opacity.is_default() {
//                    painter.set_opacity(opacity.value());
//                }

//                painter.set_composition_mode(mode.into());
//                painter.draw_image(0.0, 0.0, &last.img);
                painter.draw_surface(&last.img, 0.0, 0.0, 255, skia::BlendMode::DestinationOut);
            }
            None => {
//                if !opacity.is_default() {
//                    self.painter.set_opacity(opacity.value());
//                }

                let curr_ts = self.canvas.get_total_matrix();
                self.reset_transform();
//                self.painter.set_composition_mode(mode.into());
//                self.painter.draw_image(0.0, 0.0, &last.img);
                self.canvas.draw_surface(&last.img, 0.0, 0.0, 255, skia::BlendMode::DestinationOut);

                // Reset.
//                self.painter.set_opacity(1.0);
//                self.painter.set_composition_mode(qt::CompositionMode::SourceOver);
                self.canvas.set_matrix(&curr_ts);
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
            None => self.canvas.clip_rect(rect.x(), rect.y(), rect.width(), rect.height()),
        }
    }

    fn get_transform(&self) -> usvg::Transform {
        match self.layers.current() {
            Some(layer) => layer.ts,
            None => usvg::Transform::from_native(&self.canvas.get_total_matrix()),
        }
    }

    fn set_transform(&mut self, ts: usvg::Transform) {
        match self.layers.current_mut() {
            Some(layer) => layer.ts = ts,
            None => self.canvas.set_matrix(&ts.to_native()),
        }
    }

    fn finish(&mut self) {
        if self.layers.is_empty() {
            self.canvas.flush();
        }
    }
}
