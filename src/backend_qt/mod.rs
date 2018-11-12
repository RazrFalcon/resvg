// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Qt backend implementation.

// external
use qt;
use usvg;
use usvg::prelude::*;

// self
use prelude::*;
use {
    layers,
    OutputImage,
    Render,
};


macro_rules! try_create_image {
    ($size:expr, $ret:expr) => {
        try_opt_warn!(
            qt::Image::new($size.width as u32, $size.height as u32),
            $ret,
            "Failed to create a {}x{} image.", $size.width, $size.height
        );
    };
}


mod clippath;
mod fill;
mod gradient;
mod image;
mod mask;
mod path;
mod pattern;
mod stroke;
mod text;

mod prelude {
    pub use super::super::prelude::*;
    pub type QtLayers = super::layers::Layers<super::qt::Image>;
}


type QtLayers = layers::Layers<qt::Image>;

impl ConvTransform<qt::Transform> for usvg::Transform {
    fn to_native(&self) -> qt::Transform {
        qt::Transform::new(self.a, self.b, self.c, self.d, self.e, self.f)
    }

    fn from_native(ts: &qt::Transform) -> Self {
        let d = ts.data();
        Self::new(d.0, d.1, d.2, d.3, d.4, d.5)
    }
}

impl TransformFromBBox for qt::Transform {
    fn from_bbox(bbox: Rect) -> Self {
        debug_assert!(!bbox.width.is_fuzzy_zero());
        debug_assert!(!bbox.height.is_fuzzy_zero());

        Self::new(bbox.width, 0.0, 0.0, bbox.height, bbox.x, bbox.y)
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
}

impl OutputImage for qt::Image {
    fn save(&self, path: &::std::path::Path) -> bool {
        self.save(path.to_str().unwrap())
    }
}

/// Renders SVG to image.
pub fn render_to_image(
    tree: &usvg::Tree,
    opt: &Options,
) -> Option<qt::Image> {
    let (img, img_size) = create_root_image(tree.svg_node().size.to_screen_size(), opt)?;

    let painter = qt::Painter::new(&img);
    render_to_canvas(tree, opt, img_size, &painter);
    painter.end();

    Some(img)
}

/// Renders SVG node to image.
pub fn render_node_to_image(
    node: &usvg::Node,
    opt: &Options,
) -> Option<qt::Image> {
    let node_bbox = if let Some(bbox) = calc_node_bbox(node, opt) {
        bbox
    } else {
        warn!("Node '{}' has zero size.", node.id());
        return None;
    };

    let vbox = usvg::ViewBox {
        rect: node_bbox,
        aspect: usvg::AspectRatio::default(),
    };

    let (img, img_size) = create_root_image(node_bbox.size().to_screen_size(), opt)?;

    let painter = qt::Painter::new(&img);
    render_node_to_canvas(node, opt, vbox, img_size, &painter);
    painter.end();

    Some(img)
}

/// Renders SVG to canvas.
pub fn render_to_canvas(
    tree: &usvg::Tree,
    opt: &Options,
    img_size: ScreenSize,
    painter: &qt::Painter,
) {
    render_node_to_canvas(&tree.root(), opt, tree.svg_node().view_box,
                          img_size, painter);
}

/// Renders SVG node to canvas.
pub fn render_node_to_canvas(
    node: &usvg::Node,
    opt: &Options,
    view_box: usvg::ViewBox,
    img_size: ScreenSize,
    painter: &qt::Painter,
) {
    let mut layers = create_layers(img_size, opt);

    apply_viewbox_transform(view_box, img_size, &painter);

    let curr_ts = painter.get_transform();
    let mut ts = utils::abs_transform(node);
    ts.append(&node.transform());

    painter.apply_transform(&ts.to_native());
    render_node(node, opt, &mut layers, painter);
    painter.set_transform(&curr_ts);
}

fn create_root_image(
    size: ScreenSize,
    opt: &Options,
) -> Option<(qt::Image, ScreenSize)> {
    let img_size = utils::fit_to(size, opt.fit_to);

    debug_assert_ne!(img_size.width, 0);
    debug_assert_ne!(img_size.height, 0);

    let mut img = try_create_image!(img_size, None);

    // Fill background.
    if let Some(c) = opt.background {
        img.fill(c.red, c.green, c.blue, 255);
    } else {
        img.fill(0, 0, 0, 0);
    }
    img.set_dpi(opt.usvg.dpi);

    Some((img, img_size))
}

/// Applies viewbox transformation to the painter.
fn apply_viewbox_transform(
    view_box: usvg::ViewBox,
    img_size: ScreenSize,
    painter: &qt::Painter,
) {
    let ts = utils::view_box_to_transform(view_box.rect, view_box.aspect, img_size.to_size());
    painter.apply_transform(&ts.to_native());
}

fn render_node(
    node: &usvg::Node,
    opt: &Options,
    layers: &mut QtLayers,
    p: &qt::Painter,
) -> Option<Rect> {
    match *node.borrow() {
        usvg::NodeKind::Svg(_) => {
            Some(render_group(node, opt, layers, p))
        }
        usvg::NodeKind::Path(ref path) => {
            Some(path::draw(&node.tree(), path, opt, p))
        }
        usvg::NodeKind::Text(ref text) => {
            Some(text::draw(&node.tree(), text, opt, p))
        }
        usvg::NodeKind::Image(ref img) => {
            Some(image::draw(img, opt, p))
        }
        usvg::NodeKind::Group(ref g) => {
            render_group_impl(node, g, opt, layers, p)
        }
        _ => None,
    }
}

fn render_group(
    parent: &usvg::Node,
    opt: &Options,
    layers: &mut QtLayers,
    p: &qt::Painter,
) -> Rect {
    let curr_ts = p.get_transform();
    let mut g_bbox = Rect::new_bbox();

    for node in parent.children() {
        p.apply_transform(&node.transform().to_native());

        let bbox = render_node(&node, opt, layers, p);
        if let Some(bbox) = bbox {
            g_bbox.expand(bbox);
        }

        // Revert transform.
        p.set_transform(&curr_ts);
    }

    g_bbox
}

fn render_group_impl(
    node: &usvg::Node,
    g: &usvg::Group,
    opt: &Options,
    layers: &mut QtLayers,
    p: &qt::Painter,
) -> Option<Rect> {
    let sub_img = layers.get()?;
    let sub_img = sub_img.borrow_mut();

    let sub_p = qt::Painter::new(&sub_img);
    sub_p.set_transform(&p.get_transform());

    let bbox = render_group(node, opt, layers, &sub_p);

    if let Some(ref id) = g.clip_path {
        if let Some(clip_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::ClipPath(ref cp) = *clip_node.borrow() {
                clippath::apply(&clip_node, cp, opt, bbox, layers, &sub_p);
            }
        }
    }

    if let Some(ref id) = g.mask {
        if let Some(mask_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::Mask(ref mask) = *mask_node.borrow() {
                mask::apply(&mask_node, mask, opt, bbox, layers, &sub_p, p);
            }
        }
    }

    sub_p.end();

    if let Some(opacity) = g.opacity {
        p.set_opacity(*opacity);
    }

    let curr_ts = p.get_transform();
    p.set_transform(&qt::Transform::default());

    p.draw_image(0.0, 0.0, &sub_img);

    p.set_opacity(1.0);
    p.set_transform(&curr_ts);

    Some(bbox)
}

/// Calculates node's absolute bounding box.
///
/// Note: this method can be pretty expensive.
pub fn calc_node_bbox(
    node: &usvg::Node,
    opt: &Options,
) -> Option<Rect> {
    // Unwrap can't fail, because `None` will be returned only on OOM,
    // and we cannot hit it with a such small image.
    let mut img = qt::Image::new(1, 1).unwrap();
    img.set_dpi(opt.usvg.dpi);
    let p = qt::Painter::new(&img);

    let abs_ts = utils::abs_transform(node);
    _calc_node_bbox(node, opt, abs_ts, &p)
}

fn _calc_node_bbox(
    node: &usvg::Node,
    opt: &Options,
    ts: usvg::Transform,
    p: &qt::Painter,
) -> Option<Rect> {
    let mut ts2 = ts;
    ts2.append(&node.transform());

    match *node.borrow() {
        usvg::NodeKind::Path(ref path) => {
            Some(utils::path_bbox(&path.segments, path.stroke.as_ref(), &ts2))
        }
        usvg::NodeKind::Text(ref text) => {
            let mut bbox = Rect::new_bbox();
            let mut fm = text::QtFontMetrics::new(p);

            text::draw_blocks(text, &mut fm, |block| {
                let mut p_path = qt::PainterPath::new();
                p_path.add_text(0.0, 0.0, &block.font, &block.text);

                let y = block.bbox.y + block.font_ascent;

                let mut t = ts2;
                if !block.rotate.is_fuzzy_zero() {
                    t.rotate_at(block.rotate, block.bbox.x, y);
                }
                t.translate(block.bbox.x, y);

                let segments = from_qt_path(&p_path);
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
                if let Some(c_bbox) = _calc_node_bbox(&child, opt, ts2, p) {
                    bbox.expand(c_bbox);
                }
            }

            Some(bbox)
        }
        _ => None
    }
}

fn from_qt_path(p_path: &qt::PainterPath) -> Vec<usvg::PathSegment> {
    let mut segments = Vec::with_capacity(p_path.len() as usize);
    let p_path_len = p_path.len();
    let mut i = 0;
    while i < p_path_len {
        let (kind, x, y) = p_path.get(i);
        match kind {
            qt::PathSegmentType::MoveToSegment => {
                segments.push(usvg::PathSegment::MoveTo { x, y });
            }
            qt::PathSegmentType::LineToSegment => {
                segments.push(usvg::PathSegment::LineTo { x, y });
            }
            qt::PathSegmentType::CurveToSegment => {
                let (_, x1, y1) = p_path.get(i + 1);
                let (_, x2, y2) = p_path.get(i + 2);

                segments.push(usvg::PathSegment::CurveTo { x1, y1, x2, y2, x, y });

                i += 2;
            }
        }

        i += 1;
    }

    if segments.len() < 2 {
        segments.clear();
    }

    segments
}

fn create_layers(img_size: ScreenSize, opt: &Options) -> QtLayers {
    layers::Layers::new(img_size, opt.usvg.dpi, create_subimage, clear_image)
}

fn create_subimage(
    size: ScreenSize,
    dpi: f64,
) -> Option<qt::Image> {
    let mut img = try_create_image!(size, None);

    img.fill(0, 0, 0, 0);
    img.set_dpi(dpi);

    Some(img)
}

fn clear_image(img: &mut qt::Image) {
    img.fill(0, 0, 0, 0);
}
