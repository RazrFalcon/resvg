// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use fontdb::{Database, ID};
use rustybuzz::ttf_parser;
use rustybuzz::ttf_parser::GlyphId;
use std::sync::Arc;
use tiny_skia_path::{NonZeroRect, Transform};

use crate::tree::BBox;
use crate::{Group, Node, Path, ShapeRendering, Text, TextRendering};

fn resolve_rendering_mode(text: &Text) -> ShapeRendering {
    match text.rendering_mode {
        TextRendering::OptimizeSpeed => ShapeRendering::CrispEdges,
        TextRendering::OptimizeLegibility => ShapeRendering::GeometricPrecision,
        TextRendering::GeometricPrecision => ShapeRendering::GeometricPrecision,
    }
}

pub(crate) fn flatten(text: &mut Text, fontdb: &fontdb::Database) -> Option<(Group, NonZeroRect)> {
    let mut new_paths = vec![];

    let mut stroke_bbox = BBox::default();
    let rendering_mode = resolve_rendering_mode(text);

    for span in &text.layouted {
        if let Some(path) = span.overline.as_ref() {
            stroke_bbox = stroke_bbox.expand(path.data.bounds());
            let mut path = path.clone();
            path.rendering_mode = rendering_mode;
            new_paths.push(path);
        }

        if let Some(path) = span.underline.as_ref() {
            stroke_bbox = stroke_bbox.expand(path.data.bounds());
            let mut path = path.clone();
            path.rendering_mode = rendering_mode;
            new_paths.push(path);
        }

        let mut span_builder = tiny_skia_path::PathBuilder::new();

        for glyph in &span.positioned_glyphs {
            if let Some(outline) = fontdb.outline(glyph.font, glyph.glyph_id) {
                if let Some(outline) = outline.transform(glyph.transform) {
                    span_builder.push_path(&outline);
                }
            }
        }

        if let Some(path) = span_builder.finish().and_then(|p| {
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
        }) {
            stroke_bbox = stroke_bbox.expand(path.stroke_bounding_box());
            new_paths.push(path);
        }

        if let Some(path) = span.line_through.as_ref() {
            stroke_bbox = stroke_bbox.expand(path.data.bounds());
            let mut path = path.clone();
            path.rendering_mode = rendering_mode;
            new_paths.push(path);
        }
    }

    let mut group = Group {
        id: text.id.clone(),
        ..Group::empty()
    };

    for path in new_paths {
        group.children.push(Node::Path(Box::new(path)));
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
}
