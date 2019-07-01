// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Raqote backend implementation.

use usvg::try_opt;
use log::warn;

use crate::prelude::*;
use crate::layers::{self, Layers};
use crate::backend_utils::{self, FlatRender, ConvTransform, BlendMode};

mod filter;
mod image;
mod path;
mod style;


impl ConvTransform<raqote::Transform> for usvg::Transform {
    fn to_native(&self) -> raqote::Transform {
        raqote::Transform::row_major(self.a as f32, self.b as f32, self.c as f32,
                                     self.d as f32, self.e as f32, self.f as f32)
    }

    fn from_native(ts: &raqote::Transform) -> Self {
        Self::new(ts.m11 as f64, ts.m12 as f64, ts.m21 as f64,
                  ts.m22 as f64, ts.m31 as f64, ts.m32 as f64)
    }
}


pub(crate) trait RaqoteDrawTargetExt {
    fn transform(&mut self, ts: &raqote::Transform);
    fn as_image(&self) -> raqote::Image;
    fn make_transparent(&mut self);
}

impl RaqoteDrawTargetExt for raqote::DrawTarget {
    fn transform(&mut self, ts: &raqote::Transform) {
        self.set_transform(&self.get_transform().pre_mul(ts));
    }

    fn as_image(&self) -> raqote::Image {
        raqote::Image {
            width: self.width() as i32,
            height: self.height() as i32,
            data: self.get_data(),
        }
    }

    fn make_transparent(&mut self) {
        // This is faster than DrawTarget::clear.
        for i in self.get_data_u8_mut() {
            *i = 0;
        }
    }
}

pub(crate) trait ColorExt {
    fn to_solid(&self, a: u8) -> raqote::SolidSource;
    fn to_u32(&self, a: u8) -> u32;
}

impl ColorExt for usvg::Color {
    fn to_solid(&self, a: u8) -> raqote::SolidSource {
        raqote::SolidSource {
            r: premultiply(self.red, a),
            g: premultiply(self.green, a),
            b: premultiply(self.blue, a),
            a,
        }
    }

    fn to_u32(&self, a: u8) -> u32 {
        let r = self.red as u32;
        let g = self.green as u32;
        let b = self.blue as u32;

        ((a as u32 & 0xff) << 24) | ((r & 0xff) << 16) | ((g & 0xff) << 8) | (b & 0xff)
    }
}

fn premultiply(c: u8, a: u8) -> u8 {
    let c = a as u32 * c as u32 + 0x80;
    (((c >> 8) + c) >> 8) as u8
}


/// Raqote backend handle.
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

impl OutputImage for raqote::DrawTarget {
    fn save(&self, path: &::std::path::Path) -> bool {
        self.write_png(path).is_ok()
    }
}


/// Renders SVG to image.
pub fn render_to_image(
    tree: &usvg::Tree,
    opt: &Options,
) -> Option<raqote::DrawTarget> {
    let (mut dt, img_view) = create_target(
        tree.svg_node().size.to_screen_size(),
        opt,
    )?;

    // Fill background.
    if let Some(c) = opt.background {
        dt.clear(raqote::SolidSource { r: c.red, g: c.green, b: c.blue, a: 255 });
    }

    render_to_canvas(tree, opt, img_view, &mut dt);

    Some(dt)
}

/// Renders SVG to image.
pub fn render_node_to_image(
    node: &usvg::Node,
    opt: &Options,
) -> Option<raqote::DrawTarget> {
    let node_bbox = if let Some(bbox) = node.calculate_bbox() {
        bbox
    } else {
        warn!("Node '{}' has a zero size.", node.id());
        return None;
    };

    let (mut dt, img_size) = create_target(node_bbox.to_screen_size(), opt)?;

    let vbox = usvg::ViewBox {
        rect: node_bbox,
        aspect: usvg::AspectRatio::default(),
    };

    // Fill background.
    if let Some(c) = opt.background {
        dt.clear(raqote::SolidSource { r: c.red, g: c.green, b: c.blue, a: 255 });
    }

    render_node_to_canvas(node, opt, vbox, img_size, &mut dt);

    Some(dt)
}

/// Renders SVG to canvas.
pub fn render_to_canvas(
    tree: &usvg::Tree,
    opt: &Options,
    img_size: ScreenSize,
    dt: &mut raqote::DrawTarget,
) {
    render_node_to_canvas(&tree.root(), opt, tree.svg_node().view_box, img_size, dt);
}

/// Renders SVG node to canvas.
pub fn render_node_to_canvas(
    node: &usvg::Node,
    opt: &Options,
    view_box: usvg::ViewBox,
    img_size: ScreenSize,
    dt: &mut raqote::DrawTarget,
) {
    let tree = node.tree();
    let mut render = RaqoteFlatRender::new(&tree, opt, img_size, dt);

    let mut ts = node.abs_transform();
    ts.append(&node.transform());

    render.apply_viewbox(view_box, img_size);
    render.apply_transform(ts);
    render.render_node(node);
}

fn create_target(
    size: ScreenSize,
    opt: &Options,
) -> Option<(raqote::DrawTarget, ScreenSize)> {
    let img_size = utils::fit_to(size, opt.fit_to)?;

    let dt = raqote::DrawTarget::new(img_size.width() as i32, img_size.height() as i32);

    Some((dt, img_size))
}

impl Into<raqote::BlendMode> for BlendMode {
    fn into(self) -> raqote::BlendMode {
        match self {
            BlendMode::SourceOver => raqote::BlendMode::SrcOver,
            BlendMode::Clear => raqote::BlendMode::Clear,
            BlendMode::DestinationIn => raqote::BlendMode::DstIn,
            BlendMode::DestinationOut => raqote::BlendMode::DstOut,
            BlendMode::Xor => raqote::BlendMode::Xor,
        }
    }
}

impl layers::Image for raqote::DrawTarget {
    fn new(size: ScreenSize) -> Option<Self> {
        Some(raqote::DrawTarget::new(size.width() as i32, size.height() as i32))
    }

    fn clear(&mut self) {
        self.make_transparent();
    }
}

struct RaqoteFlatRender<'a> {
    tree: &'a usvg::Tree,
    opt: &'a Options,
    dt: &'a mut raqote::DrawTarget,
    blend_mode: BlendMode,
    clip_rect: Option<Rect>,
    layers: Layers<raqote::DrawTarget>,
}

impl<'a> RaqoteFlatRender<'a> {
    fn new(
        tree: &'a usvg::Tree,
        opt: &'a Options,
        img_size: ScreenSize,
        dt: &'a mut raqote::DrawTarget,
    ) -> Self {
        RaqoteFlatRender {
            tree,
            opt,
            dt,
            blend_mode: BlendMode::default(),
            clip_rect: None,
            layers: Layers::new(img_size),
        }
    }

    fn paint<F>(&mut self, f: F)
        where F: FnOnce(&usvg::Tree, &Options, raqote::DrawOptions, &mut raqote::DrawTarget)
    {
        match self.layers.current_mut() {
            Some(layer) => {
                let dt = &mut layer.img;
                dt.set_transform(&layer.ts.to_native());

                if let Some(r) = layer.clip_rect {
                    let mut pb = raqote::PathBuilder::new();
                    pb.rect(r.x() as f32, r.y() as f32, r.width() as f32, r.height() as f32);
                    dt.push_clip(&pb.finish());
                }

                let mut draw_opt = raqote::DrawOptions::default();
                draw_opt.blend_mode = layer.blend_mode.into();

                f(self.tree, self.opt, draw_opt, dt);

                dt.set_transform(&raqote::Transform::default());

                if layer.clip_rect.is_some() {
                    dt.pop_clip();
                }
            }
            None => {
                if let Some(r) = self.clip_rect {
                    let mut pb = raqote::PathBuilder::new();
                    pb.rect(r.x() as f32, r.y() as f32, r.width() as f32, r.height() as f32);
                    self.dt.push_clip(&pb.finish());
                }

                let mut draw_opt = raqote::DrawOptions::default();
                draw_opt.blend_mode = self.blend_mode.into();

                f(self.tree, self.opt, draw_opt, self.dt);

                if self.clip_rect.is_some() {
                    self.dt.pop_clip();
                }
            }
        }
    }
}

impl<'a> FlatRender for RaqoteFlatRender<'a> {
    fn draw_path(&mut self, path: &usvg::Path, bbox: Option<Rect>) {
        self.paint(|tree, opt, draw_opt, dt| {
            path::draw(tree, path, opt, draw_opt, bbox, dt);
        });
    }

    fn draw_svg_image(&mut self, data: &usvg::ImageData, view_box: usvg::ViewBox) {
        self.paint(|_, opt, _, dt| {
            image::draw_svg(data, view_box, opt, dt);
        });
    }

    fn draw_raster_image(
        &mut self,
        data: &usvg::ImageData,
        view_box: usvg::ViewBox,
        rendering_mode: usvg::ImageRendering,
    ) {
        self.paint(|_, opt, _, dt| {
            image::draw_raster(data, view_box, rendering_mode, opt, dt);
        });
    }

    fn filter(&mut self, filter: &usvg::Filter, bbox: Option<Rect>, ts: usvg::Transform) {
        if let Some(layer) = self.layers.current_mut() {
            filter::apply(filter, bbox, &ts, &self.opt, &mut layer.img);
        }
    }

    fn fill_layer(&mut self, r: u8, g: u8, b: u8, a: u8) {
        if let Some(layer) = self.layers.current_mut() {
            layer.img.clear(raqote::SolidSource { r, g, b, a });
        }
    }

    fn push_layer(&mut self) -> Option<()> {
        self.layers.push()
    }

    fn pop_layer(&mut self, opacity: usvg::Opacity, mode: BlendMode) {
        let last = try_opt!(self.layers.pop());
        match self.layers.current_mut() {
            Some(prev) => {
                prev.img.draw_image_at(0.0, 0.0, &last.img.as_image(), &raqote::DrawOptions {
                    blend_mode: mode.into(),
                    alpha: opacity.value() as f32,
                    antialias: raqote::AntialiasMode::Gray,
                });
            }
            None => {
                let curr_ts = *self.dt.get_transform();
                self.reset_transform();

                self.dt.draw_image_at(0.0, 0.0, &last.img.as_image(), &raqote::DrawOptions {
                    blend_mode: mode.into(),
                    alpha: opacity.value() as f32,
                    antialias: raqote::AntialiasMode::Gray,
                });

                self.dt.set_transform(&curr_ts);
                self.blend_mode = BlendMode::default();
                self.clip_rect = None;
            }
        }

        self.layers.push_back(last);
    }

    fn apply_mask(&mut self) {
        let img_size = self.layers.img_size();
        if let Some(layer) = self.layers.current_mut() {
            backend_utils::image_to_mask(layer.img.get_data_u8_mut(), img_size);
        }
    }

    fn set_composition_mode(&mut self, mode: BlendMode) {
        match self.layers.current_mut() {
            Some(layer) => layer.blend_mode = mode,
            None => self.blend_mode = mode.into(),
        }
    }

    fn set_clip_rect(&mut self, rect: Rect) {
        match self.layers.current_mut() {
            Some(layer) => layer.clip_rect = Some(rect),
            None => self.clip_rect = Some(rect),
        }
    }

    fn get_transform(&self) -> usvg::Transform {
        match self.layers.current() {
            Some(layer) => layer.ts,
            None => usvg::Transform::from_native(self.dt.get_transform()),
        }
    }

    fn set_transform(&mut self, ts: usvg::Transform) {
        match self.layers.current_mut() {
            Some(layer) => layer.ts = ts,
            None => self.dt.set_transform(&ts.to_native()),
        }
    }
}
