// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Skia backend implementation.

use crate::skia;
use log::warn;
use usvg::try_opt;
use std::io::Write;

use crate::prelude::*;
use crate::layers::{self, Layers};
use crate::backend_utils::{self, FlatRender, ConvTransform, BlendMode};

macro_rules! try_create_surface {
    ($size:expr, $ret:expr) => {
        usvg::try_opt_warn_or!(
            skia::Surface::new_raster_n32_premul(($size.width() as i32, $size.height() as i32)),
            $ret,
            "Failed to create a {}x{} surface.", $size.width(), $size.height()
        );
    };
}

// mod filter;
mod image;
mod path;
mod style;

impl ConvTransform<skia::Matrix> for usvg::Transform {
    fn to_native(&self) -> skia::Matrix {
        skia::Matrix::new_all(self.a as f32, self.b as f32, self.c as f32, self.d as f32, self.e as f32, self.f as f32, 0.0, 0.0, 0.0)
    }

    fn from_native(mat: &skia::Matrix) -> Self {
        // let d = mat.data();
        Self::new(
            mat.scale_x().into(),
            mat.skew_x().into(),
            mat.translate_x().into(),
            mat.skew_y().into(),
            mat.scale_y().into(),
            mat.translate_y().into())
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
        let image = self.image_snapshot();
        let d = image.encode_to_data(skia::EncodedImageFormat::PNG).unwrap();
        let mut file = std::fs::File::create(path).unwrap();
        let bytes = d.as_bytes();
        if let Ok(_) = file.write_all(bytes) {
            return true;
        }
        false
    }
}

/// Renders SVG to image.
pub fn render_to_image(
    tree: &usvg::Tree,
    opt: &Options,
) -> Option<skia::Surface> {

    let (mut surface, img_size) = create_root_surface(tree.svg_node().size.to_screen_size(), opt)?;
    render_to_canvas(tree, opt, img_size, &mut surface);
    surface.flush();

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
    surface.flush();

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
    let canvas = surface.canvas();

    // Fill background.
    if let Some(c) = opt.background {
        let rgb = skia::RGB::from((c.red, c.green, c.blue));
        canvas.clear(skia::Color::from(rgb));
    } else {
        canvas.clear(skia::Color::WHITE);
    }

    Some((surface, img_size))
}

impl Into<skia::BlendMode> for BlendMode {
    fn into(self) -> skia::BlendMode {
        match self {
            BlendMode::SourceOver => skia::BlendMode::SrcOver,
            BlendMode::Clear => skia::BlendMode::Clear,
            BlendMode::DestinationIn => skia::BlendMode::DstIn,
            BlendMode::DestinationOut => skia::BlendMode::DstOut,
            BlendMode::Xor => skia::BlendMode::Xor,
        }
    }
}

impl layers::Image for skia::Surface {
    fn new(size: ScreenSize) -> Option<Self> {
        let mut surface = try_create_surface!(size, None);

        let canvas = surface.canvas();
        canvas.clear(skia::Color::WHITE);

        Some(surface)
    }

    fn clear(&mut self) {
        self.canvas().clear(skia::Color::WHITE);
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
                let mut canvas = layer.img.canvas();
                canvas.save();
                canvas.set_matrix(&layer.ts.to_native());

                if let Some(r) = layer.clip_rect {
                    let rect = skia::Rect::new(r.x() as f32, r.y() as f32, r.width() as f32, r.height() as f32);
                    canvas.clip_rect(&rect, None, true);
                }

                f(self.tree, self.opt, layer.blend_mode, &mut layer.img);

                canvas.restore();
            }
            None => {
                let mut canvas = self.surface.canvas();
                canvas.save();

                if let Some(r) = self.clip_rect {
                    let rect = skia::Rect::new(r.x() as f32, r.y() as f32, r.width() as f32, r.height() as f32);
                    canvas.clip_rect(&rect, None, true);
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
            // filter::apply(filter, bbox, &ts, &self.opt, &mut layer.img);
        }
    }

    fn fill_layer(&mut self, r: u8, g: u8, b: u8, a: u8) {
        if let Some(layer) = self.layers.current_mut() {
            let rgb = skia::RGB::from((r, g, b));
            layer.img.canvas().clear(skia::Color::from(rgb));
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
        let image = &last.img.image_snapshot();

        match self.layers.current_mut() {
            Some(prev) => {
                let mut canvas = prev.img.canvas();
                canvas.draw_image(&image, skia::Point::new(0.0, 0.0), None);
                // canvas.draw_surface(&last.img, 0.0, 0.0, a, mode.into(),
                //                     skia::FilterQuality::Low);
            }
            None => {
                let mut canvas = self.surface.canvas();

                let curr_ts = canvas.total_matrix();
                canvas.reset_matrix();
                canvas.draw_image(&image, skia::Point::new(0.0, 0.0), None);

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
            let image = layer.img.image_snapshot();
            let mut data = image.encoded_data().unwrap();
            backend_utils::image_to_mask(&mut data, img_size);
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
            None => {
                let rect = skia::Rect::new(rect.x() as f32, rect.y() as f32, rect.width() as f32, rect.height() as f32);
                self.surface.canvas().clip_rect(
                    rect,
                    None,
                    true,
                );
            }
        }
    }

    fn get_transform(&self) -> usvg::Transform {
        match self.layers.current() {
            Some(layer) => layer.ts,
            None => usvg::Transform::from_native(&self.surface.canvas().total_matrix()),
        }
    }

    fn set_transform(&mut self, ts: usvg::Transform) {
        match self.layers.current_mut() {
            Some(layer) => layer.ts = ts,
            None => { let _ = self.surface.canvas().set_matrix(&ts.to_native()); },
        }
    }

    fn finish(&mut self) {
        if self.layers.is_empty() {
            self.surface.canvas().flush();
        }
    }
}
