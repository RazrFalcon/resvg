// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom::{
    self,
    ElementType,
    FilterSvg,
    Length,
};

// self
use tree;
use tree::prelude::*;
use short::*;
use geom::*;
use {
    Error,
    Options,
};
pub use self::preprocess::prepare_doc;
pub use self::svgdom_ext::IsDefault;

mod clip_and_mask;
mod filter;
mod image;
mod marker;
mod paint_server;
mod path;
mod preprocess;
mod shapes;
mod style;
mod svgdom_ext;
mod switch;
mod text;
mod units;
mod use_node;

mod prelude {
    pub use svgdom::{
        AttributeType,
        ElementType,
        FilterSvg,
        FilterSvgAttrs,
        FilterSvgAttrsMut,
        FuzzyEq,
        FuzzyZero,
        Length,
    };
    pub use geom::*;
    pub use short::*;
    pub use Options;
    pub use super::svgdom_ext::*;
    pub use super::State;
}

use self::svgdom_ext::*;


#[derive(Clone)]
pub struct State<'a> {
    current_root: svgdom::Node,
    size: Size,
    view_box: Rect,
    opt: &'a Options,
}

impl<'a> State<'a> {
    pub fn is_in_clip_path(&self) -> bool {
        self.current_root.is_tag_name(EId::ClipPath)
    }
}


/// Converts an input `Document` into a `Tree`.
///
/// # Errors
///
/// - If `Document` doesn't have an SVG node - returns an empty tree.
/// - If `Document` doesn't have a valid size - returns `Error::InvalidSize`.
pub fn convert_doc(
    svg_doc: &svgdom::Document,
    opt: &Options,
) -> Result<tree::Tree, Error> {
    let svg = if let Some(svg) = svg_doc.svg_element() {
        svg
    } else {
        // Can be reached if 'preproc' module has a bug,
        // otherwise document will always have an svg node.
        //
        // Or if someone passed an invalid document directly though API.

        warn!("An invalid SVG structure. An empty tree will be produced.");
        return Ok(gen_empty_tree());
    };

    let size = resolve_svg_size(&svg, opt)?;

    let view_box = {
        let attrs = svg.attributes();
        tree::ViewBox {
            rect: get_view_box(&svg, size),
            aspect: convert_aspect(&attrs),
        }
    };

    if !style::is_visible_element(&svg, opt) {
        let svg_kind = tree::Svg {
            size,
            view_box,
        };

        return Ok(tree::Tree::create(svg_kind));
    }

    let svg_kind = tree::Svg {
        size,
        view_box,
    };

    let state = State {
        current_root: svg.clone(),
        size,
        view_box: view_box.rect,
        opt: &opt,
    };

    let mut tree = tree::Tree::create(svg_kind);

    convert_children(&svg, &state, &mut tree.root(), &mut tree);

    remove_empty_groups(&mut tree);
    ungroup_groups(&mut tree, opt);
    remove_unused_defs(&mut tree);

    Ok(tree)
}

fn resolve_svg_size(svg: &svgdom::Node, opt: &Options) -> Result<Size, Error> {
    let mut state = State {
        current_root: svg.clone(),
        size: Size::new(100.0, 100.0),
        view_box: Rect::new(0.0, 0.0, 100.0, 100.0),
        opt,
    };

    let def = Length::new(100.0, Unit::Percent);
    let width = svg.attributes().get_length(AId::Width).unwrap_or(def);
    let height = svg.attributes().get_length(AId::Height).unwrap_or(def);

    let view_box = svg.get_viewbox();

    if (width.unit == Unit::Percent || height.unit == Unit::Percent) && view_box.is_none() {
        // TODO: it this case we should detect the bounding box of all elements,
        //       which is currently impossible
        return Err(Error::InvalidSize);
    }

    let size = if let Some(vbox) = view_box {
        state.view_box = vbox;

        let w = if width.unit == Unit::Percent {
            vbox.width * (width.num / 100.0)
        } else {
            svg.convert_user_length(AId::Width, &state, def)
        };

        let h = if height.unit == Unit::Percent {
            vbox.height * (height.num / 100.0)
        } else {
            svg.convert_user_length(AId::Height, &state, def)
        };

        Size::new(w, h)
    } else {
        Size::new(
            svg.convert_user_length(AId::Width, &state, def),
            svg.convert_user_length(AId::Height, &state, def),
        )
    };

    if size.is_valid() {
        Ok(size)
    } else {
        Err(Error::InvalidSize)
    }
}

fn get_view_box(svg: &svgdom::Node, size: Size) -> Rect {
    match svg.get_viewbox() {
        Some(vb) => vb,
        None => size.to_rect(0.0, 0.0),
    }
}

fn gen_empty_tree() -> tree::Tree {
    let svg_kind = tree::Svg {
        size: Size::new(100.0, 100.0),
        view_box: tree::ViewBox {
            rect: (0.0, 0.0, 100.0, 100.0).into(),
            aspect: tree::AspectRatio::default(),
        },
    };

    tree::Tree::create(svg_kind)
}

fn convert_children(
    parent_node: &svgdom::Node,
    state: &State,
    parent: &mut tree::Node,
    tree: &mut tree::Tree,
) {
    for (_, node) in parent_node.children().svg() {
        convert_element(&node, state, parent, tree);
    }
}

fn convert_element(
    node: &svgdom::Node,
    state: &State,
    parent: &mut tree::Node,
    tree: &mut tree::Tree,
) {
    let eid = try_opt!(node.tag_id(), ());

    let is_valid_child =    node.is_graphic()
                         || eid == EId::G
                         || eid == EId::Switch
                         || eid == EId::Svg;
    if !is_valid_child {
        return;
    }

    if !style::is_visible_element(node, state.opt) {
        return;
    }

    match eid {
        EId::Rect |
        EId::Circle |
        EId::Ellipse |
        EId::Line |
        EId::Polyline |
        EId::Polygon |
        EId::Path => {
            if let Some(path) = shapes::convert(&node, state) {
                convert_path(&node, path, state, parent, tree);
            }
        }
        EId::Image => {
            image::convert(&node, state, parent);
        }
        EId::Text => {
            text::convert(&node, state, parent, tree);
        }
        EId::Switch => {
            switch::convert(&node, state, parent, tree);
        }
        EId::Svg => {
            use_node::convert_svg(&node, state, parent, tree);
        }
        EId::Use => {
            // Already resolved.
        }
        EId::G => {
            if node.has_attribute("usvg-use") {
                use_node::convert(&node, state, parent, tree);
            } else {
                match convert_group(&node, state, false, parent, tree) {
                    GroupKind::Keep(mut g) => {
                        convert_children(&node, state, &mut g, tree);
                    }
                    GroupKind::Skip => {
                        convert_children(&node, state, parent, tree);
                    }
                    GroupKind::Ignore => {}
                }
            }
        }
        _ => {},
    }
}

enum GroupKind {
    Keep(tree::Node),
    Skip,
    Ignore,
}

fn convert_group(
    node: &svgdom::Node,
    state: &State,
    force: bool,
    parent: &mut tree::Node,
    tree: &mut tree::Tree,
) -> GroupKind {
    // A `clipPath` child cannot have an opacity.
    let opacity = if !state.is_in_clip_path() {
        node.convert_opacity(AId::Opacity)
    } else {
        tree::Opacity::default()
    };

    macro_rules! resolve_link {
        ($aid:expr, $f:expr) => {{
            let mut v = None;
            if let Some(&AValue::FuncLink(ref link)) = node.attributes().get_value($aid) {
                v = $f(link, state, tree);

                // If `$aid` is linked to an invalid element - skip this group completely.
                if v.is_none() {
                    return GroupKind::Ignore;
                }
            }

            v
        }};
    }

    // `mask` and `filter` cannot be set on `clipPath` children.
    // But `clip-path` can.

    let clip_path = resolve_link!(AId::ClipPath, clip_and_mask::convert_clip);

    let mask = if !state.is_in_clip_path() {
        resolve_link!(AId::Mask, clip_and_mask::convert_mask)
    } else {
        None
    };


    let mut filter = None;
    if !state.is_in_clip_path() {
        match node.attributes().get_value(AId::Filter) {
            Some(&AValue::FuncLink(ref link)) => {
                filter = filter::convert(link, state, tree);

                // If `filter` is linked to an invalid element - skip this group completely.
                if filter.is_none() {
                    return GroupKind::Ignore;
                }
            }
            Some(&AValue::None) => {}
            Some(_) => {
                // Unlike `clip-path` and `mask`, when `filter` is invalid
                // than the whole element should be removed.
                return GroupKind::Ignore;
            }
            None => {}
        }
    }

    let required =    opacity.value().fuzzy_ne(&1.0)
                   || clip_path.is_some()
                   || mask.is_some()
                   || filter.is_some()
                   || !node.attributes().get_transform(AId::Transform).is_default()
                   || state.opt.keep_named_groups
                   || force;

    if required {
        let g = parent.append_kind(tree::NodeKind::Group(tree::Group {
            id: node.id().clone(),
            transform: node.attributes().get_transform(AId::Transform),
            opacity: node.convert_opacity(AId::Opacity),
            clip_path,
            mask,
            filter,
        }));

        GroupKind::Keep(g)
    } else {
        GroupKind::Skip
    }
}

fn remove_empty_groups(tree: &mut tree::Tree) {
    fn rm(parent: tree::Node) -> bool {
        let mut changed = false;

        let mut curr_node = parent.first_child();
        while let Some(mut node) = curr_node {
            curr_node = node.next_sibling();

            let is_g = if let tree::NodeKind::Group(ref g) = *node.borrow() {
                // Skip empty groups when they do not have a `filter` property.
                // The `filter` property can be set on empty groups. For example:
                //
                // <filter id="filter1" filterUnits="userSpaceOnUse"
                //         x="20" y="20" width="160" height="160">
                //   <feFlood flood-color="green"/>
                // </filter>
                // <g filter="url(#filter1)"/>
                g.filter.is_none()
            } else {
                false
            };

            if is_g && !node.has_children() {
                node.detach();
                changed = true;
            } else {
                if rm(node) {
                    changed = true;
                }
            }
        }

        changed
    }

    while rm(tree.root()) {}
}

fn ungroup_groups(tree: &mut tree::Tree, opt: &Options) {
    fn prepend_ts(ts1: &mut tree::Transform, mut ts2: tree::Transform) {
        ts2.append(ts1);
        *ts1 = ts2;
    }

    fn ungroup(parent: tree::Node, opt: &Options) -> bool {
        let mut changed = false;

        let mut curr_node = parent.first_child();
        while let Some(mut node) = curr_node {
            curr_node = node.next_sibling();

            let mut ts = tree::Transform::default();
            let is_ok = if let tree::NodeKind::Group(ref g) = *node.borrow() {
                ts = g.transform;

                   g.opacity.is_default()
                && g.clip_path.is_none()
                && g.mask.is_none()
                && g.filter.is_none()
                && !opt.keep_named_groups
            } else {
                false
            };

            if is_ok {
                let mut curr_child = node.last_child();
                while let Some(mut child) = curr_child {
                    curr_child = child.previous_sibling();

                    // Update transform.
                    match *child.borrow_mut() {
                        tree::NodeKind::Path(ref mut path) => {
                            prepend_ts(&mut path.transform, ts);
                        }
                        tree::NodeKind::Text(ref mut text) => {
                            prepend_ts(&mut text.transform, ts);
                        }
                        tree::NodeKind::Image(ref mut img) => {
                            prepend_ts(&mut img.transform, ts);
                        }
                        tree::NodeKind::Group(ref mut g) => {
                            prepend_ts(&mut g.transform, ts);
                        }
                        _ => {}
                    }

                    child.detach();
                    node.insert_after(child.clone());
                }

                node.detach();
                changed = true;
            } else {
                if ungroup(node, opt) {
                    changed = true;
                }
            }
        }

        changed
    }

    while ungroup(tree.root(), opt) {}
}

fn remove_unused_defs(tree: &mut tree::Tree) {
    macro_rules! check_id {
        ($from:expr, $id:expr) => {
            if let Some(ref id) = $from {
                if $id == id {
                    return true;
                }
            }
        };
    }

    macro_rules! check_paint_id {
        ($from:expr, $id:expr) => {
            if let Some(ref v) = $from {
                if let tree::Paint::Link(ref paint_id) = v.paint {
                    if $id == paint_id {
                        return true;
                    }
                }
            }
        };
    }

    fn is_used(tree: &tree::Tree, id: &str) -> bool {
        for node in tree.root().descendants() {
            match *node.borrow() {
                tree::NodeKind::ClipPath(ref clip) => {
                    check_id!(clip.clip_path, id);
                }
                tree::NodeKind::Mask(ref mask) => {
                    check_id!(mask.mask, id);
                }
                tree::NodeKind::Path(ref path) => {
                    check_paint_id!(path.fill, id);
                    check_paint_id!(path.stroke, id);
                }
                tree::NodeKind::Text(ref text) => {
                    for chunk in &text.chunks {
                        for span in &chunk.spans {
                            check_paint_id!(span.fill, id);
                            check_paint_id!(span.stroke, id);
                        }
                    }
                }
                tree::NodeKind::Group(ref g) => {
                    check_id!(g.clip_path, id);
                    check_id!(g.mask, id);
                    check_id!(g.filter, id);
                }
                _ => {}
            }
        }

        false
    }

    let mut is_changed = true;
    while is_changed {
        is_changed = false;

        let mut curr_node = tree.defs().first_child();
        while let Some(mut node) = curr_node {
            curr_node = node.next_sibling();

            if !is_used(tree, node.id().as_ref()) {
                node.detach();
                is_changed = true;
            }
        }
    }
}

fn convert_path(
    node: &svgdom::Node,
    segments: Vec<tree::PathSegment>,
    state: &State,
    parent: &mut tree::Node,
    tree: &mut tree::Tree,
) {
    debug_assert!(segments.len() >= 2);
    if segments.len() < 2 {
        return;
    }

    let has_bbox = path::has_bbox(&segments);
    let attrs = node.attributes();
    let fill = style::resolve_fill(node, has_bbox, state, tree);
    let stroke = style::resolve_stroke(node, has_bbox, state, tree);
    let transform = attrs.get_transform(AId::Transform);
    let mut visibility = convert_visibility(node);
    let rendering_mode = node.find_enum(AId::ShapeRendering)
                             .unwrap_or(state.opt.shape_rendering);

    // If a path doesn't have a fill or a stroke than it's invisible.
    // By setting `visibility` to `hidden` we are disabling the rendering of this path.
    if fill.is_none() && stroke.is_none() {
        visibility = tree::Visibility::Hidden
    }

    parent.append_kind(tree::NodeKind::Path(tree::Path {
        id: node.id().clone(),
        transform,
        visibility,
        fill,
        stroke,
        rendering_mode,
        segments: segments.clone(), // TODO: remove
    }));

    if visibility == tree::Visibility::Visible {
        marker::convert(node, &segments, state, parent, tree);
    }
}

fn convert_aspect(attrs: &svgdom::Attributes) -> tree::AspectRatio {
    let ratio: Option<&tree::AspectRatio> = attrs.get_type(AId::PreserveAspectRatio);
    match ratio {
        Some(v) => *v,
        None => {
            tree::AspectRatio {
                defer: false,
                align: tree::Align::XMidYMid,
                slice: false,
            }
        }
    }
}

fn convert_visibility(node: &svgdom::Node) -> tree::Visibility {
    node.find_str(AId::Visibility, "visible", |value| {
        match value {
            "hidden" =>   tree::Visibility::Hidden,
            "collapse" => tree::Visibility::Collapse,
            _ =>          tree::Visibility::Visible,
        }
    })
}

fn convert_rect(node: &svgdom::Node, state: &State) -> Rect {
    Rect::new(
        node.convert_user_length(AId::X, state, Length::zero()),
        node.convert_user_length(AId::Y, state, Length::zero()),
        node.convert_user_length(AId::Width, state, Length::zero()),
        node.convert_user_length(AId::Height, state, Length::zero()),
    )
}
