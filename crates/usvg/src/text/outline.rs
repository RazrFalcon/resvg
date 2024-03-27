use crate::text::layout::PositionedTextFragment;
use crate::text::old::DatabaseExt;
use crate::tree::BBox;
use crate::{Group, Node, Path, ShapeRendering, Text};
use std::sync::Arc;
use tiny_skia_path::Transform;

pub(crate) fn convert(text: &mut Text, fontdb: &fontdb::Database) -> Option<()> {
    let mut new_paths = vec![];

    let mut stroke_bbox = BBox::default();
    let rendering_mode = crate::text::old::resolve_rendering_mode(text);

    for span in &text.layouted {
        match span {
            PositionedTextFragment::Path(path) => {
                stroke_bbox = stroke_bbox.expand(path.data.bounds());
                let mut path = path.clone();
                path.rendering_mode = rendering_mode;
                new_paths.push(path);
            }
            PositionedTextFragment::Span(span) => {
                let mut span_builder = tiny_skia_path::PathBuilder::new();

                for cluster in &span.glyph_clusters {
                    for glyph in &cluster.glyphs {
                        if let Some(outline) = fontdb.outline(glyph.font, glyph.glyph_id) {
                            let mut ts = Transform::from_scale(1.0, -1.0);
                            ts = ts
                                .pre_concat(glyph.transform)
                                .post_concat(cluster.transform())
                                .post_concat(span.transform);

                            if let Some(outline) = outline.transform(ts) {
                                span_builder.push_path(&outline);
                            }
                        }
                    }
                }

                if let Some(path) = span_builder
                    .finish()
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
                    }) {
                    stroke_bbox = stroke_bbox.expand(path.stroke_bounding_box());
                    new_paths.push(path);
                }
            }
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
    text.flattened = Box::new(group);
    let stroke_bbox = stroke_bbox.to_non_zero_rect()?;

    // TODO: test
    // TODO: should we stroke transformed paths?
    text.stroke_bounding_box = stroke_bbox.to_rect();
    text.abs_stroke_bounding_box = stroke_bbox.transform(text.abs_transform)?.to_rect();

    Some(())
}
