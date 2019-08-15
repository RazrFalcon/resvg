// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Skia backend implementation.

use crate::skia;
use log::warn;

use crate::{prelude::*, layers, ConvTransform};

macro_rules! try_create_surface {
    ($size:expr, $ret:expr) => {
        try_opt_warn_or!(
            skia::Surface::new_rgba_premultiplied($size.width(), $size.height()),
            $ret,
            "Failed to create a {}x{} surface.", $size.width(), $size.height()
        );
    };
}

type SkiaLayers = layers::Layers<skia::Surface>;

mod clip_and_mask;
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

impl OutputImage for skia::Surface {
    fn save_png(
        &mut self,
        path: &std::path::Path,
    ) -> bool {
        skia::Surface::save_png(self, path.to_str().unwrap())
    }
}

/// Renders SVG to image.
pub fn render_to_image(
    tree: &usvg::Tree,
    opt: &Options,
) -> Option<skia::Surface> {
    let (mut img, img_size) = create_root_image(tree.svg_node().size.to_screen_size(), opt)?;
    render_to_canvas(tree, opt, img_size, &mut img);
    Some(img)
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

    let (mut img, img_size) = create_root_image(node_bbox.size().to_screen_size(), opt)?;

    render_node_to_canvas(node, opt, vbox, img_size, &mut img);
    Some(img)
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
    let mut layers = create_layers(img_size);

    apply_viewbox_transform(view_box, img_size, surface);

    let curr_ts = surface.get_matrix();

    let mut ts = node.abs_transform();
    ts.append(&node.transform());

    surface.concat(&ts.to_native());
    render_node(node, opt, &mut layers, surface);
    surface.set_matrix(&curr_ts);
}

fn create_root_image(
    size: ScreenSize,
    opt: &Options,
) -> Option<(skia::Surface, ScreenSize)> {
    let img_size = utils::fit_to(size, opt.fit_to)?;

    let mut img = try_create_surface!(img_size, None);

    // Fill background.
    if let Some(c) = opt.background {
        img.fill(c.red, c.green, c.blue, 255);
    } else {
        img.fill(0, 0, 0, 0);
    }

    Some((img, img_size))
}

/// Applies viewbox transformation to the painter.
fn apply_viewbox_transform(
    view_box: usvg::ViewBox,
    img_size: ScreenSize,
    surface: &mut skia::Surface,
) {
    let ts = utils::view_box_to_transform(view_box.rect, view_box.aspect, img_size.to_size());
    surface.concat(&ts.to_native());
}

fn render_node(
    node: &usvg::Node,
    opt: &Options,
    layers: &mut SkiaLayers,
    surface: &mut skia::Surface,
) -> Option<Rect> {
    match *node.borrow() {
        usvg::NodeKind::Svg(_) => {
            render_group(node, opt, layers, surface)
        }
        usvg::NodeKind::Path(ref path) => {
            path::draw(&node.tree(), path, opt, skia::BlendMode::SourceOver, surface)
        }
        usvg::NodeKind::Image(ref img) => {
            Some(image::draw(img, opt, surface))
        }
        usvg::NodeKind::Group(ref g) => {
            render_group_impl(node, g, opt, layers, surface)
        }
        _ => None,
    }
}

fn render_group(
    parent: &usvg::Node,
    opt: &Options,
    layers: &mut SkiaLayers,
    surface: &mut skia::Surface,
) -> Option<Rect> {
    let curr_ts = surface.get_matrix();
    let mut g_bbox = Rect::new_bbox();

    for node in parent.children() {
        surface.concat(&node.transform().to_native());

        let bbox = render_node(&node, opt, layers, surface);
        if let Some(bbox) = bbox {
            if let Some(bbox) = bbox.transform(&node.transform()) {
                g_bbox = g_bbox.expand(bbox);
            }
        }

        // Revert transform.
        surface.set_matrix(&curr_ts);
    }

    // Check that bbox was changed, otherwise we will have a rect with x/y set to f64::MAX.
    if g_bbox.fuzzy_ne(&Rect::new_bbox()) {
        Some(g_bbox)
    } else {
        None
    }
}

fn render_group_impl(
    node: &usvg::Node,
    g: &usvg::Group,
    opt: &Options,
    layers: &mut SkiaLayers,
    surface: &mut skia::Surface,
) -> Option<Rect> {
    let sub_surface = layers.get()?;
    let mut sub_surface = sub_surface.borrow_mut();

    let curr_ts = surface.get_matrix();

    let bbox = {
        sub_surface.set_matrix(&curr_ts);
        render_group(node, opt, layers, &mut sub_surface)
    };

    // Filter can be rendered on an object without a bbox,
    // as long as filter uses `userSpaceOnUse`.
    if let Some(ref id) = g.filter {
        if let Some(filter_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::Filter(ref filter) = *filter_node.borrow() {
                let ts = usvg::Transform::from_native(&curr_ts);
                filter::apply(filter, bbox, &ts, opt, &mut sub_surface);
            }
        }
    }

    // Clipping and masking can be done only for objects with a valid bbox.
    if let Some(bbox) = bbox {
        if let Some(ref id) = g.clip_path {
            if let Some(clip_node) = node.tree().defs_by_id(id) {
                if let usvg::NodeKind::ClipPath(ref cp) = *clip_node.borrow() {
                    sub_surface.set_matrix(&curr_ts);
                    clip_and_mask::clip(&clip_node, cp, opt, bbox, layers, &mut sub_surface);
                }
            }
        }

        if let Some(ref id) = g.mask {
            if let Some(mask_node) = node.tree().defs_by_id(id) {
                if let usvg::NodeKind::Mask(ref mask) = *mask_node.borrow() {
                    sub_surface.set_matrix(&curr_ts);
                    clip_and_mask::mask(&mask_node, mask, opt, bbox, layers, &mut sub_surface);
                }
            }
        }
    }

    let a = if !g.opacity.is_default() {
        (g.opacity.value() * 255.0) as u8
    } else {
        255
    };

    let curr_ts = surface.get_matrix();
    surface.reset_matrix();
    surface.draw_surface(
        &sub_surface, 0.0, 0.0, a, skia::BlendMode::SourceOver, skia::FilterQuality::Low,
    );
    surface.set_matrix(&curr_ts);

    bbox
}

fn create_layers(
    img_size: ScreenSize,
) -> SkiaLayers {
    layers::Layers::new(img_size, create_subimage, clear_image)
}

fn create_subimage(
    size: ScreenSize,
) -> Option<skia::Surface> {
    let mut img = try_create_surface!(size, None);
    img.fill(0, 0, 0, 0);

    Some(img)
}

fn clear_image(img: &mut skia::Surface) {
    img.fill(0, 0, 0, 0);
}
