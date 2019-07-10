// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Skia backend implementation.

// external
use crate::skia;
use crate::svgdom;

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

    // TODO:  finish implementation
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

    let view_box = tree.svg_node().view_box;
    let size = tree.svg_node().size.to_screen_size();
    let img_size = utils::fit_to(size, opt.fit_to)?; 
    
    let mut surface = skia::Surface::new_rgba_premultiplied(img_size.width(), img_size.height())?;       
    let mut canvas = surface.get_canvas();
    canvas.reset_matrix();

    // This stores the GrContext so that layered surfaced are created using the same context.
    skia::Context::set_from_canvas(&canvas);
    canvas.clear(0xFFFFFFFF);

	canvas.save();

    apply_viewbox_transform(view_box, img_size.to_size(), &mut canvas);
    render_node_to_canvas(&tree.root(), opt, img_size, &mut canvas);

	canvas.restore();

    Some(surface)
}

/// Renders SVG node to image.
pub fn render_node_to_image(
    node: &usvg::Node,
    opt: &Options,
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

    let view_box = tree.svg_node().view_box;

    // This stores the GrContext so that layered surfaced are created using the same context.
    // TODO:  This should only get called once!  Current image::draw_svg() is calling here which
    // could change the context if a raster surface is used in a GL enviroment.
    // NOTE:  At the moment only raster surfaces are being created.
    skia::Context::set_from_canvas(canvas);
    
    // Save and restore to remove the view box transformation and clip region.
    canvas.save();
    
	apply_viewbox_transform(view_box, img_size.to_size(), canvas);
	render_node_to_canvas(&tree.root(), opt, img_size, canvas);
    
	canvas.restore();
}

/// Renders rectangular region from source SVG to canvas.
pub fn render_rect_to_canvas(
    tree: &usvg::Tree,
    opt: &Options,
    img_size: ScreenSize,
    src: Option<usvg::Rect>,
    canvas: &mut skia::Canvas,
) {

    let svg_size = tree.svg_node().size;

    // This stores the GrContext so that layered surfaced are created using the same context.
    skia::Context::set_from_canvas(canvas);
    
    let src_rect = match src {
		Some(rect) => { rect }
        None => { Rect::new(0.0, 0.0, svg_size.width(), svg_size.height()).unwrap() }
    };

    // TODO:  What happened to applying the viewbox transform?

	let dst_matrix = skia::Matrix::new();
    dst_matrix.pre_scale(img_size.width() as f64 / src_rect.width(), img_size.height() as f64 / src_rect.height());
    dst_matrix.pre_translate(-src_rect.left(), -src_rect.top());
    canvas.concat(&dst_matrix);

    render_node_to_canvas(&tree.root(), opt, img_size, canvas);    
}


/// Renders SVG node to canvas.
fn render_node_to_canvas(
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


// Applies viewbox transformation to the canvas.
fn apply_viewbox_transform(
    view_box: usvg::ViewBox,
    img_size: Size,
    canvas: &mut skia::Canvas,
) -> skia::Matrix  {
    let ts = utils::view_box_to_transform(view_box.rect, view_box.aspect, img_size);
    let matrix = ts.to_native();
    canvas.concat(&matrix);
    matrix
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
    canvas.clear(0);
    
    Some(surface)
}

fn clear_surface(surface: &mut skia::Surface) {
    surface.get_canvas().clear(0);
}
