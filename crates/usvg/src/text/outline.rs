use crate::text::layout::PositionedTextFragment;
use crate::text::old::DatabaseExt;
use crate::{Group, Node, Path, ShapeRendering, Text};
use std::sync::Arc;
use tiny_skia_path::Transform;

pub(crate) fn convert(text: &mut Text, fontdb: &fontdb::Database) -> Option<()> {
    let mut new_paths = vec![];
    for span in &text.layouted {
        match span {
            PositionedTextFragment::Path(path) => new_paths.push(path.clone()),
            PositionedTextFragment::Span(span) => {
                let mut span_builder = tiny_skia_path::PathBuilder::new();

                for cluster in &span.glyph_clusters {
                    let mut cluster_builder = tiny_skia_path::PathBuilder::new();

                    for glyph in &cluster.glyphs {
                        if let Some(outline) = fontdb.outline(span.font, glyph.glyph_id) {
                            let mut ts = Transform::from_scale(1.0, -1.0);
                            ts = ts.pre_concat(glyph.transform);

                            if let Some(outline) = outline.transform(ts) {
                                cluster_builder.push_path(&outline);
                            }
                        }
                    }

                    if let Some(cluster_path) = cluster_builder.finish() {
                        if let Some(cluster_path) = cluster_path.transform(cluster.transform) {
                            span_builder.push_path(&cluster_path);
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
                            new_paths.push(path);
                        }
                    }
                }
            }
        }
    }

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

    Some(())
}
