// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::rc::Rc;

use crate::{svgtree, tree, tree::prelude::*};
use super::prelude::*;

mod convert;
use self::convert::*;

mod shaper;
use self::shaper::OutlinedCluster;


mod private {
    use super::*;

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
use self::private::*;


/// A text decoration span.
///
/// Basically a horizontal line, that will be used for underline, overline and line-through.
/// It doesn't have a height, since it depends on the font metrics.
#[derive(Clone, Copy)]
struct DecorationSpan {
    width: f64,
    transform: tree::Transform,
}


pub fn convert(
    node: svgtree::Node,
    state: &State,
    parent: &mut tree::Node,
    tree: &mut tree::Tree,
) {
    state.db.borrow_mut().populate();

    let text_node = TextNode::new(node.clone());
    let mut new_paths = text_to_paths(text_node, state, parent, tree);

    let mut bbox = Rect::new_bbox();
    for path in &new_paths {
        if let Some(r) = path.data.bbox() {
            bbox = bbox.expand(r);
        }
    }

    if new_paths.len() == 1 {
        // Copy `text` id to the first path.
        new_paths[0].id = node.element_id().to_string();
    }

    let mut parent = if state.opt.keep_named_groups && new_paths.len() > 1 {
        // Create a group will all paths that was created during text-to-path conversion.
        parent.append_kind(tree::NodeKind::Group(tree::Group {
            id: node.element_id().to_string(),
            .. tree::Group::default()
        }))
    } else {
        parent.clone()
    };

    let rendering_mode = resolve_rendering_mode(text_node, state);
    for mut path in new_paths {
        fix_obj_bounding_box(&mut path, bbox, tree);
        path.rendering_mode = rendering_mode;
        parent.append_kind(tree::NodeKind::Path(path));
    }
}

fn text_to_paths(
    text_node: TextNode,
    state: &State,
    parent: &mut tree::Node,
    tree: &mut tree::Tree,
) -> Vec<tree::Path> {
    let pos_list = resolve_positions_list(text_node, state);
    let rotate_list = resolve_rotate_list(text_node);
    let writing_mode = convert_writing_mode(text_node);
    let mut text_ts = tree::Transform::default();

    let mut chunks = collect_text_chunks(text_node, &pos_list, state, tree);
    let mut char_offset = 0;
    let mut last_x = 0.0;
    let mut last_y = 0.0;
    let mut new_paths = Vec::new();
    for chunk in &mut chunks {
        let (x, y) = match chunk.text_flow {
            TextFlow::Horizontal => (chunk.x.unwrap_or(last_x), chunk.y.unwrap_or(last_y)),
            TextFlow::Path(_) => (0.0, 0.0),
        };

        let mut clusters = shaper::outline_chunk(&chunk, state);
        if clusters.is_empty() {
            char_offset += chunk.text.chars().count();
            continue;
        }

        shaper::apply_writing_mode(writing_mode, &mut clusters);
        shaper::apply_letter_spacing(&chunk, &mut clusters);
        shaper::apply_word_spacing(&chunk, &mut clusters);
        let curr_pos = shaper::resolve_clusters_positions(
            chunk, char_offset, &pos_list, &rotate_list, writing_mode, &mut clusters
        );

        if writing_mode == WritingMode::TopToBottom {
            if let TextFlow::Horizontal = chunk.text_flow {
                text_ts.rotate_at(90.0, x, y);
            }
        }

        for span in &mut chunk.spans {
            let decoration_spans = collect_decoration_spans(span, &clusters);

            let mut span_ts = text_ts.clone();
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

                new_paths.push(convert_decoration(
                    offset, &span, decoration, &decoration_spans, span_ts,
                ));
            }

            if let Some(decoration) = span.decoration.overline.take() {
                let offset = match writing_mode {
                    WritingMode::LeftToRight => -span.font.ascent(span.font_size),
                    WritingMode::TopToBottom => -span.font.height(span.font_size) / 2.0,
                };

                new_paths.push(convert_decoration(
                    offset, &span, decoration, &decoration_spans, span_ts,
                ));
            }

            if let Some(path) = convert_span(span, &mut clusters, &span_ts, parent, false) {
                new_paths.push(path);
            }

            if let Some(decoration) = span.decoration.line_through.take() {
                let offset = match writing_mode {
                    WritingMode::LeftToRight => -span.font.line_through_position(span.font_size),
                    WritingMode::TopToBottom => 0.0,
                };

                new_paths.push(convert_decoration(
                    offset, &span, decoration, &decoration_spans, span_ts,
                ));
            }
        }

        char_offset += chunk.text.chars().count();
        last_x = x + curr_pos.0;
        last_y = y + curr_pos.1;
    }

    new_paths
}

fn convert_span(
    span: &mut TextSpan,
    clusters: &mut [OutlinedCluster],
    text_ts: &tree::Transform,
    parent: &mut tree::Node,
    dump_clusters: bool,
) -> Option<tree::Path> {
    let mut path_data = tree::PathData::new();

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

            let mut path = std::mem::replace(&mut cluster.path, tree::PathData::new());
            path.transform(cluster.transform);

            path_data.extend_from_slice(&path);
        }
    }

    if path_data.is_empty() {
        return None;
    }

    path_data.transform(*text_ts);

    let mut fill = span.fill.take();
    if let Some(ref mut fill) = fill {
        // The `fill-rule` should be ignored.
        // https://www.w3.org/TR/SVG2/text.html#TextRenderingOrder
        //
        // 'Since the fill-rule property does not apply to SVG text elements,
        // the specific order of the subpaths within the equivalent path does not matter.'
        fill.rule = tree::FillRule::NonZero;
    }

    let path = tree::Path {
        id: String::new(),
        transform: tree::Transform::default(),
        visibility: span.visibility,
        fill,
        stroke: span.stroke.take(),
        rendering_mode: tree::ShapeRendering::default(),
        data: Rc::new(path_data),
    };

    Some(path)
}

// Only for debug purposes.
fn dump_cluster(
    cluster: &OutlinedCluster,
    text_ts: tree::Transform,
    parent: &mut tree::Node,
) {
    fn new_stroke(color: tree::Color) -> Option<tree::Stroke> {
        Some(tree::Stroke {
            paint: tree::Paint::Color(color),
            width: tree::StrokeWidth::new(0.2),
            .. tree::Stroke::default()
        })
    }

    let mut base_path = tree::Path {
        transform: text_ts,
        .. tree::Path::default()
    };

    // Cluster bbox.
    let r = Rect::new(0.0, -cluster.ascent, cluster.advance, cluster.height()).unwrap();
    base_path.stroke = new_stroke(tree::Color::blue());
    base_path.data = Rc::new(tree::PathData::from_rect(r));
    parent.append_kind(tree::NodeKind::Path(base_path.clone()));

    // Baseline.
    base_path.stroke = new_stroke(tree::Color::red());
    base_path.data = Rc::new(tree::PathData(vec![
        tree::PathSegment::MoveTo { x: 0.0,             y: 0.0 },
        tree::PathSegment::LineTo { x: cluster.advance, y: 0.0 },
    ]));
    parent.append_kind(tree::NodeKind::Path(base_path));
}

fn collect_decoration_spans(
    span: &TextSpan,
    clusters: &[OutlinedCluster],
) -> Vec<DecorationSpan> {
    let mut spans = Vec::new();

    let mut started = false;
    let mut width = 0.0;
    let mut transform = tree::Transform::default();
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
    transform: tree::Transform,
) -> tree::Path {
    debug_assert!(!decoration_spans.is_empty());

    let thickness = span.font.underline_thickness(span.font_size);

    let mut path = tree::PathData::new();
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

    tree::Path {
        visibility: span.visibility,
        fill: decoration.fill.take(),
        stroke: decoration.stroke.take(),
        data: Rc::new(path),
        .. tree::Path::default()
    }
}

/// By the SVG spec, `tspan` doesn't have a bbox and uses the parent `text` bbox.
/// Since we converted `text` and `tspan` to `path`, we have to update
/// all linked paint servers (gradients and patterns) too.
fn fix_obj_bounding_box(
    path: &mut tree::Path,
    bbox: Rect,
    tree: &mut tree::Tree,
) {
    if let Some(ref mut fill) = path.fill {
        if let tree::Paint::Link(ref mut id) = fill.paint {
            if let Some(new_id) = paint_server_to_user_space_on_use(id, bbox, tree) {
                *id = new_id;
            }
        }
    }

    if let Some(ref mut stroke) = path.stroke {
        if let tree::Paint::Link(ref mut id) = stroke.paint {
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
    bbox: Rect,
    tree: &mut tree::Tree,
) -> Option<String> {
    if let Some(ps) = tree.defs_by_id(id) {
        if ps.units() != Some(tree::Units::ObjectBoundingBox) {
            return None;
        }

        // TODO: is `pattern` copying safe? Maybe we should reset id's on all `pattern` children.
        // We have to clone a paint server, in case some other element is already using it.
        // If not, the `convert` module will remove unused defs anyway.
        let mut new_ps = ps.clone().make_deep_copy();
        tree.defs().append(new_ps.clone());

        let new_id = gen_paint_server_id(tree);

        // Update id, transform and units.
        let ts = tree::Transform::from_bbox(bbox);
        match *new_ps.borrow_mut() {
            tree::NodeKind::LinearGradient(ref mut lg) => {
                lg.id = new_id.clone();
                lg.base.transform.prepend(&ts);
                lg.base.units = tree::Units::UserSpaceOnUse;
            }
            tree::NodeKind::RadialGradient(ref mut rg) => {
                rg.id = new_id.clone();
                rg.base.transform.prepend(&ts);
                rg.base.units = tree::Units::UserSpaceOnUse;
            }
            tree::NodeKind::Pattern(ref mut patt) => {
                patt.id = new_id.clone();
                patt.transform.prepend(&ts);
                patt.units = tree::Units::UserSpaceOnUse;
            }
            _ => {}
        }

        Some(new_id)
    } else {
        None
    }
}

/// Creates a free id for a paint server.
fn gen_paint_server_id(
    tree: &tree::Tree,
) -> String {
    // TODO: speed up

    let mut idx = 1;
    let mut id = format!("usvg{}", idx);
    while tree.defs().children().any(|n| *n.id() == id) {
        idx += 1;
        id = format!("usvg{}", idx);
    }

    id
}
