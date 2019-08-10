// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Qt backend implementation.

use crate::qt;
use log::warn;
use usvg::try_opt;

use crate::prelude::*;
use crate::layers::{self, Layer, Layers};
use crate::{FlatRender, ConvTransform, BlendMode};


macro_rules! try_create_image {
    ($size:expr, $ret:expr) => {
        usvg::try_opt_warn_or!(
            qt::Image::new_rgba_premultiplied($size.width(), $size.height()),
            $ret,
            "Failed to create a {}x{} image.", $size.width(), $size.height()
        );
    };
}


mod filter;
mod image;
mod path;
mod style;


impl ConvTransform<qt::Transform> for usvg::Transform {
    fn to_native(&self) -> qt::Transform {
        qt::Transform::new(self.a, self.b, self.c, self.d, self.e, self.f)
    }

    fn from_native(ts: &qt::Transform) -> Self {
        let d = ts.data();
        Self::new(d.0, d.1, d.2, d.3, d.4, d.5)
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

impl OutputImage for qt::Image {
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
) -> Option<qt::Image> {
    let (mut img, img_size) = create_root_image(tree.svg_node().size.to_screen_size(), opt)?;

    let mut painter = qt::Painter::new(&mut img);
    render_to_canvas(tree, opt, img_size, &mut painter);
    painter.end();

    Some(img)
}

/// Renders SVG node to image.
pub fn render_node_to_image(
    node: &usvg::Node,
    opt: &Options,
) -> Option<qt::Image> {
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

    let (mut img, img_size) = create_root_image(node_bbox.size().to_screen_size(), opt)?;

    let mut painter = qt::Painter::new(&mut img);
    render_node_to_canvas(node, opt, vbox, img_size, &mut painter);
    painter.end();

    Some(img)
}

/// Renders SVG to canvas.
pub fn render_to_canvas(
    tree: &usvg::Tree,
    opt: &Options,
    img_size: ScreenSize,
    painter: &mut qt::Painter,
) {
    render_node_to_canvas(&tree.root(), opt, tree.svg_node().view_box, img_size, painter);
}

/// Renders SVG node to canvas.
pub fn render_node_to_canvas(
    node: &usvg::Node,
    opt: &Options,
    view_box: usvg::ViewBox,
    img_size: ScreenSize,
    painter: &mut qt::Painter,
) {
    let tree = node.tree();
    let mut render = QtFlatRender::new(&tree, opt, img_size, painter);

    let mut ts = node.abs_transform();
    ts.append(&node.transform());

    render.apply_viewbox(view_box, img_size);
    render.apply_transform(ts);
    render.render_node(node);
}

fn create_root_image(
    size: ScreenSize,
    opt: &Options,
) -> Option<(qt::Image, ScreenSize)> {
    let img_size = utils::fit_to(size, opt.fit_to)?;

    let mut img = try_create_image!(img_size, None);

    // Fill background.
    if let Some(c) = opt.background {
        img.fill(c.red, c.green, c.blue, 255);
    } else {
        img.fill(0, 0, 0, 0);
    }

    Some((img, img_size))
}

impl Into<qt::CompositionMode> for BlendMode {
    fn into(self) -> qt::CompositionMode {
        match self {
            BlendMode::SourceOver => qt::CompositionMode::SourceOver,
            BlendMode::Clear => qt::CompositionMode::Clear,
            BlendMode::DestinationIn => qt::CompositionMode::DestinationIn,
            BlendMode::DestinationOut => qt::CompositionMode::DestinationOut,
            BlendMode::Xor => qt::CompositionMode::Xor,
        }
    }
}

impl layers::Image for qt::Image {
    fn new(size: ScreenSize) -> Option<Self> {
        let mut img = try_create_image!(size, None);
        img.fill(0, 0, 0, 0);
        Some(img)
    }

    fn clear(&mut self) {
        self.fill(0, 0, 0, 0);
    }
}

struct QtFlatRender<'a> {
    tree: &'a usvg::Tree,
    opt: &'a Options,
    painter: &'a mut qt::Painter,
    layers: Layers<qt::Image>,
}

impl<'a> QtFlatRender<'a> {
    fn new(
        tree: &'a usvg::Tree,
        opt: &'a Options,
        img_size: ScreenSize,
        painter: &'a mut qt::Painter,
    ) -> Self {
        QtFlatRender {
            tree,
            opt,
            painter,
            layers: Layers::new(img_size),
        }
    }

    fn new_painter(layer: &mut Layer<qt::Image>) -> qt::Painter {
        let mut p = qt::Painter::new(&mut layer.img);
        p.set_transform(&layer.ts.to_native());
        p.set_composition_mode(layer.blend_mode.into());

        if let Some(rect) = layer.clip_rect {
            p.set_clip_rect(rect.x(), rect.y(), rect.width(), rect.height());
        }

        p
    }

    fn paint<F>(&mut self, f: F)
        where F: FnOnce(&usvg::Tree, &Options, &mut qt::Painter)
    {
        match self.layers.current_mut() {
            Some(layer) => {
                let mut p = Self::new_painter(layer);
                f(self.tree, self.opt, &mut p);
            }
            None => {
                f(self.tree, self.opt, self.painter);
            }
        }
    }
}

impl<'a> FlatRender for QtFlatRender<'a> {
    fn draw_path(&mut self, path: &usvg::Path, bbox: Option<Rect>) {
        self.paint(|tree, opt, p| {
            path::draw(tree, path, opt, bbox, p);
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
            layer.img.fill(r, g, b, a);
        }
    }

    fn push_layer(&mut self) -> Option<()> {
        self.layers.push()
    }

    fn pop_layer(&mut self, opacity: usvg::Opacity, mode: BlendMode) {
        let last = try_opt!(self.layers.pop());
        match self.layers.current_mut() {
            Some(prev) => {
                let mut painter = qt::Painter::new(&mut prev.img);

                if !opacity.is_default() {
                    painter.set_opacity(opacity.value());
                }

                painter.set_composition_mode(mode.into());
                painter.draw_image(0.0, 0.0, &last.img);
            }
            None => {
                if !opacity.is_default() {
                    self.painter.set_opacity(opacity.value());
                }

                let curr_ts = self.painter.get_transform();
                self.reset_transform();
                self.painter.set_composition_mode(mode.into());
                self.painter.draw_image(0.0, 0.0, &last.img);

                // Reset.
                self.painter.set_opacity(1.0);
                self.painter.set_composition_mode(qt::CompositionMode::SourceOver);
                self.painter.set_transform(&curr_ts);
            }
        }

        self.layers.push_back(last);
    }

    fn apply_mask(&mut self) {
        let img_size = self.layers.img_size();
        if let Some(layer) = self.layers.current_mut() {
            crate::image_to_mask(&mut layer.img.data_mut(), img_size);
        }
    }

    fn set_composition_mode(&mut self, mode: BlendMode) {
        match self.layers.current_mut() {
            Some(layer) => layer.blend_mode = mode,
            None => self.painter.set_composition_mode(mode.into()),
        }
    }

    fn set_clip_rect(&mut self, rect: Rect) {
        match self.layers.current_mut() {
            Some(layer) => layer.clip_rect = Some(rect),
            None => self.painter.set_clip_rect(rect.x(), rect.y(), rect.width(), rect.height()),
        }
    }

    fn get_transform(&self) -> usvg::Transform {
        match self.layers.current() {
            Some(layer) => layer.ts,
            None => usvg::Transform::from_native(&self.painter.get_transform()),
        }
    }

    fn set_transform(&mut self, ts: usvg::Transform) {
        match self.layers.current_mut() {
            Some(layer) => layer.ts = ts,
            None => self.painter.set_transform(&ts.to_native()),
        }
    }

    fn finish(&mut self) {
        if self.layers.is_empty() {
            self.painter.end();
        }
    }
}
