// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::rc::Rc;

mod convert;
mod shaper;
mod fontdb_ext;

use crate::{FillRule, Group, Node, NodeExt, NodeKind, Paint, Path, PathData, PathSegment, Rect};
use crate::{ShapeRendering, Stroke, StrokeWidth, Transform, TransformFromBBox, Tree, Units};
use crate::PathBbox;
use crate::{converter, svgtree};
use convert::{TextFlow, WritingMode, TextSpan};
use shaper::OutlinedCluster;
use convert::TextDecorationStyle;

mod private {
    use crate::svgtree::{self, EId};

    /// A type-safe container for a `text` node.
    ///
    /// This way we can be sure that we are passing the `text` node and not just a random node.
    #[derive(Clone, Copy)]
    pub struct TextNode<'a>(svgtree::Node<'a>);

    impl<'a> TextNode<'a> {
        pub fn new(node: svgtree::Node<'a>) -> Self {
            debug_assert!(node.has_tag_name(EId::Text));
            TextNode(node)
        }
    }

    impl<'a> std::ops::Deref for TextNode<'a> {
        type Target = svgtree::Node<'a>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
}
use private::TextNode;
use svgtypes::Color;


/// A text decoration span.
///
/// Basically a horizontal line, that will be used for underline, overline and line-through.
/// It doesn't have a height, since it depends on the font metrics.
#[derive(Clone, Copy)]
struct DecorationSpan {
    width: f64,
    transform: Transform,
}


pub(crate) fn convert(
    node: svgtree::Node,
    state: &converter::State,
    id_generator: &mut converter::NodeIdGenerator,
    parent: &mut Node,
    tree: &mut Tree,
) {
    let text_node = TextNode::new(node);
    let (mut new_paths, bbox) = text_to_paths(text_node, state, id_generator, parent, tree);

    if new_paths.len() == 1 {
        // Copy `text` id to the first path.
        new_paths[0].id = node.element_id().to_string();
    }

    let mut parent = if state.opt.keep_named_groups && new_paths.len() > 1 {
        // Create a group will all paths that was created during text-to-path conversion.
        parent.append_kind(NodeKind::Group(Group {
            id: node.element_id().to_string(),
            ..Group::default()
        }))
    } else {
        parent.clone()
    };

    let rendering_mode = convert::resolve_rendering_mode(text_node, state);
    for mut path in new_paths {
        fix_obj_bounding_box(&mut path, bbox, tree);
        path.rendering_mode = rendering_mode;
        parent.append_kind(NodeKind::Path(path));
    }
}

fn text_to_paths(
    text_node: TextNode,
    state: &converter::State,
    id_generator: &mut converter::NodeIdGenerator,
    parent: &mut Node,
    tree: &mut Tree,
) -> (Vec<Path>, PathBbox) {
    let abs_ts = {
        let mut ts = parent.abs_transform();
        ts.append(&text_node.attribute(svgtree::AId::Transform).unwrap_or_default());
        ts
    };

    let pos_list = convert::resolve_positions_list(text_node, state);
    let rotate_list = convert::resolve_rotate_list(text_node);
    let writing_mode = convert::convert_writing_mode(text_node);

    let mut bbox = PathBbox::new_bbox();
    let mut chunks = convert::collect_text_chunks(text_node, &pos_list, state, id_generator, tree);
    let mut char_offset = 0;
    let mut last_x = 0.0;
    let mut last_y = 0.0;
    let mut new_paths = Vec::new();
    for chunk in &mut chunks {
        let (x, y) = match chunk.text_flow {
            TextFlow::Horizontal => (chunk.x.unwrap_or(last_x), chunk.y.unwrap_or(last_y)),
            TextFlow::Path(_) => (0.0, 0.0),
        };

        let mut clusters = shaper::outline_chunk(chunk, state);
        if clusters.is_empty() {
            char_offset += chunk.text.chars().count();
            continue;
        }

        shaper::apply_writing_mode(writing_mode, &mut clusters);
        shaper::apply_letter_spacing(chunk, &mut clusters);
        shaper::apply_word_spacing(chunk, &mut clusters);
        let mut curr_pos = shaper::resolve_clusters_positions(
            chunk, char_offset, &pos_list, &rotate_list, writing_mode, abs_ts, &mut clusters
        );

        let mut text_ts = Transform::default();
        if writing_mode == WritingMode::TopToBottom {
            if let TextFlow::Horizontal = chunk.text_flow {
                text_ts.rotate_at(90.0, x, y);
            }
        }

        for span in &mut chunk.spans {
            let decoration_spans = collect_decoration_spans(span, &clusters);

            let mut span_ts = text_ts;
            span_ts.translate(x, y);
            if let TextFlow::Horizontal = chunk.text_flow {
                // In case of a horizontal flow, shift transform and not clusters,
                // because clusters can be rotated and an additional shift will lead
                // to invalid results.
                span_ts.translate(0.0, -span.baseline_shift);
            }

            if let Some(decoration) = span.decoration.underline.take() {
                // TODO: No idea what offset should be used for top-to-bottom layout.
                // There is
                // https://www.w3.org/TR/css-text-decor-3/#text-underline-position-property
                // but it doesn't go into details.
                let offset = match writing_mode {
                    WritingMode::LeftToRight => -span.font.underline_position(span.font_size),
                    WritingMode::TopToBottom => span.font.height(span.font_size) / 2.0,
                };

                let path = convert_decoration(
                    offset, span, decoration, &decoration_spans, span_ts,
                );

                if let Some(r) = path.data.bbox() {
                    bbox = bbox.expand(r);
                }

                new_paths.push(path);
            }

            if let Some(decoration) = span.decoration.overline.take() {
                let offset = match writing_mode {
                    WritingMode::LeftToRight => -span.font.ascent(span.font_size),
                    WritingMode::TopToBottom => -span.font.height(span.font_size) / 2.0,
                };

                let path = convert_decoration(
                    offset, span, decoration, &decoration_spans, span_ts,
                );

                if let Some(r) = path.data.bbox() {
                    bbox = bbox.expand(r);
                }

                new_paths.push(path);
            }

            if let Some(path) = convert_span(span, &mut clusters, &span_ts, parent, false) {
                // Use `text_bbox` here and not `path.data.bbox()`.
                if let Some(r) = path.text_bbox {
                    bbox = bbox.expand(r.to_path_bbox());
                }

                new_paths.push(path);
            }

            if let Some(decoration) = span.decoration.line_through.take() {
                let offset = match writing_mode {
                    WritingMode::LeftToRight => -span.font.line_through_position(span.font_size),
                    WritingMode::TopToBottom => 0.0,
                };

                let path = convert_decoration(
                    offset, span, decoration, &decoration_spans, span_ts,
                );

                if let Some(r) = path.data.bbox() {
                    bbox = bbox.expand(r);
                }

                new_paths.push(path);
            }
        }

        char_offset += chunk.text.chars().count();

        if writing_mode == WritingMode::TopToBottom {
            if let TextFlow::Horizontal = chunk.text_flow {
                std::mem::swap(&mut curr_pos.0, &mut curr_pos.1);
            }
        }

        last_x = x + curr_pos.0;
        last_y = y + curr_pos.1;
    }

    (new_paths, bbox)
}

fn convert_span(
    span: &mut TextSpan,
    clusters: &mut [OutlinedCluster],
    text_ts: &Transform,
    parent: &mut Node,
    dump_clusters: bool,
) -> Option<Path> {
    let mut path_data = PathData::new();
    let mut bboxes_data = PathData::new();

    for cluster in clusters {
        if !cluster.visible {
            continue;
        }

        if span.contains(cluster.byte_idx) {
            if dump_clusters {
                let mut ts = *text_ts;
                ts.append(&cluster.transform);
                dump_cluster(cluster, ts, parent);
            }

            let mut path = std::mem::replace(&mut cluster.path, PathData::new());
            path.transform(cluster.transform);

            path_data.extend_from_slice(&path);

            // We have to calculate text bbox using font metrics and not glyph shape.
            if let Some(r) = Rect::new(0.0, -cluster.ascent, cluster.advance, cluster.height()) {
                if let Some(r) = r.transform(&cluster.transform) {
                    bboxes_data.push_rect(r);
                }
            }
        }
    }

    if path_data.is_empty() {
        return None;
    }

    path_data.transform(*text_ts);
    bboxes_data.transform(*text_ts);

    let mut fill = span.fill.take();
    if let Some(ref mut fill) = fill {
        // The `fill-rule` should be ignored.
        // https://www.w3.org/TR/SVG2/text.html#TextRenderingOrder
        //
        // 'Since the fill-rule property does not apply to SVG text elements,
        // the specific order of the subpaths within the equivalent path does not matter.'
        fill.rule = FillRule::NonZero;
    }

    let path = Path {
        id: String::new(),
        transform: Transform::default(),
        visibility: span.visibility,
        fill,
        stroke: span.stroke.take(),
        rendering_mode: ShapeRendering::default(),
        text_bbox: bboxes_data.bbox().and_then(|r| r.to_rect()),
        data: Rc::new(path_data),
    };

    Some(path)
}

// Only for debug purposes.
fn dump_cluster(
    cluster: &OutlinedCluster,
    text_ts: Transform,
    parent: &mut Node,
) {
    fn new_stroke(color: Color) -> Option<Stroke> {
        Some(Stroke {
            paint: Paint::Color(color),
            width: StrokeWidth::new(0.2),
            .. Stroke::default()
        })
    }

    let mut base_path = Path {
        transform: text_ts,
        .. Path::default()
    };

    // Cluster bbox.
    let r = Rect::new(0.0, -cluster.ascent, cluster.advance, cluster.height()).unwrap();
    base_path.stroke = new_stroke(Color::blue());
    base_path.data = Rc::new(PathData::from_rect(r));
    parent.append_kind(NodeKind::Path(base_path.clone()));

    // Baseline.
    base_path.stroke = new_stroke(Color::red());
    base_path.data = Rc::new(PathData(vec![
        PathSegment::MoveTo { x: 0.0,             y: 0.0 },
        PathSegment::LineTo { x: cluster.advance, y: 0.0 },
    ]));
    parent.append_kind(NodeKind::Path(base_path));
}

fn collect_decoration_spans(
    span: &TextSpan,
    clusters: &[OutlinedCluster],
) -> Vec<DecorationSpan> {
    let mut spans = Vec::new();

    let mut started = false;
    let mut width = 0.0;
    let mut transform = Transform::default();
    for cluster in clusters {
        if span.contains(cluster.byte_idx) {
            if started && cluster.has_relative_shift {
                started = false;
                spans.push(DecorationSpan { width, transform });
            }

            if !started {
                width = cluster.advance;
                started = true;
                transform = cluster.transform;
            } else {
                width += cluster.advance;
            }
        } else if started {
            spans.push(DecorationSpan { width, transform });
            started = false;
        }
    }

    if started {
        spans.push(DecorationSpan { width, transform });
    }

    spans
}

fn convert_decoration(
    dy: f64,
    span: &TextSpan,
    mut decoration: TextDecorationStyle,
    decoration_spans: &[DecorationSpan],
    transform: Transform,
) -> Path {
    debug_assert!(!decoration_spans.is_empty());

    let thickness = span.font.underline_thickness(span.font_size);

    let mut path = PathData::new();
    for dec_span in decoration_spans {
        let rect = Rect::new(
            0.0,
            -thickness / 2.0,
            dec_span.width,
            thickness,
        ).unwrap();

        let start_idx = path.len();
        path.push_rect(rect);

        let mut ts = dec_span.transform;
        ts.translate(0.0, dy);
        path.transform_from(start_idx, ts);
    }

    path.transform(transform);

    Path {
        visibility: span.visibility,
        fill: decoration.fill.take(),
        stroke: decoration.stroke.take(),
        data: Rc::new(path),
        .. Path::default()
    }
}

/// By the SVG spec, `tspan` doesn't have a bbox and uses the parent `text` bbox.
/// Since we converted `text` and `tspan` to `path`, we have to update
/// all linked paint servers (gradients and patterns) too.
fn fix_obj_bounding_box(
    path: &mut Path,
    bbox: PathBbox,
    tree: &mut Tree,
) {
    if let Some(ref mut fill) = path.fill {
        if let Paint::Link(ref mut id) = fill.paint {
            if let Some(new_id) = paint_server_to_user_space_on_use(id, bbox, tree) {
                *id = new_id;
            }
        }
    }

    if let Some(ref mut stroke) = path.stroke {
        if let Paint::Link(ref mut id) = stroke.paint {
            if let Some(new_id) = paint_server_to_user_space_on_use(id, bbox, tree) {
                *id = new_id;
            }
        }
    }
}

/// Converts a selected paint server's units to `UserSpaceOnUse`.
///
/// Creates a deep copy of a selected paint server and returns its ID.
///
/// Returns `None` if a paint server already uses `UserSpaceOnUse`.
fn paint_server_to_user_space_on_use(
    id: &str,
    bbox: PathBbox,
    tree: &mut Tree,
) -> Option<String> {
    if let Some(mut ps) = tree.defs_by_id(id) {
        if ps.units() != Some(Units::ObjectBoundingBox) {
            return None;
        }

        // TODO: is `pattern` copying safe? Maybe we should reset id's on all `pattern` children.
        // We have to clone a paint server, in case some other element is already using it.
        // If not, the `convert` module will remove unused defs anyway.
        let mut new_ps = ps.make_deep_copy();
        tree.defs().append(new_ps.clone());

        let new_id = gen_paint_server_id(tree);

        // Update id, transform and units.
        let ts = Transform::from_bbox(bbox.to_rect()?);
        match *new_ps.borrow_mut() {
            NodeKind::LinearGradient(ref mut lg) => {
                lg.id = new_id.clone();
                lg.base.transform.prepend(&ts);
                lg.base.units = Units::UserSpaceOnUse;
            }
            NodeKind::RadialGradient(ref mut rg) => {
                rg.id = new_id.clone();
                rg.base.transform.prepend(&ts);
                rg.base.units = Units::UserSpaceOnUse;
            }
            NodeKind::Pattern(ref mut patt) => {
                patt.id = new_id.clone();
                patt.transform.prepend(&ts);
                patt.units = Units::UserSpaceOnUse;
            }
            _ => {}
        }

        Some(new_id)
    } else {
        None
    }
}

/// Creates a free id for a paint server.
fn gen_paint_server_id(tree: &Tree) -> String {
    // TODO: speed up

    let mut idx = 1;
    let mut id = format!("usvg{}", idx);
    while tree.defs().children().any(|n| *n.id() == id) {
        idx += 1;
        id = format!("usvg{}", idx);
    }

    id
}
