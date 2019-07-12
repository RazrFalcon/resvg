// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Skia backend implementation.

// external
use crate::skia;

// self
use crate::prelude::*;
use crate::layers;
use crate::backend_utils::{
    ConvTransform,
};

macro_rules! try_create_surface {
    ($size:expr, $ret:expr) => {
        usvg::try_opt_warn_or!(
            skia::Surface::new_rgba_premultiplied($size.width(), $size.height()),
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

type SkiaLayers = layers::Layers<skia::Surface>;

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

/// Renders rectangular region from source SVG to canvas.
pub fn render_rect_to_canvas(
    tree: &usvg::Tree,
    opt: &Options,
    img_size: ScreenSize,
    src: &usvg::Rect,
    canvas: &mut skia::Canvas,
) {
    // Translate and scale the source rectangle (after viewbox transformation) into the image size.
    let dst_matrix = skia::Matrix::new();
    dst_matrix.pre_scale(img_size.width() as f64 / src.width(), img_size.height() as f64 / src.height());
    dst_matrix.pre_translate(-src.left(), -src.top());
    canvas.concat(&dst_matrix);

    // Apply the viewbox transform to the viewport (instead of the image size)
    apply_viewbox_transform(tree.svg_node().view_box, tree.svg_node().size, canvas);
    render_node_to_canvas(&tree.root(), opt, img_size, canvas);
    canvas.flush();
}

/// Renders SVG to canvas.
pub fn render_to_canvas(
    tree: &usvg::Tree,
    opt: &Options,
    img_size: ScreenSize,
    canvas: &mut skia::Canvas,
) {
    apply_viewbox_transform(tree.svg_node().view_box, img_size.to_size(), canvas);
    render_node_to_canvas(&tree.root(), opt, img_size, canvas);
    canvas.flush();
}

/// Renders SVG node to canvas.
pub fn render_node_to_canvas(
    node: &usvg::Node,
    opt: &Options,
    img_size: ScreenSize,
    canvas: &mut skia::Canvas,
) {
    let mut layers = create_layers(img_size);

    let curr_mat = canvas.get_total_matrix();

    let mut ts = node.abs_transform();
    ts.append(&node.transform());

    canvas.concat(&ts.to_native());
    render_node(node, opt, &mut layers, canvas);
    canvas.set_matrix(&curr_mat);
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

// Applies viewbox transformation to the canvas.
fn apply_viewbox_transform(
    view_box: usvg::ViewBox,
    img_size: Size,
    canvas: &mut skia::Canvas,
) {
    let ts = utils::view_box_to_transform(view_box.rect, view_box.aspect, img_size);
    canvas.concat(&ts.to_native());
}

fn render_node(
    node: &usvg::Node,
    opt: &Options,
    layers: &mut SkiaLayers,
    canvas: &mut skia::Canvas,
) -> Option<Rect> {
    match *node.borrow() {
        usvg::NodeKind::Svg(_) => {
            Some(render_group(node, opt, layers, canvas))
        }
        usvg::NodeKind::Path(ref path) => {
            path::draw(&node.tree(), path, opt, canvas, skia::BlendMode::SourceOver)
        }
        usvg::NodeKind::Image(ref img) => {
            Some(image::draw(img, opt, canvas))
        }
        usvg::NodeKind::Group(ref g) => {
            render_group_impl(node, g, opt, layers, canvas)
        }
        _ => None,
    }
}

fn render_group(
    parent: &usvg::Node,
    opt: &Options,
    layers: &mut SkiaLayers,
    canvas: &mut skia::Canvas,
) -> Rect {

    let curr_mat = canvas.get_total_matrix();
    let mut g_bbox = Rect::new_bbox();

    for node in parent.children() {
        canvas.concat(&node.transform().to_native());

        let bbox = render_node(&node, opt, layers, canvas);
        if let Some(bbox) = bbox {
            if let Some(bbox) = bbox.transform(&node.transform()) {
                g_bbox = g_bbox.expand(bbox);
            }
        }

        // Revert transform.
        canvas.set_matrix(&curr_mat);
    }

    canvas.flush();

    return g_bbox;
}

fn render_group_impl(
    node: &usvg::Node,
    g: &usvg::Group,
    opt: &Options,
    layers: &mut SkiaLayers,
    canvas: &mut skia::Canvas,
) -> Option<Rect> {

    let sub_surface = layers.get()?;
    let mut sub_surface = sub_surface.borrow_mut();

    let curr_mat = canvas.get_total_matrix();

    let bbox = {
        let mut sub_canvas = sub_surface.get_canvas();
        sub_canvas.set_matrix(&curr_mat);

        render_group(node, opt, layers, &mut sub_canvas)
    };

    if let Some(ref id) = g.filter {
        if let Some(filter_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::Filter(ref filter) = *filter_node.borrow() {
                let ts = usvg::Transform::from_native(&curr_mat);
                filter::apply(filter, Some(bbox), &ts, opt, &mut sub_surface);
            }
        }
    }

    if let Some(ref id) = g.clip_path {
        if let Some(clip_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::ClipPath(ref cp) = *clip_node.borrow() {
                let mut sub_canvas = sub_surface.get_canvas();
                sub_canvas.set_matrix(&curr_mat);
                clip_and_mask::clip(&clip_node, cp, opt, bbox, layers, &mut sub_canvas);
            }
        }
    }

    if let Some(ref id) = g.mask {
        if let Some(mask_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::Mask(ref mask) = *mask_node.borrow() {
                let mut sub_canvas = sub_surface.get_canvas();
                sub_canvas.set_matrix(&curr_mat);
                clip_and_mask::mask(&mask_node, mask, opt, bbox, layers, &mut sub_canvas);
            }
        }
    }

    let opacity = {
        if !g.opacity.is_default() {
            (g.opacity.value()* 255.0) as u8
        }
        else {
            255 as u8
        }
    };

    let curr_mat = canvas.get_total_matrix();

    canvas.reset_matrix();
    canvas.draw_surface(&sub_surface, 0.0, 0.0, opacity, skia::BlendMode::SourceOver);

    canvas.set_matrix(&curr_mat);

    Some(bbox)
}

fn create_layers(
    img_size: ScreenSize
) -> SkiaLayers {
    layers::Layers::new(img_size, create_subsurface, clear_surface)
}

fn create_subsurface(
    size: ScreenSize
) -> Option<skia::Surface> {

    let mut surface = try_create_surface!(size, None);

    let canvas = surface.get_canvas();
    canvas.clear();

    Some(surface)
}

fn clear_surface(surface: &mut skia::Surface) {
    surface.get_canvas().clear();
}
