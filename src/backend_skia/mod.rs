// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Skia backend implementation.

use crate::skia;
use log::warn;

use crate::{prelude::*, layers, ConvTransform, RenderState};

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

    fn make_vec(&mut self) -> Vec<u8> {
        self.data().to_vec()
    }

    fn make_rgba_vec(&mut self) -> Vec<u8> {
        use rgb::FromSlice;
        use std::mem::swap;

        let mut data = self.make_vec();

        // BGRA_Premultiplied -> BGRA
        svgfilters::demultiply_alpha(data.as_bgra_mut());
        // BGRA -> RGBA.
        if skia::Surface::is_bgra() {
            data.as_bgra_mut().iter_mut().for_each(|p| swap(&mut p.r, &mut p.b));
        }

        data
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
    render_node_to_canvas_impl(node, opt, view_box, img_size, &mut RenderState::Ok, canvas)
}

fn render_node_to_canvas_impl(
    node: &usvg::Node,
    opt: &Options,
    view_box: usvg::ViewBox,
    img_size: ScreenSize,
    state: &mut RenderState,
    canvas: &mut skia::Canvas,
) {
    let mut layers = create_layers(img_size);

    apply_viewbox_transform(view_box, img_size, canvas);

    let curr_ts = canvas.get_matrix();

    let mut ts = node.abs_transform();
    ts.append(&node.transform());

    canvas.concat(&ts.to_native());
    render_node(node, opt, state, &mut layers, canvas);
    canvas.set_matrix(&curr_ts);
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
    canvas: &mut skia::Canvas,
) {
    let ts = utils::view_box_to_transform(view_box.rect, view_box.aspect, img_size.to_size());
    canvas.concat(&ts.to_native());
}

fn render_node(
    node: &usvg::Node,
    opt: &Options,
    state: &mut RenderState,
    layers: &mut SkiaLayers,
    canvas: &mut skia::Canvas,
) -> Option<Rect> {
    match *node.borrow() {
        usvg::NodeKind::Svg(_) => {
            render_group(node, opt, state, layers, canvas)
        }
        usvg::NodeKind::Path(ref path) => {
            path::draw(&node.tree(), path, opt, skia::BlendMode::SourceOver, canvas)
        }
        usvg::NodeKind::Image(ref img) => {
            Some(image::draw(img, opt, canvas))
        }
        usvg::NodeKind::Group(ref g) => {
            render_group_impl(node, g, opt, state, layers, canvas)
        }
        _ => None,
    }
}

fn render_group(
    parent: &usvg::Node,
    opt: &Options,
    state: &mut RenderState,
    layers: &mut SkiaLayers,
    canvas: &mut skia::Canvas,
) -> Option<Rect> {
    let curr_ts = canvas.get_matrix();
    let mut g_bbox = Rect::new_bbox();

    for node in parent.children() {
        match state {
            RenderState::Ok => {}
            RenderState::RenderUntil(ref last) => {
                if node == *last {
                    // Stop rendering.
                    *state = RenderState::BackgroundFinished;
                    break;
                }
            }
            RenderState::BackgroundFinished => break,
        }

        canvas.concat(&node.transform().to_native());

        let bbox = render_node(&node, opt, state, layers, canvas);
        if let Some(bbox) = bbox {
            if let Some(bbox) = bbox.transform(&node.transform()) {
                g_bbox = g_bbox.expand(bbox);
            }
        }

        // Revert transform.
        canvas.set_matrix(&curr_ts);
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
    state: &mut RenderState,
    layers: &mut SkiaLayers,
    canvas: &mut skia::Canvas,
) -> Option<Rect> {
    let sub_surface = layers.get()?;
    let mut sub_surface = sub_surface.borrow_mut();

    let curr_ts = canvas.get_matrix();

    let bbox = {
        sub_surface.set_matrix(&curr_ts);
        render_group(node, opt, state, layers, &mut sub_surface)
    };

    // During the background rendering for filters,
    // an opacity, a filter, a clip and a mask should be ignored for the inner group.
    // So we are simply rendering the `sub_img` without any postprocessing.
    //
    // SVG spec, 15.6 Accessing the background image
    // 'Any filter effects, masking and group opacity that might be set on A[i] do not apply
    // when rendering the children of A[i] into BUF[i].'
    if *state == RenderState::BackgroundFinished {
        let curr_ts = canvas.get_matrix();
        canvas.reset_matrix();
        canvas.draw_surface(
            &sub_surface, 0.0, 0.0, 255, skia::BlendMode::SourceOver, skia::FilterQuality::Low,
        );
        canvas.set_matrix(&curr_ts);
        return bbox;
    }

    // Filter can be rendered on an object without a bbox,
    // as long as filter uses `userSpaceOnUse`.
    if let Some(ref id) = g.filter {
        if let Some(filter_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::Filter(ref filter) = *filter_node.borrow() {
                let ts = usvg::Transform::from_native(&curr_ts);
                let background = prepare_filter_background(node, filter, opt);
                let fill_paint = prepare_filter_fill_paint(node, filter, bbox, ts, opt, &sub_surface);
                let stroke_paint = prepare_filter_stroke_paint(node, filter, bbox, ts, opt, &sub_surface);
                filter::apply(filter, bbox, &ts, opt, &node.tree(),
                              background.as_ref(), fill_paint.as_ref(), stroke_paint.as_ref(),
                              &mut sub_surface);
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

    let curr_ts = canvas.get_matrix();
    canvas.reset_matrix();
    canvas.draw_surface(
        &sub_surface, 0.0, 0.0, a, skia::BlendMode::SourceOver, skia::FilterQuality::Low,
    );
    canvas.set_matrix(&curr_ts);

    bbox
}

/// Renders an image used by `BackgroundImage` or `BackgroundAlpha` filter inputs.
fn prepare_filter_background(
    parent: &usvg::Node,
    filter: &usvg::Filter,
    opt: &Options,
) -> Option<skia::Surface> {
    let start_node = crate::filter_background_start_node(parent, filter)?;

    let tree = parent.tree();
    let (mut img, img_size) = create_root_image(tree.svg_node().size.to_screen_size(), opt)?;
    let view_box = tree.svg_node().view_box;

    // Render from the `start_node` until the `parent`. The `parent` itself is excluded.
    let mut state = RenderState::RenderUntil(parent.clone());
    render_node_to_canvas_impl(&start_node, opt, view_box, img_size, &mut state, &mut img);

    Some(img)
}

/// Renders an image used by `FillPaint`/`StrokePaint` filter input.
///
/// FillPaint/StrokePaint is mostly an undefined behavior and will produce different results
/// in every application.
/// And since there are no expected behaviour, we will simply fill the filter region.
///
/// https://github.com/w3c/fxtf-drafts/issues/323
fn prepare_filter_fill_paint(
    parent: &usvg::Node,
    filter: &usvg::Filter,
    bbox: Option<Rect>,
    ts: usvg::Transform,
    opt: &Options,
    canvas: &skia::Surface,
) -> Option<skia::Surface> {
    let region = crate::filter::calc_region(filter, bbox, &ts, canvas).ok()?;
    let mut surface = create_subimage(region.size())?;
    if let usvg::NodeKind::Group(ref g) = *parent.borrow() {
        if let Some(paint) = g.filter_fill.clone() {
            let style_bbox = bbox.unwrap_or_else(|| Rect::new(0.0, 0.0, 1.0, 1.0).unwrap());
            let fill = Some(usvg::Fill::from_paint(paint));
            let fill = style::fill(&parent.tree(), &fill, opt, style_bbox, ts);
            surface.draw_rect(0.0, 0.0, region.width() as f64, region.height() as f64, &fill);
        }
    }

    Some(surface)
}

/// The same as `prepare_filter_fill_paint`, but for `StrokePaint`.
fn prepare_filter_stroke_paint(
    parent: &usvg::Node,
    filter: &usvg::Filter,
    bbox: Option<Rect>,
    ts: usvg::Transform,
    opt: &Options,
    canvas: &skia::Surface,
) -> Option<skia::Surface> {
    let region = crate::filter::calc_region(filter, bbox, &ts, canvas).ok()?;
    let mut surface = create_subimage(region.size())?;
    if let usvg::NodeKind::Group(ref g) = *parent.borrow() {
        if let Some(paint) = g.filter_stroke.clone() {
            let style_bbox = bbox.unwrap_or_else(|| Rect::new(0.0, 0.0, 1.0, 1.0).unwrap());
            let fill = Some(usvg::Fill::from_paint(paint));
            let fill = style::fill(&parent.tree(), &fill, opt, style_bbox, ts);
            surface.draw_rect(0.0, 0.0, region.width() as f64, region.height() as f64, &fill);
        }
    }

    Some(surface)
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
