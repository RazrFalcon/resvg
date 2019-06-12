// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Qt backend implementation.

use crate::qt;
use log::warn;

use crate::{prelude::*, layers};
use crate::backend_utils::ConvTransform;


macro_rules! try_create_image {
    ($size:expr, $ret:expr) => {
        usvg::try_opt_warn_or!(
            qt::Image::new_rgba_premultiplied($size.width(), $size.height()),
            $ret,
            "Failed to create a {}x{} image.", $size.width(), $size.height()
        );
    };
}


mod clip_and_mask;
mod filter;
mod image;
mod path;
mod style;


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
    let mut layers = create_layers(img_size, opt);

    apply_viewbox_transform(view_box, img_size, painter);

    let curr_ts = painter.get_transform();

    let mut ts = node.abs_transform();
    ts.append(&node.transform());

    painter.apply_transform(&ts.to_native());
    render_node(node, opt, &mut layers, painter);
    painter.set_transform(&curr_ts);
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
    img.set_dpi(opt.usvg.dpi);

    Some((img, img_size))
}

/// Applies viewbox transformation to the painter.
fn apply_viewbox_transform(
    view_box: usvg::ViewBox,
    img_size: ScreenSize,
    painter: &mut qt::Painter,
) {
    let ts = utils::view_box_to_transform(view_box.rect, view_box.aspect, img_size.to_size());
    painter.apply_transform(&ts.to_native());
}

fn render_node(
    node: &usvg::Node,
    opt: &Options,
    layers: &mut QtLayers,
    p: &mut qt::Painter,
) -> Option<Rect> {
    match *node.borrow() {
        usvg::NodeKind::Svg(_) => {
            render_group(node, opt, layers, p)
        }
        usvg::NodeKind::Path(ref path) => {
            path::draw(&node.tree(), path, opt, p)
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
    p: &mut qt::Painter,
) -> Option<Rect> {
    let curr_ts = p.get_transform();
    let mut g_bbox = Rect::new_bbox();

    for node in parent.children() {
        p.apply_transform(&node.transform().to_native());

        let bbox = render_node(&node, opt, layers, p);
        if let Some(bbox) = bbox {
            if let Some(bbox) = bbox.transform(&node.transform()) {
                g_bbox = g_bbox.expand(bbox);
            }
        }

        // Revert transform.
        p.set_transform(&curr_ts);
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
    layers: &mut QtLayers,
    p: &mut qt::Painter,
) -> Option<Rect> {
    let sub_img = layers.get()?;
    let mut sub_img = sub_img.borrow_mut();

    let curr_ts = p.get_transform();

    let bbox = {
        let mut sub_p = qt::Painter::new(&mut sub_img);
        sub_p.set_transform(&curr_ts);

        render_group(node, opt, layers, &mut sub_p)
    };

    // Filter can be rendered on an object without a bbox,
    // as long as filter uses `userSpaceOnUse`.
    if let Some(ref id) = g.filter {
        if let Some(filter_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::Filter(ref filter) = *filter_node.borrow() {
                let ts = usvg::Transform::from_native(&curr_ts);
                filter::apply(filter, bbox, &ts, opt, &mut sub_img);
            }
        }
    }

    // Clipping and masking can be done only for objects with a valid bbox.
    if let Some(bbox) = bbox {
        if let Some(ref id) = g.clip_path {
            if let Some(clip_node) = node.tree().defs_by_id(id) {
                if let usvg::NodeKind::ClipPath(ref cp) = *clip_node.borrow() {
                    let mut sub_p = qt::Painter::new(&mut sub_img);
                    sub_p.set_transform(&curr_ts);

                    clip_and_mask::clip(&clip_node, cp, opt, bbox, layers, &mut sub_p);
                }
            }
        }

        if let Some(ref id) = g.mask {
            if let Some(mask_node) = node.tree().defs_by_id(id) {
                if let usvg::NodeKind::Mask(ref mask) = *mask_node.borrow() {
                    let mut sub_p = qt::Painter::new(&mut sub_img);
                    sub_p.set_transform(&curr_ts);

                    clip_and_mask::mask(&mask_node, mask, opt, bbox, layers, &mut sub_p);
                }
            }
        }
    }

    if !g.opacity.is_default() {
        p.set_opacity(g.opacity.value());
    }

    let curr_ts = p.get_transform();
    p.set_transform(&qt::Transform::default());

    p.draw_image(0.0, 0.0, &sub_img);

    p.set_opacity(1.0);
    p.set_transform(&curr_ts);

    bbox
}

fn create_layers(
    img_size: ScreenSize,
    opt: &Options,
) -> QtLayers {
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
