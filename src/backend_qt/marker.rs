// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use qt;

// self
use super::prelude::*;
use backend_utils::marker::*;


pub fn apply(
    tree: &usvg::Tree,
    path: &usvg::Path,
    opt: &Options,
    layers: &mut QtLayers,
    p: &mut qt::Painter,
) {
    let mut draw_marker = |id: &Option<String>, kind: MarkerKind| {
        if let Some(ref id) = id {
            if let Some(node) = tree.defs_by_id(id) {
                if let usvg::NodeKind::Marker(ref marker) = *node.borrow() {
                    _apply(path, marker, &node, kind, opt, layers, p);
                }
            }
        }
    };

    draw_marker(&path.marker.start, MarkerKind::Start);
    draw_marker(&path.marker.mid, MarkerKind::Middle);
    draw_marker(&path.marker.end, MarkerKind::End);
}

fn _apply(
    path: &usvg::Path,
    marker: &usvg::Marker,
    marker_node: &usvg::Node,
    marker_kind: MarkerKind,
    opt: &Options,
    layers: &mut QtLayers,
    p: &mut qt::Painter,
) {
    let stroke_scale = try_opt!(stroke_scale(marker, path), ());

    let r = marker.rect;
    debug_assert!(r.is_valid());

    let draw_marker = |x: f64, y: f64, idx: usize| {
        let old_ts = p.get_transform();
        p.translate(x, y);

        let angle = match marker.orientation {
            usvg::MarkerOrientation::Auto => calc_vertex_angle(&path.segments, idx),
            usvg::MarkerOrientation::Angle(angle) => angle,
        };

        if !angle.is_fuzzy_zero() {
            let ts = usvg::Transform::new_rotate(angle);
            p.apply_transform(&ts.to_native());
        }

        if let Some(vbox) = marker.view_box {
            let size = Size::new(r.width * stroke_scale, r.height * stroke_scale);
            let ts = utils::view_box_to_transform(vbox.rect, vbox.aspect, size);
            p.apply_transform(&ts.to_native());

            p.translate(vbox.rect.x, vbox.rect.y);
        } else {
            p.scale(stroke_scale, stroke_scale);
        }

        p.translate(-r.x, -r.y);

        match marker.overflow {
            usvg::Overflow::Hidden | usvg::Overflow::Scroll => {
                if let Some(vbox) = marker.view_box {
                    p.set_clip_rect(vbox.rect.x, vbox.rect.y, vbox.rect.width, vbox.rect.height);
                } else {
                    p.set_clip_rect(0.0, 0.0, r.width, r.height);
                }
            }
            _ => {}
        }

        super::render_group(marker_node, opt, layers, p);

        p.set_transform(&old_ts);
        p.reset_clip_path();
    };

    draw_markers(&path.segments, marker_kind, draw_marker);
}
