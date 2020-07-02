// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use log::warn;

pub(crate) mod prelude {
    pub(crate) use usvg::*;
    pub(crate) use crate::layers::Layers;
    pub(crate) use crate::*;
    pub(crate) use tiny_skia as skia;
    pub(crate) use super::*;
}

use prelude::*;


/// Indicates the current rendering state.
#[derive(Clone, PartialEq, Debug)]
pub(crate) enum RenderState {
    /// A default value. Doesn't indicate anything.
    Ok,
    /// Indicates that the current rendering task should stop after reaching the specified node.
    RenderUntil(usvg::Node),
    /// Indicates that `usvg::FilterInput::BackgroundImage` rendering task was finished.
    BackgroundFinished,
}


pub(crate) trait ConvTransform {
    fn to_native(&self) -> skia::Transform;
    fn from_native(_: skia::Transform) -> Self;
}

impl ConvTransform for usvg::Transform {
    fn to_native(&self) -> skia::Transform {
        skia::Transform::new(
            self.a as f32, self.b as f32, self.c as f32,
            self.d as f32, self.e as f32, self.f as f32,
        )
    }

    fn from_native(ts: skia::Transform) -> Self {
        Self::new(
            ts.a as f64, ts.b as f64, ts.c as f64,
            ts.d as f64, ts.e as f64, ts.f as f64,
        )
    }
}

pub(crate) fn render_to_canvas(
    tree: &usvg::Tree,
    img_size: ScreenSize,
    canvas: &mut tiny_skia::Canvas,
) {
    render_node_to_canvas(&tree.root(), tree.svg_node().view_box, img_size, &mut RenderState::Ok, canvas);
}

pub(crate) fn render_node_to_canvas(
    node: &usvg::Node,
    view_box: usvg::ViewBox,
    img_size: ScreenSize,
    state: &mut RenderState,
    canvas: &mut skia::Canvas,
) {
    let mut layers = Layers::new(img_size);

    apply_viewbox_transform(view_box, img_size, canvas);

    let curr_ts = canvas.get_transform();

    let mut ts = node.abs_transform();
    ts.append(&node.transform());

    canvas.concat(ts.to_native());
    render_node(node, state, &mut layers, canvas);
    canvas.set_transform(curr_ts);
}

pub(crate) fn create_root_image(
    size: ScreenSize,
    fit_to: usvg::FitTo,
    background: Option<usvg::Color>,
) -> Option<(skia::Surface, ScreenSize)> {
    let img_size = fit_to.fit_to(size)?;

    let mut img = create_subsurface(img_size)?;

    if let Some(c) = background {
        img.fill(c.red, c.green, c.blue, 255);
    }

    Some((img, img_size))
}

/// Applies viewbox transformation to the painter.
fn apply_viewbox_transform(
    view_box: usvg::ViewBox,
    img_size: ScreenSize,
    canvas: &mut skia::Canvas,
) {
    let ts = usvg::utils::view_box_to_transform(view_box.rect, view_box.aspect, img_size.to_size());
    canvas.concat(ts.to_native());
}

pub(crate) fn render_node(
    node: &usvg::Node,
    state: &mut RenderState,
    layers: &mut Layers,
    canvas: &mut skia::Canvas,
) -> Option<Rect> {
    match *node.borrow() {
        usvg::NodeKind::Svg(_) => {
            render_group(node, state, layers, canvas)
        }
        usvg::NodeKind::Path(ref path) => {
            crate::path::draw(&node.tree(), path, skia::BlendMode::SourceOver, canvas)
        }
        usvg::NodeKind::Image(ref img) => {
            Some(crate::image::draw(img, canvas))
        }
        usvg::NodeKind::Group(ref g) => {
            render_group_impl(node, g, state, layers, canvas)
        }
        _ => None,
    }
}

pub(crate) fn render_group(
    parent: &usvg::Node,
    state: &mut RenderState,
    layers: &mut Layers,
    canvas: &mut skia::Canvas,
) -> Option<Rect> {
    let curr_ts = canvas.get_transform();
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

        canvas.concat(node.transform().to_native());

        let bbox = render_node(&node, state, layers, canvas);
        if let Some(bbox) = bbox {
            if let Some(bbox) = bbox.transform(&node.transform()) {
                g_bbox = g_bbox.expand(bbox);
            }
        }

        // Revert transform.
        canvas.set_transform(curr_ts);
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
    state: &mut RenderState,
    layers: &mut Layers,
    canvas: &mut skia::Canvas,
) -> Option<Rect> {
    let sub_surface = layers.get()?;
    let mut sub_surface = sub_surface.borrow_mut();

    let curr_ts = canvas.get_transform();

    let bbox = {
        sub_surface.set_transform(curr_ts);
        render_group(node, state, layers, &mut sub_surface)
    };

    // During the background rendering for filters,
    // an opacity, a filter, a clip and a mask should be ignored for the inner group.
    // So we are simply rendering the `sub_img` without any postprocessing.
    //
    // SVG spec, 15.6 Accessing the background image
    // 'Any filter effects, masking and group opacity that might be set on A[i] do not apply
    // when rendering the children of A[i] into BUF[i].'
    if *state == RenderState::BackgroundFinished {
        let curr_ts = canvas.get_transform();
        canvas.reset_transform();
        canvas.draw_surface(
            &sub_surface, 0.0, 0.0, 255, skia::BlendMode::SourceOver, skia::FilterQuality::Low,
        );
        canvas.set_transform(curr_ts);
        return bbox;
    }

    // Filter can be rendered on an object without a bbox,
    // as long as filter uses `userSpaceOnUse`.
    if let Some(ref id) = g.filter {
        if let Some(filter_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::Filter(ref filter) = *filter_node.borrow() {
                let ts = usvg::Transform::from_native(curr_ts);
                let background = prepare_filter_background(node, filter, layers.image_size());
                let fill_paint = prepare_filter_fill_paint(node, filter, bbox, ts, &sub_surface);
                let stroke_paint = prepare_filter_stroke_paint(node, filter, bbox, ts, &sub_surface);
                crate::filter::apply(filter, bbox, &ts, &node.tree(),
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
                    sub_surface.set_transform(curr_ts);
                    crate::clip::clip(&clip_node, cp, bbox, layers, &mut sub_surface);
                }
            }
        }

        if let Some(ref id) = g.mask {
            if let Some(mask_node) = node.tree().defs_by_id(id) {
                if let usvg::NodeKind::Mask(ref mask) = *mask_node.borrow() {
                    sub_surface.set_transform(curr_ts);
                    crate::mask::mask(&mask_node, mask, bbox, layers, &mut sub_surface);
                }
            }
        }
    }

    let a = if !g.opacity.is_default() {
        g.opacity.to_u8()
    } else {
        255
    };

    let curr_ts = canvas.get_transform();
    canvas.reset_transform();
    canvas.draw_surface(
        &sub_surface, 0.0, 0.0, a, skia::BlendMode::SourceOver, skia::FilterQuality::Low,
    );
    canvas.set_transform(curr_ts);

    bbox
}

/// Renders an image used by `BackgroundImage` or `BackgroundAlpha` filter inputs.
fn prepare_filter_background(
    parent: &usvg::Node,
    filter: &usvg::Filter,
    img_size: ScreenSize,
) -> Option<skia::Surface> {
    let start_node = parent.filter_background_start_node(filter)?;

    let tree = parent.tree();
    let mut img = create_subsurface(img_size)?;
    let view_box = tree.svg_node().view_box;

    // Render from the `start_node` until the `parent`. The `parent` itself is excluded.
    let mut state = RenderState::RenderUntil(parent.clone());
    crate::render::render_node_to_canvas(&start_node, view_box, img_size, &mut state, &mut img);

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
    canvas: &skia::Surface,
) -> Option<skia::Surface> {
    let region = crate::filter::calc_region(filter, bbox, &ts, canvas).ok()?;
    let mut surface = create_subsurface(region.size())?;
    if let usvg::NodeKind::Group(ref g) = *parent.borrow() {
        if let Some(paint) = g.filter_fill.clone() {
            let style_bbox = bbox.unwrap_or_else(|| Rect::new(0.0, 0.0, 1.0, 1.0).unwrap());
            let fill = Some(usvg::Fill::from_paint(paint));
            let fill = crate::paint_server::fill(&parent.tree(), &fill, style_bbox, ts);
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
    canvas: &skia::Surface,
) -> Option<skia::Surface> {
    let region = crate::filter::calc_region(filter, bbox, &ts, canvas).ok()?;
    let mut surface = create_subsurface(region.size())?;
    if let usvg::NodeKind::Group(ref g) = *parent.borrow() {
        if let Some(paint) = g.filter_stroke.clone() {
            let style_bbox = bbox.unwrap_or_else(|| Rect::new(0.0, 0.0, 1.0, 1.0).unwrap());
            let fill = Some(usvg::Fill::from_paint(paint));
            let fill = crate::paint_server::fill(&parent.tree(), &fill, style_bbox, ts);
            surface.draw_rect(0.0, 0.0, region.width() as f64, region.height() as f64, &fill);
        }
    }

    Some(surface)
}

pub(crate) fn create_subsurface(size: ScreenSize) -> Option<skia::Surface> {
    let surface = skia::Surface::new_rgba_premultiplied(size.width(), size.height());
    match surface {
        Some(mut surface) => {
            surface.fill(0, 0, 0, 0);
            Some(surface)
        }
        None => {
            warn!("Failed to create a {}x{} surface.", size.width(), size.height());
            None
        }
    }
}
