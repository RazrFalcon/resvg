// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use fontdb::{Database, ID};
use rustybuzz::ttf_parser;
use rustybuzz::ttf_parser::{GlyphId, RasterGlyphImage, RasterImageFormat};
use std::sync::Arc;
use tiny_skia_path::{NonZeroRect, Size, Transform};

use crate::tree::BBox;
use crate::{
    Group, Image, ImageKind, ImageRendering, Node, Path, ShapeRendering, Text, TextRendering,
    Visibility,
};

fn resolve_rendering_mode(text: &Text) -> ShapeRendering {
    match text.rendering_mode {
        TextRendering::OptimizeSpeed => ShapeRendering::CrispEdges,
        TextRendering::OptimizeLegibility => ShapeRendering::GeometricPrecision,
        TextRendering::GeometricPrecision => ShapeRendering::GeometricPrecision,
    }
}

pub(crate) fn flatten(text: &mut Text, fontdb: &fontdb::Database) -> Option<(Group, NonZeroRect)> {
    let mut new_children = vec![];

    let mut stroke_bbox = BBox::default();
    let rendering_mode = resolve_rendering_mode(text);

    for span in &text.layouted {
        if let Some(path) = span.overline.as_ref() {
            stroke_bbox = stroke_bbox.expand(path.data.bounds());
            let mut path = path.clone();
            path.rendering_mode = rendering_mode;
            new_children.push(Node::Path(Box::new(path)));
        }

        if let Some(path) = span.underline.as_ref() {
            stroke_bbox = stroke_bbox.expand(path.data.bounds());
            let mut path = path.clone();
            path.rendering_mode = rendering_mode;
            new_children.push(Node::Path(Box::new(path)));
        }


        for glyph in &span.positioned_glyphs {
            if let Some((raster, x, y, pixels_per_em, descender)) = fontdb.raster(glyph.font, glyph.glyph_id) {
                let mut group = Group {
                    transform: glyph.raster_transform(x, y, raster.size.height(), pixels_per_em, descender),
                    ..Group::empty()
                };
                group.children.push(Node::Image(Box::new(raster)));
                group.calculate_bounding_boxes();

                stroke_bbox = stroke_bbox.expand(group.stroke_bounding_box);
                new_children.push(Node::Group(Box::new(group)));
            } else if let Some(outline) = fontdb.outline(glyph.font, glyph.glyph_id) {
                if let Some(path) = outline.transform(glyph.outline_transform())
                    .and_then(|p| {
                        Path::new(
                            String::new(),
                            span.visibility,
                            span.fill.clone(),
                            span.stroke.clone(),
                            span.paint_order,
                            rendering_mode,
                            Arc::new(p),
                            Transform::default(),
                        )
                    }){
                    stroke_bbox = stroke_bbox.expand(path.stroke_bounding_box());
                    new_children.push(Node::Path(Box::new(path)));
                }
            }
        }

        if let Some(path) = span.line_through.as_ref() {
            stroke_bbox = stroke_bbox.expand(path.data.bounds());
            let mut path = path.clone();
            path.rendering_mode = rendering_mode;
            new_children.push(Node::Path(Box::new(path)));
        }
    }

    let mut group = Group {
        id: text.id.clone(),
        ..Group::empty()
    };

    for child in new_children {
        group.children.push(child);
    }

    group.calculate_bounding_boxes();
    Some((group, stroke_bbox.to_non_zero_rect()?))
}

struct PathBuilder {
    builder: tiny_skia_path::PathBuilder,
}

impl ttf_parser::OutlineBuilder for PathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.builder.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.builder.line_to(x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.builder.quad_to(x1, y1, x, y);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.builder.cubic_to(x1, y1, x2, y2, x, y);
    }

    fn close(&mut self) {
        self.builder.close();
    }
}

pub(crate) trait DatabaseExt {
    fn outline(&self, id: ID, glyph_id: GlyphId) -> Option<tiny_skia_path::Path>;
    fn raster(&self, id: ID, glyph_id: GlyphId) -> Option<(Image, i16, i16, u16, i16)>;
}

impl DatabaseExt for Database {
    #[inline(never)]
    fn outline(&self, id: ID, glyph_id: GlyphId) -> Option<tiny_skia_path::Path> {
        self.with_face_data(id, |data, face_index| -> Option<tiny_skia_path::Path> {
            let font = ttf_parser::Face::parse(data, face_index).ok()?;

            let mut builder = PathBuilder {
                builder: tiny_skia_path::PathBuilder::new(),
            };
            font.outline_glyph(glyph_id, &mut builder)?;
            builder.builder.finish()
        })?
    }

    fn raster(&self, id: ID, glyph_id: GlyphId) -> Option<(Image, i16, i16, u16, i16)> {
        self.with_face_data(id, |data, face_index| -> Option<(Image, i16, i16, u16, i16)> {
            let font = ttf_parser::Face::parse(data, face_index).ok()?;
            let image = font.glyph_raster_image(glyph_id, u16::MAX)?;

            println!("{:?}, {:?}, {:?},{:?}", font.ascender(), font.descender(), font.units_per_em(), font.glyph_bounding_box(glyph_id));
            println!("{:?}, {:?}, {:?}, {:?}, {:?}", image.x, image.y, image.width, image.height, image.pixels_per_em);

            if image.format == RasterImageFormat::PNG {
                return Some((Image {
                    id: String::new(),
                    visibility: Visibility::Visible,
                    size: Size::from_wh(image.width as f32, image.height as f32)?,
                    rendering_mode: ImageRendering::OptimizeQuality,
                    kind: ImageKind::PNG(Arc::new(image.data.into())),
                    abs_transform: Transform::default(),
                    abs_bounding_box: NonZeroRect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap(),
                }, image.x, image.y, image.pixels_per_em, font.descender()));
            }

            None
        })?
    }
}
