// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use usvg::try_opt;

use crate::prelude::*;
use crate::utils;

pub mod filter;
pub mod image;


#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum BlendMode {
    SourceOver,
    Clear,
    DestinationIn,
    DestinationOut,
    Xor,
}

impl Default for BlendMode {
    fn default() -> Self {
        BlendMode::SourceOver
    }
}


pub trait FlatRender {
    fn apply_viewbox(&mut self, view_box: usvg::ViewBox, img_size: ScreenSize) {
        let ts = utils::view_box_to_transform(view_box.rect, view_box.aspect, img_size.to_size());
        self.apply_transform(ts);
    }

    fn render_node(&mut self, node: &usvg::Node) -> Option<Rect> {
        match *node.borrow() {
            usvg::NodeKind::Svg(_) => {
                self.render_group(node)
            }
            usvg::NodeKind::Path(ref path) => {
                self.draw_path_impl(path)
            }
            usvg::NodeKind::Image(ref img) => {
                self.draw_image_impl(img)
            }
            usvg::NodeKind::Group(ref g) => {
                self.render_group_impl(node, g)
            }
            _ => None,
        }
    }

    fn render_group_impl(
        &mut self,
        node: &usvg::Node,
        g: &usvg::Group,
    ) -> Option<Rect> {
        let curr_ts = self.get_transform();
        self.push_layer()?;
        self.set_transform(curr_ts);

        let bbox = self.render_group(node);

        // Filter can be rendered on an object without a bbox,
        // as long as filter uses `userSpaceOnUse`.
        if let Some(ref id) = g.filter {
            if let Some(filter_node) = node.tree().defs_by_id(id) {
                if let usvg::NodeKind::Filter(ref filter) = *filter_node.borrow() {
                    self.filter(filter, bbox, curr_ts);
                }
            }
        }

        // Clipping and masking can be done only for objects with a valid bbox.
        if let Some(bbox) = bbox {
            if let Some(ref id) = g.clip_path {
                if let Some(ref clip_node) = node.tree().defs_by_id(id) {
                    if let usvg::NodeKind::ClipPath(ref cp) = *clip_node.borrow() {
                        self.clip(clip_node, cp, bbox, curr_ts);
                    }
                }
            }

            if let Some(ref id) = g.mask {
                if let Some(ref mask_node) = node.tree().defs_by_id(id) {
                    if let usvg::NodeKind::Mask(ref mask) = *mask_node.borrow() {
                        self.mask(mask_node, mask, bbox, curr_ts);
                    }
                }
            }
        }

        self.pop_layer(g.opacity, BlendMode::SourceOver);

        bbox
    }

    fn render_group(
        &mut self,
        parent: &usvg::Node,
    ) -> Option<Rect> {
        let curr_ts = self.get_transform();
        let mut g_bbox = Rect::new_bbox();

        for node in parent.children() {
            let mut ts = curr_ts;
            ts.append(&node.transform());
            self.set_transform(ts);

            let bbox = self.render_node(&node);
            if let Some(bbox) = bbox {
                if let Some(bbox) = bbox.transform(&node.transform()) {
                    g_bbox = g_bbox.expand(bbox);
                }
            }

            // Revert transform.
            self.set_transform(curr_ts);
        }

        // Check that bbox have changed, otherwise we will have a rect with x/y set to f64::MAX.
        if g_bbox.fuzzy_ne(&Rect::new_bbox()) {
            Some(g_bbox)
        } else {
            None
        }
    }

    fn draw_path_impl(&mut self, path: &usvg::Path) -> Option<Rect> {
        let bbox = utils::path_bbox(&path.segments, None, None);
        if path.visibility == usvg::Visibility::Visible {
            self.draw_path(path, bbox);
        }

        bbox
    }

    fn draw_image_impl(&mut self, image: &usvg::Image) -> Option<Rect> {
        let bbox = Some(image.view_box.rect);
        if image.visibility == usvg::Visibility::Visible {
            if image.format == usvg::ImageFormat::SVG {
                self.draw_svg_image(&image.data, image.view_box);
            } else {
                self.draw_raster_image(&image.data, image.view_box, image.rendering_mode);
            }

        }

        bbox
    }

    fn clip(
        &mut self,
        clip_node: &usvg::Node,
        clip: &usvg::ClipPath,
        bbox: Rect,
        ts: usvg::Transform,
    ) {
        try_opt!(self.push_layer());
        self.fill_layer(0, 0, 0, 255);

        self.set_transform(ts);
        self.apply_transform(clip.transform);
        if clip.units == usvg::Units::ObjectBoundingBox {
            self.apply_transform(usvg::Transform::from_bbox(bbox));
        }

        self.set_composition_mode(BlendMode::Clear);

        let ts2 = self.get_transform();
        for node in clip_node.children() {
            self.apply_transform(node.transform());

            match *node.borrow() {
                usvg::NodeKind::Path(ref path) => {
                    self.draw_path_impl(path);
                }
                usvg::NodeKind::Group(ref g) => {
                    self.clip_group(&node, g, bbox);
                }
                _ => {}
            }

            self.set_transform(ts2);
        }

        self.pop_layer(usvg::Opacity::default(), BlendMode::DestinationOut);

        if let Some(ref id) = clip.clip_path {
            if let Some(ref clip_node) = clip_node.tree().defs_by_id(id) {
                if let usvg::NodeKind::ClipPath(ref clip) = *clip_node.borrow() {
                    self.clip(clip_node, clip, bbox, ts);
                }
            }
        }
    }

    fn clip_group(
        &mut self,
        node: &usvg::Node,
        g: &usvg::Group,
        bbox: Rect,
    ) {
        if let Some(ref id) = g.clip_path {
            if let Some(ref clip_node) = node.tree().defs_by_id(id) {
                if let usvg::NodeKind::ClipPath(ref cp) = *clip_node.borrow() {
                    // If a `clipPath` child also has a `clip-path`
                    // then we should render this child on a new canvas,
                    // clip it, and only then draw it to the `clipPath`.

                    let ts = self.get_transform();
                    try_opt!(self.push_layer());
                    self.set_transform(ts);

                    if let Some(child) = node.first_child() {
                        self.apply_transform(child.transform());

                        if let usvg::NodeKind::Path(ref path) = *child.borrow() {
                            self.draw_path_impl(path);
                        }
                    }

                    self.clip(clip_node, cp, bbox, ts);
                    self.pop_layer(usvg::Opacity::default(), BlendMode::Xor);
                }
            }
        }
    }

    fn mask(
        &mut self,
        mask_node: &usvg::Node,
        mask: &usvg::Mask,
        bbox: Rect,
        ts: usvg::Transform,
    ) {
        try_opt!(self.push_layer());
        self.set_transform(ts);

        let r = if mask.units == usvg::Units::ObjectBoundingBox {
            mask.rect.bbox_transform(bbox)
        } else {
            mask.rect
        };

        self.set_clip_rect(r);

        if mask.content_units == usvg::Units::ObjectBoundingBox {
            self.apply_transform(usvg::Transform::from_bbox(bbox));
        }

        self.render_group(mask_node);
        self.apply_mask();
        self.pop_layer(usvg::Opacity::default(), BlendMode::DestinationIn);

        if let Some(ref id) = mask.mask {
            if let Some(ref mask_node) = mask_node.tree().defs_by_id(id) {
                if let usvg::NodeKind::Mask(ref mask) = *mask_node.borrow() {
                    self.mask(mask_node, mask, bbox, ts);
                }
            }
        }
    }

    fn apply_transform(&mut self, ts: usvg::Transform) {
        let mut ts2 = self.get_transform();
        ts2.append(&ts);
        self.set_transform(ts2);
    }

    fn reset_transform(&mut self) {
        self.set_transform(usvg::Transform::default());
    }

    fn draw_path(&mut self, path: &usvg::Path, bbox: Option<Rect>);
    fn draw_svg_image(&mut self, data: &usvg::ImageData, view_box: usvg::ViewBox);
    fn draw_raster_image(&mut self, data: &usvg::ImageData, view_box: usvg::ViewBox,
                         rendering_mode: usvg::ImageRendering);
    fn filter(&mut self, filter: &usvg::Filter, bbox: Option<Rect>, ts: usvg::Transform);
    fn fill_layer(&mut self, r: u8, g: u8, b: u8, a: u8);
    fn push_layer(&mut self) -> Option<()>;
    fn pop_layer(&mut self, opacity: usvg::Opacity, mode: BlendMode);
    fn apply_mask(&mut self);
    fn set_composition_mode(&mut self, mode: BlendMode);
    fn set_clip_rect(&mut self, rect: Rect);
    fn get_transform(&self) -> usvg::Transform;
    fn set_transform(&mut self, ts: usvg::Transform);
    fn finish(&mut self) {}
}


pub fn use_shape_antialiasing(
    mode: usvg::ShapeRendering,
) -> bool {
    match mode {
        usvg::ShapeRendering::OptimizeSpeed         => false,
        usvg::ShapeRendering::CrispEdges            => false,
        usvg::ShapeRendering::GeometricPrecision    => true,
    }
}

/// Converts an image to an alpha mask.
pub fn image_to_mask(
    data: &mut [u8],
    img_size: ScreenSize,
) {
    let width = img_size.width();
    let height = img_size.height();
    let stride = width * 4;

    let coeff_r = 0.2125 / 255.0;
    let coeff_g = 0.7154 / 255.0;
    let coeff_b = 0.0721 / 255.0;

    for y in 0..height {
        for x in 0..width {
            let idx = (y * stride + x * 4) as usize;

            let r = data[idx + 2] as f64;
            let g = data[idx + 1] as f64;
            let b = data[idx + 0] as f64;

            let luma = r * coeff_r + g * coeff_g + b * coeff_b;

            data[idx + 0] = 0;
            data[idx + 1] = 0;
            data[idx + 2] = 0;
            data[idx + 3] = f64_bound(0.0, luma * 255.0, 255.0) as u8;
        }
    }
}

pub trait ConvTransform<T> {
    fn to_native(&self) -> T;
    fn from_native(_: &T) -> Self;
}
