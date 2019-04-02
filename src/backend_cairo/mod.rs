// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Cairo backend implementation.

// external
use cairo::{
    self,
    MatrixTrait,
};
use pangocairo::functions as pc;

extern crate cairo_sys;

// self
use prelude::*;
use {
    backend_utils,
    layers,
};

use std::ffi::CString;

use std::ffi::{ c_void };
use std::os::raw::{ c_uchar, c_uint, c_int };

use std::io;
use std::io::Write;

use self::cairo_sys::{ cairo_status_t, cairo_write_func_t };

use { RenderFormat };

macro_rules! try_create_surface {
    ($size:expr, $ret:expr) => {
        try_opt_warn!(
            cairo::ImageSurface::create(cairo::Format::ARgb32,
                $size.width as i32, $size.height as i32,
            ).ok(),
            $ret,
            "Failed to create a {}x{} surface.", $size.width, $size.height
        );
    };
}


mod clippath;
mod fill;
mod filter;
mod gradient;
mod image;
mod mask;
mod path;
mod pattern;
mod stroke;
mod text;

mod prelude {
    pub use super::super::prelude::*;
    pub type CairoLayers = super::layers::Layers<super::cairo::ImageSurface>;

    // It's actually used. Rust bug?
    #[allow(unused_imports)]
    pub(super) use super::ReCairoContextExt;
}


type CairoLayers = layers::Layers<cairo::ImageSurface>;


impl ConvTransform<cairo::Matrix> for usvg::Transform {
    fn to_native(&self) -> cairo::Matrix {
        cairo::Matrix::new(self.a, self.b, self.c, self.d, self.e, self.f)
    }

    fn from_native(ts: &cairo::Matrix) -> Self {
        Self::new(ts.xx, ts.yx, ts.xy, ts.yy, ts.x0, ts.y0)
    }
}

impl TransformFromBBox for cairo::Matrix {
    fn from_bbox(bbox: Rect) -> Option<Self> {
        if bbox.is_valid() {
            Some(Self::new(bbox.width, 0.0, 0.0, bbox.height, bbox.x, bbox.y))
        } else {
            None
        }
    }
}

pub(crate) trait ReCairoContextExt {
    fn set_source_color(&self, color: usvg::Color, opacity: usvg::Opacity);
    fn reset_source_rgba(&self);
}

impl ReCairoContextExt for cairo::Context {
    fn set_source_color(&self, color: usvg::Color, opacity: usvg::Opacity) {
        self.set_source_rgba(
            color.red as f64 / 255.0,
            color.green as f64 / 255.0,
            color.blue as f64 / 255.0,
            opacity.value(),
        );
    }

    fn reset_source_rgba(&self) {
        self.set_source_rgba(0.0, 0.0, 0.0, 0.0);
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

    fn render_to_stream(
        &self,
        tree: &usvg::Tree,
        opt: &Options,
        format: RenderFormat,
        writer: &mut dyn Write
    ) -> bool {
        render_to_stream(tree, opt, format,writer)
    }

    fn render_node_to_stream(
        &self,
        node: &usvg::Node,
        opt: &Options,
        format: RenderFormat,
        writer: &mut dyn Write
    ) -> bool {
        render_node_to_stream(node, opt, format, writer)
    }
}

impl OutputImage for cairo::ImageSurface {
    fn save(&self, path: &::std::path::Path) -> bool {
        use std::fs;

        if let Ok(mut buffer) = fs::File::create(path) {
            if let Ok(_) = self.write_to_png(&mut buffer) {
                return true;
            }
        }

        false
    }
}

struct State<'a> {
    writer: &'a mut dyn Write,
}

pub fn render_to_stream(
    tree: &usvg::Tree,
    opt: &Options,
    format: RenderFormat,
    writer: &mut dyn Write
) -> bool {
    let size = tree.svg_node().size;
    let screen_size = tree.svg_node().size.to_screen_size();
    render_to_stream_common(
        &tree.root(),
        opt,
        format,
        tree.svg_node().size.to_screen_size(),
        tree.svg_node().view_box,
        writer
    )
}

pub fn render_node_to_stream(
    node: &usvg::Node,
    opt: &Options,
    format: RenderFormat,
    writer: &mut dyn Write
) -> bool {
    let node_bbox = if let Some(bbox) = calc_node_bbox(node, opt) {
        bbox
    } else {
        warn!("Node '{}' has a zero size.", node.id());
        return false;
    };
    let vbox = usvg::ViewBox {
        rect: node_bbox,
        aspect: usvg::AspectRatio::default(),
    };
    let size = node_bbox;
    let screen_size = size.to_screen_size();

    render_to_stream_common(
        &node,
        opt,
        format,
        screen_size,
        vbox,
        writer
    )
}


fn render_to_stream_common(
    node: &usvg::Node,
    opt: &Options,
    format: RenderFormat,
    size: ScreenSize,
    view_box: usvg::ViewBox,
    writer: &mut dyn Write
) -> bool {
    let mut state = State { writer: writer };
    let state_ptr: *mut c_void = &mut state as *mut _ as *mut c_void;

    unsafe extern "C" fn output_to_stdout(data: *mut c_void, buffer: *mut c_uchar, len: c_uint) -> cairo_status_t {
        let data: &mut State = unsafe { &mut *(data as *mut State) };
        let slice = std::slice::from_raw_parts(buffer, len as usize);
        data.writer.write(slice);
        cairo_sys::STATUS_SUCCESS
    }

    let surface = unsafe { match format  {
        RenderFormat::PNG => {
            let image_surface = cairo_sys::cairo_image_surface_create(
                cairo::Format::ARgb32.into(),
                size.width as i32,
                size.height as i32,
            );
            cairo::Surface::from_raw_full(image_surface)
        },
        RenderFormat::SVG | RenderFormat::PDF | RenderFormat::EPS | RenderFormat::PS => {
            let surface_create_for_stream = match format {
                RenderFormat::SVG => cairo_sys::cairo_svg_surface_create_for_stream,
                RenderFormat::PDF => cairo_sys::cairo_pdf_surface_create_for_stream,
                RenderFormat::PS | RenderFormat::EPS => {
                    cairo_sys::cairo_ps_surface_create_for_stream
                },
                _ => panic!("Unrecognized format")
            };

            let surface_ptr = surface_create_for_stream(
                Some(output_to_stdout),
                state_ptr,
                size.width.into(),
                size.height.into()
            );

            match format {
                RenderFormat::EPS => cairo_sys::cairo_ps_surface_set_eps(
                    surface_ptr,
                    cairo_sys::cairo_bool_t::from(true)
                ),
                _ => {}
            };

            cairo::Surface::from_raw_full(surface_ptr)
        },
        _ => panic!("Unrecognized format")
    } };

    let cr = cairo::Context::new(&surface);

    // Fill background.
    if let Some(color) = opt.background {
        cr.set_source_color(color, 1.0.into());
        cr.paint();
    }

    render_node_to_canvas(node, opt, view_box, size, &cr);

    unsafe { match format {
        RenderFormat::PNG => {
            cairo_sys::cairo_surface_write_to_png_stream(
                surface.to_raw_none(),
                Some(output_to_stdout),
                state_ptr) == cairo_sys::STATUS_SUCCESS
        },
        RenderFormat::SVG | RenderFormat::PDF | RenderFormat::EPS | RenderFormat::PS => {
            cairo_sys::cairo_show_page(cr.to_raw_none());
            true
        }
    } }
}


/// Renders SVG to image.
pub fn render_to_image(
    tree: &usvg::Tree,
    opt: &Options,
) -> Option<cairo::ImageSurface> {
    // let (surface, img_view) = create_surface(
    //     tree.svg_node().size.to_screen_size(),
    //     opt,
    // )?;
    let out = CString::new("out.svg").expect("CString::new failed");
    let ptr = out.as_ptr();
    let size = tree.svg_node().size.to_screen_size();
    let surface = unsafe {
         let surface_ptr = cairo_sys::cairo_svg_surface_create(
            ptr,
            size.width.into(),
            size.height.into()
        );

        cairo::Surface::from_raw_full(surface_ptr)
    };

    let cr = cairo::Context::new(&surface);

    // Fill background.
    if let Some(color) = opt.background {
        cr.set_source_color(color, 1.0.into());
        cr.paint();
    }

    render_to_canvas(tree, opt, size, &cr);

    println!("SHOW PAGE");
    unsafe {
        cairo_sys::cairo_show_page(cr.to_raw_none());
    }

    None
    // Some(surface)
}

/// Renders SVG to image.
pub fn render_node_to_surface(
    node: &usvg::Node,
    opt: &Options,
    surface: &cairo::Surface
) -> bool {
    let node_bbox = if let Some(bbox) = calc_node_bbox(node, opt) {
        bbox
    } else {
        warn!("Node '{}' has a zero size.", node.id());
        return false;
    };

    //let (surface, img_size) = create_surface(node_bbox.to_screen_size(), opt)?;
    let size = node_bbox.to_screen_size();
    let img_size = utils::fit_to(size, opt.fit_to);

    let vbox = usvg::ViewBox {
        rect: node_bbox,
        aspect: usvg::AspectRatio::default(),
    };

    let cr = cairo::Context::new(&surface);

    // Fill background.
    if let Some(color) = opt.background {
        cr.set_source_color(color, 1.0.into());
        cr.paint();
    }

    render_node_to_canvas(node, opt, vbox, img_size, &cr);

    true
}

/// Renders SVG to image.
pub fn render_node_to_image(
    node: &usvg::Node,
    opt: &Options,
) -> Option<cairo::ImageSurface> {
    let node_bbox = if let Some(bbox) = calc_node_bbox(node, opt) {
        bbox
    } else {
        warn!("Node '{}' has a zero size.", node.id());
        return None;
    };
    // let (surface, img_size) = create_surface(node_bbox.to_screen_size(), opt)?;
    let out = CString::new("out.svg").expect("CString::new failed");
    let ptr = out.as_ptr();

    println!("Hello {0}", node_bbox);
    let surface = unsafe {
         let surface_ptr = cairo_sys::cairo_svg_surface_create(
            ptr,
            node_bbox.width,
            node_bbox.height
        );

        cairo::Surface::from_raw_full(surface_ptr)
    };

    let status = render_node_to_surface(node, opt, &surface);
    if !status {
        return None
    }

    None
    // Some(surface)
}

/// Renders SVG to canvas.
pub fn render_to_canvas(
    tree: &usvg::Tree,
    opt: &Options,
    img_size: ScreenSize,
    cr: &cairo::Context,
) {
    render_node_to_canvas(&tree.root(), opt, tree.svg_node().view_box,
                          img_size, cr);
}

/// Renders SVG node to canvas.
pub fn render_node_to_canvas(
    node: &usvg::Node,
    opt: &Options,
    view_box: usvg::ViewBox,
    img_size: ScreenSize,
    cr: &cairo::Context,
) {
    let mut layers = create_layers(img_size, opt);

    apply_viewbox_transform(view_box, img_size, &cr);

    let curr_ts = cr.get_matrix();
    let mut ts = utils::abs_transform(node);
    ts.append(&node.transform());

    cr.transform(ts.to_native());
    render_node(node, opt, &mut layers, cr);
    cr.set_matrix(curr_ts);
}

fn create_surface(
    size: ScreenSize,
    opt: &Options,
) -> Option<(cairo::ImageSurface, ScreenSize)> {
    let img_size = utils::fit_to(size, opt.fit_to);

    debug_assert_ne!(img_size.width, 0);
    debug_assert_ne!(img_size.height, 0);
    let surface = try_create_surface!(img_size, None);

    Some((surface, img_size))
}

/// Applies viewbox transformation to the painter.
fn apply_viewbox_transform(
    view_box: usvg::ViewBox,
    img_size: ScreenSize,
    cr: &cairo::Context,
) {
    let ts = utils::view_box_to_transform(view_box.rect, view_box.aspect, img_size.to_size());
    cr.transform(ts.to_native());
}

fn render_node(
    node: &usvg::Node,
    opt: &Options,
    layers: &mut CairoLayers,
    cr: &cairo::Context,
) -> Option<Rect> {
    match *node.borrow() {
        usvg::NodeKind::Svg(_) => {
            Some(render_group(node, opt, layers, cr))
        }
        usvg::NodeKind::Path(ref path) => {
            Some(path::draw(&node.tree(), path, opt, cr))
        }
        usvg::NodeKind::Text(ref text) => {
            Some(text::draw(&node.tree(), text, opt, cr))
        }
        usvg::NodeKind::Image(ref img) => {
            Some(image::draw(img, opt, cr))
        }
        usvg::NodeKind::Group(ref g) => {
            render_group_impl(node, g, opt, layers, cr)
        }
        _ => None,
    }
}

fn render_group(
    parent: &usvg::Node,
    opt: &Options,
    layers: &mut CairoLayers,
    cr: &cairo::Context,
) -> Rect {
    let curr_ts = cr.get_matrix();
    let mut g_bbox = Rect::new_bbox();

    for node in parent.children() {
        cr.transform(node.transform().to_native());

        let bbox = render_node(&node, opt, layers, cr);

        if let Some(bbox) = bbox {
            let bbox = bbox.transform(&node.transform());
            g_bbox.expand(bbox);
        }

        // Revert transform.
        cr.set_matrix(curr_ts);
    }

    g_bbox
}

fn render_group_impl(
    node: &usvg::Node,
    g: &usvg::Group,
    opt: &Options,
    layers: &mut CairoLayers,
    cr: &cairo::Context,
) -> Option<Rect> {
    let sub_surface = layers.get()?;
    let mut sub_surface = sub_surface.borrow_mut();

    let curr_ts = cr.get_matrix();

    let bbox = {
        let sub_cr = cairo::Context::new(&*sub_surface);
        sub_cr.set_matrix(curr_ts);

        render_group(node, opt, layers, &sub_cr)
    };

    if let Some(ref id) = g.filter {
        if let Some(filter_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::Filter(ref filter) = *filter_node.borrow() {
                let ts = usvg::Transform::from_native(&curr_ts);
                filter::apply(filter, bbox, &ts, opt, &mut *sub_surface);
            }
        }
    }

    if let Some(ref id) = g.clip_path {
        if let Some(clip_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::ClipPath(ref cp) = *clip_node.borrow() {
                let sub_cr = cairo::Context::new(&*sub_surface);
                sub_cr.set_matrix(curr_ts);

                clippath::apply(&clip_node, cp, opt, bbox, layers, &sub_cr);
            }
        }
    }

    if let Some(ref id) = g.mask {
        if let Some(mask_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::Mask(ref mask) = *mask_node.borrow() {
                let sub_cr = cairo::Context::new(&*sub_surface);
                sub_cr.set_matrix(curr_ts);

                mask::apply(&mask_node, mask, opt, bbox, layers, &sub_cr);
            }
        }
    }

    let curr_matrix = cr.get_matrix();
    cr.set_matrix(cairo::Matrix::identity());
    cr.set_source_surface(&*sub_surface, 0.0, 0.0);
    if !g.opacity.is_default() {
        cr.paint_with_alpha(g.opacity.value());
    } else {
        cr.paint();
    }

    cr.set_matrix(curr_matrix);

    // All layers must be unlinked from the main context/cr after used.
    // TODO: find a way to automate this
    cr.reset_source_rgba();

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
    let (surface, img_view) = create_surface(
        tree.svg_node().size.to_screen_size(),
        opt,
    )?;
    let cr = cairo::Context::new(&surface);

    // We also have to apply the viewbox transform,
    // otherwise text hinting will be different and bbox will be different too.
    apply_viewbox_transform(tree.svg_node().view_box, img_view, &cr);

    let abs_ts = utils::abs_transform(node);
    _calc_node_bbox(node, opt, abs_ts, &cr)
}

fn _calc_node_bbox(
    node: &usvg::Node,
    opt: &Options,
    ts: usvg::Transform,
    cr: &cairo::Context,
) -> Option<Rect> {
    let mut ts2 = ts;
    ts2.append(&node.transform());

    match *node.borrow() {
        usvg::NodeKind::Path(ref path) => {
            Some(utils::path_bbox(&path.segments, path.stroke.as_ref(), &ts2))
        }
        usvg::NodeKind::Text(ref text) => {
            let mut bbox = Rect::new_bbox();
            let mut fm = text::PangoFontMetrics::new(opt, cr);
            let (blocks, _) = backend_utils::text::prepare_blocks(text, &mut fm);
            backend_utils::text::draw_blocks(blocks, |block| {
                cr.new_path();

                let context = text::init_pango_context(opt, cr);
                let layout = text::init_pango_layout(&block, &context);

                pc::layout_path(cr, &layout);
                let path = cr.copy_path();
                let segments = from_cairo_path(&path);

                let mut t = ts2;
                if let Some(rotate) = block.rotate {
                    t.rotate_at(rotate, block.bbox.x, block.bbox.y + block.font_ascent);
                }
                t.translate(block.bbox.x, block.bbox.y);

                if !segments.is_empty() {
                    let c_bbox = utils::path_bbox(&segments, block.stroke.as_ref(), &t);
                    bbox.expand(c_bbox);
                }
            });

            Some(bbox)
        }
        usvg::NodeKind::Image(ref img) => {
            let segments = utils::rect_to_path(img.view_box.rect);
            Some(utils::path_bbox(&segments, None, &ts2))
        }
        usvg::NodeKind::Group(_) => {
            let mut bbox = Rect::new_bbox();

            for child in node.children() {
                if let Some(c_bbox) = _calc_node_bbox(&child, opt, ts2, cr) {
                    bbox.expand(c_bbox);
                }
            }

            Some(bbox)
        }
        _ => None
    }
}

fn from_cairo_path(path: &cairo::Path) -> Vec<usvg::PathSegment> {
    let mut segments = Vec::new();
    for seg in path.iter() {
        match seg {
            cairo::PathSegment::MoveTo((x, y)) => {
                segments.push(usvg::PathSegment::MoveTo { x, y });
            }
            cairo::PathSegment::LineTo((x, y)) => {
                segments.push(usvg::PathSegment::LineTo { x, y });
            }
            cairo::PathSegment::CurveTo((x1, y1), (x2, y2), (x, y)) => {
                segments.push(usvg::PathSegment::CurveTo { x1, y1, x2, y2, x, y });
            }
            cairo::PathSegment::ClosePath => {
                segments.push(usvg::PathSegment::ClosePath);
            }
        }
    }

    if segments.len() == 1 {
        segments.clear();
    }

    segments
}

fn create_layers(img_size: ScreenSize, opt: &Options) -> CairoLayers {
    layers::Layers::new(img_size, opt.usvg.dpi, create_subsurface, clear_subsurface)
}

fn create_subsurface(
    size: ScreenSize,
    _: f64,
) -> Option<cairo::ImageSurface> {
    Some(try_create_surface!(size, None))
}

fn clear_subsurface(surface: &mut cairo::ImageSurface) {
    let cr = cairo::Context::new(&surface);
    cr.set_operator(cairo::Operator::Clear);
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
    cr.paint();
}
