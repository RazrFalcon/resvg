// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Raqote backend implementation.

use log::warn;

use crate::{prelude::*, layers, backend_utils::ConvTransform};

mod clip_and_mask;
mod filter;
mod image;
mod path;
mod style;


type RaqoteLayers = layers::Layers<raqote::DrawTarget>;


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
}

impl RaqoteDrawTargetExt for raqote::DrawTarget {
    fn transform(&mut self, ts: &raqote::Transform) {
        self.set_transform(&self.get_transform().pre_mul(ts));
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

    fn calc_node_bbox(
        &self,
        node: &usvg::Node,
        opt: &Options,
    ) -> Option<Rect> {
        calc_node_bbox(node, opt)
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
    let node_bbox = if let Some(bbox) = calc_node_bbox(node, opt) {
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
    let mut layers = create_layers(img_size, opt);

    apply_viewbox_transform(view_box, img_size, dt);

    let curr_ts = *dt.get_transform();
    let mut ts = utils::abs_transform(node);
    ts.append(&node.transform());

    dt.transform(&ts.to_native());
    render_node(node, opt, &mut layers, dt);
    dt.set_transform(&curr_ts);
}

fn create_target(
    size: ScreenSize,
    opt: &Options,
) -> Option<(raqote::DrawTarget, ScreenSize)> {
    let img_size = utils::fit_to(size, opt.fit_to)?;

    let dt = raqote::DrawTarget::new(img_size.width() as i32, img_size.height() as i32);

    Some((dt, img_size))
}

/// Applies viewbox transformation to the painter.
fn apply_viewbox_transform(
    view_box: usvg::ViewBox,
    img_size: ScreenSize,
    dt: &mut raqote::DrawTarget,
) {
    let ts = utils::view_box_to_transform(view_box.rect, view_box.aspect, img_size.to_size());
    dt.transform(&ts.to_native());
}

fn render_node(
    node: &usvg::Node,
    opt: &Options,
    layers: &mut RaqoteLayers,
    dt: &mut raqote::DrawTarget,
) -> Option<Rect> {
    match *node.borrow() {
        usvg::NodeKind::Svg(_) => {
            Some(render_group(node, opt, layers, dt))
        }
        usvg::NodeKind::Path(ref path) => {
            path::draw(&node.tree(), path, opt, &raqote::DrawOptions::default(), dt)
        }
        usvg::NodeKind::Image(ref img) => {
            Some(image::draw(img, opt, dt))
        }
        usvg::NodeKind::Group(ref g) => {
            render_group_impl(node, g, opt, layers, dt)
        }
        _ => None,
    }
}

fn render_group(
    parent: &usvg::Node,
    opt: &Options,
    layers: &mut RaqoteLayers,
    dt: &mut raqote::DrawTarget,
) -> Rect {
    let curr_ts = *dt.get_transform();
    let mut g_bbox = Rect::new_bbox();

    for node in parent.children() {
        dt.transform(&node.transform().to_native());

        let bbox = render_node(&node, opt, layers, dt);

        if let Some(bbox) = bbox {
            let bbox = bbox.transform(&node.transform()).unwrap();
            g_bbox = g_bbox.expand(bbox);
        }

        // Revert transform.
        dt.set_transform(&curr_ts);
    }

    g_bbox
}

fn render_group_impl(
    node: &usvg::Node,
    g: &usvg::Group,
    opt: &Options,
    layers: &mut RaqoteLayers,
    dt: &mut raqote::DrawTarget,
) -> Option<Rect> {
    let sub_dt = layers.get()?;
    let mut sub_dt = sub_dt.borrow_mut();

    let curr_ts = *dt.get_transform();

    let bbox = {
        sub_dt.set_transform(&curr_ts);
        render_group(node, opt, layers, &mut sub_dt)
    };

    if let Some(ref id) = g.filter {
        if let Some(filter_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::Filter(ref filter) = *filter_node.borrow() {
                let ts = usvg::Transform::from_native(&curr_ts);
                filter::apply(filter, bbox, &ts, opt, &mut sub_dt);
            }
        }
    }

    if let Some(ref id) = g.clip_path {
        if let Some(clip_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::ClipPath(ref cp) = *clip_node.borrow() {
                sub_dt.set_transform(&curr_ts);

                clip_and_mask::clip(&clip_node, cp, opt, bbox, layers, &mut sub_dt);
            }
        }
    }

    if let Some(ref id) = g.mask {
        if let Some(mask_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::Mask(ref mask) = *mask_node.borrow() {
                sub_dt.set_transform(&curr_ts);

                clip_and_mask::mask(&mask_node, mask, opt, bbox, layers, &mut sub_dt);
            }
        }
    }

    dt.set_transform(&raqote::Transform::default());

    let sub_img = raqote::Image {
        width: layers.image_size().width() as i32,
        height: layers.image_size().height() as i32,
        data: sub_dt.get_data(),
    };
    dt.draw_image_at(0.0, 0.0, &sub_img, &raqote::DrawOptions {
        blend_mode: raqote::BlendMode::SrcOver,
        alpha: g.opacity.value() as f32,
    });

    dt.set_transform(&curr_ts);

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
    let (mut dt, img_view) = create_target(
        ScreenSize::new(1, 1).unwrap(),
        opt,
    )?;

    // We also have to apply the viewbox transform,
    // otherwise text hinting will be different and bbox will be different too.
    apply_viewbox_transform(tree.svg_node().view_box, img_view, &mut dt);

    let abs_ts = utils::abs_transform(node);
    _calc_node_bbox(node, opt, abs_ts, &mut dt)
}

fn _calc_node_bbox(
    node: &usvg::Node,
    opt: &Options,
    ts: usvg::Transform,
    dt: &mut raqote::DrawTarget,
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
                if let Some(c_bbox) = _calc_node_bbox(&child, opt, ts2, dt) {
                    bbox = bbox.expand(c_bbox);
                }
            }

            Some(bbox)
        }
        _ => None
    }
}

fn create_layers(img_size: ScreenSize, opt: &Options) -> RaqoteLayers {
    layers::Layers::new(img_size, opt.usvg.dpi, create_subsurface, clear_subsurface)
}

fn create_subsurface(
    size: ScreenSize,
    _: f64,
) -> Option<raqote::DrawTarget> {
    Some(raqote::DrawTarget::new(size.width() as i32, size.height() as i32))
}

fn clear_subsurface(dt: &mut raqote::DrawTarget) {
    dt.clear(raqote::SolidSource { r: 0, g: 0, b: 0, a: 0 });
}
