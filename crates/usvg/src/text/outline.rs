use crate::text::layout::PositionedTextFragment;
use crate::text::old::DatabaseExt;
use crate::tree::BBox;
use crate::{Group, Node, Path, ShapeRendering, Text};
use std::sync::Arc;
use tiny_skia_path::{NonZeroRect, Transform};

pub(crate) fn convert(text: &mut Text, fontdb: &fontdb::Database) -> Option<()> {
    let mut new_paths = vec![];

    let mut bbox = BBox::default();
    let mut stroke_bbox = BBox::default();

    for span in &text.layouted {
        match span {
            PositionedTextFragment::Path(path) => {
                bbox = bbox.expand(path.data.bounds());
                stroke_bbox = stroke_bbox.expand(path.data.bounds());
                new_paths.push(path.clone());
            }
            PositionedTextFragment::Span(span) => {
                let mut span_builder = tiny_skia_path::PathBuilder::new();
                let mut bboxes_builder = tiny_skia_path::PathBuilder::new();

                for cluster in &span.glyph_clusters {
                    let mut cluster_builder = tiny_skia_path::PathBuilder::new();

                    for glyph in &cluster.glyphs {
                        if let Some(outline) = fontdb.outline(glyph.font, glyph.glyph_id) {
                            let mut ts = Transform::from_scale(1.0, -1.0);
                            ts = ts.pre_concat(glyph.transform);

                            if let Some(outline) = outline.transform(ts) {
                                cluster_builder.push_path(&outline);
                            }
                        }
                    }

                    if let Some(cluster_path) = cluster_builder.finish() {
                        if let Some(cluster_path) = cluster_path
                            .transform(cluster.path_transform)
                            .and_then(|p| p.transform(cluster.transform))
                        {
                            span_builder.push_path(&cluster_path);
                        }
                    }

                    let mut advance = cluster.advance;
                    if advance <= 0.0 {
                        advance = 1.0;
                    }

                    // We have to calculate text bbox using font metrics and not glyph shape.
                    if let Some(r) =
                        NonZeroRect::from_xywh(0.0, -cluster.ascent, advance, cluster.height())
                    {
                        if let Some(r) = r.transform(cluster.transform) {
                            bboxes_builder.push_rect(r.to_rect());
                        }
                    }
                }

                if let Some(span_path) = span_builder.finish() {
                    if let Some(span_path) = span_path.transform(span.transform) {
                        if let Some(path) = Path::new(
                            String::new(),
                            span.visibility,
                            span.fill.clone(),
                            span.stroke.clone(),
                            span.paint_order,
                            ShapeRendering::default(),
                            Arc::new(span_path),
                            Transform::default(),
                        ) {
                            stroke_bbox = stroke_bbox.expand(path.stroke_bounding_box());
                            new_paths.push(path);
                        }
                    }
                }

                if let Some(span_bbox) = bboxes_builder
                    .finish()
                    .and_then(|p| p.transform(span.transform))
                    .and_then(|p| p.compute_tight_bounds())
                    .and_then(|p| p.to_non_zero_rect())
                {
                    bbox = bbox.expand(span_bbox);
                }
            }
        }
    }

    // println!("{:?}", new_paths);

    let mut group = Group {
        id: text.id.clone(),
        ..Group::empty()
    };

    let rendering_mode = crate::text::old::resolve_rendering_mode(text);
    for mut path in new_paths {
        path.rendering_mode = rendering_mode;
        group.children.push(Node::Path(Box::new(path)));
    }

    group.calculate_bounding_boxes();
    text.flattened = Box::new(group);

    let bbox = bbox.to_non_zero_rect()?;
    let stroke_bbox = stroke_bbox.to_non_zero_rect()?;

    text.bounding_box = bbox.to_rect();
    text.abs_bounding_box = bbox.transform(text.abs_transform)?.to_rect();
    // TODO: test
    // TODO: should we stroke transformed paths?
    text.stroke_bounding_box = stroke_bbox.to_rect();
    text.abs_stroke_bounding_box = stroke_bbox.transform(text.abs_transform)?.to_rect();

    Some(())
}
