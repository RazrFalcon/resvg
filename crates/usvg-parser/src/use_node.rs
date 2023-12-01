// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::rc::Rc;

use svgtypes::{Length, LengthUnit};
use usvg_tree::{
    tiny_skia_path, Group, IsValidLength, Node, NodeExt, NodeKind, NonZeroRect, Path, Size,
    Transform,
};

use crate::converter;
use crate::svgtree::{AId, EId, SvgNode};

pub(crate) fn convert(
    node: SvgNode,
    state: &converter::State,
    cache: &mut converter::Cache,
    parent: &mut Node,
) -> Option<()> {
    let child = node.first_child()?;

    if state.parent_clip_path.is_some() && child.tag_name() == Some(EId::Symbol) {
        // Ignore `symbol` referenced by `use` inside a `clipPath`.
        // It will be ignored later anyway, but this will prevent
        // a redundant `clipPath` creation (which is required for `symbol`).
        return None;
    }

    // We require an original transformation to setup 'clipPath'.
    let mut orig_ts = node.resolve_transform(AId::Transform, state);
    let mut new_ts = Transform::default();

    {
        let x = node.convert_user_length(AId::X, state, Length::zero());
        let y = node.convert_user_length(AId::Y, state, Length::zero());
        new_ts = new_ts.pre_translate(x, y);
    }

    let linked_to_symbol = child.tag_name() == Some(EId::Symbol);

    if linked_to_symbol {
        if let Some(ts) = viewbox_transform(node, child, state) {
            new_ts = new_ts.pre_concat(ts);
        }

        if let Some(clip_rect) = get_clip_rect(node, child, state) {
            let mut g = clip_element(node, clip_rect, orig_ts, state, parent);

            // Make group for `use`.
            let mut parent = match converter::convert_group(node, state, true, cache, &mut g) {
                converter::GroupKind::Create(g) => {
                    // We must reset transform, because it was already set
                    // to the group with clip-path.
                    if let NodeKind::Group(ref mut g) = *g.borrow_mut() {
                        g.id = String::new(); // Prevent ID duplication.
                        g.transform = Transform::default();
                    }

                    g
                }
                converter::GroupKind::Skip => g.clone(),
                converter::GroupKind::Ignore => return None,
            };

            convert_children(child, new_ts, state, cache, &mut parent);
            return None;
        }
    }

    orig_ts = orig_ts.pre_concat(new_ts);

    if linked_to_symbol {
        // Make group for `use`.
        let mut parent = match converter::convert_group(node, state, false, cache, parent) {
            converter::GroupKind::Create(g) => {
                if let NodeKind::Group(ref mut g) = *g.borrow_mut() {
                    g.transform = Transform::default();
                }

                g
            }
            converter::GroupKind::Skip => parent.clone(),
            converter::GroupKind::Ignore => return None,
        };

        convert_children(child, orig_ts, state, cache, &mut parent);
    } else {
        let linked_to_svg = child.tag_name() == Some(EId::Svg);
        if linked_to_svg {
            // When a `use` element references a `svg` element,
            // we have to remember `use` element size and use it
            // instead of `svg` element size.

            let def = Length::new(100.0, LengthUnit::Percent);

            let mut state = state.clone();
            // As per usual, the SVG spec doesn't clarify this edge case,
            // but it seems like `use` size has to be reset by each `use`.
            // Meaning if we have two nested `use` elements, where one had set `width` and
            // other set `height`, we have to ignore the first `width`.
            //
            // Example:
            // <use id="use1" xlink:href="#use2" width="100"/>
            // <use id="use2" xlink:href="#svg2" height="100"/>
            // <svg id="svg2" x="40" y="40" width="80" height="80" xmlns="http://www.w3.org/2000/svg"/>
            //
            // In this case `svg2` size is 80x100 and not 100x100.
            state.use_size = (None, None);

            // Width and height can be set independently.
            if node.has_attribute(AId::Width) {
                state.use_size.0 = Some(node.convert_user_length(AId::Width, &state, def));
            }
            if node.has_attribute(AId::Height) {
                state.use_size.1 = Some(node.convert_user_length(AId::Height, &state, def));
            }

            convert_children(node, orig_ts, &state, cache, parent);
        } else {
            convert_children(node, orig_ts, state, cache, parent);
        }
    }

    Some(())
}

pub(crate) fn convert_svg(
    node: SvgNode,
    state: &converter::State,
    cache: &mut converter::Cache,
    parent: &mut Node,
) {
    // We require original transformation to setup 'clipPath'.
    let mut orig_ts = node.resolve_transform(AId::Transform, state);
    let mut new_ts = Transform::default();

    {
        let x = node.convert_user_length(AId::X, state, Length::zero());
        let y = node.convert_user_length(AId::Y, state, Length::zero());
        new_ts = new_ts.pre_translate(x, y);
    }

    if let Some(ts) = viewbox_transform(node, node, state) {
        new_ts = new_ts.pre_concat(ts);
    }

    // We have to create a new state which would have its viewBox set to the current SVG element.
    // Note that we're not updating State::size - it's a completely different property.
    let mut new_state = state.clone();
    new_state.view_box = {
        if let Some(vb) = node.parse_viewbox() {
            vb
        } else {
            // No `viewBox` attribute? Then use `x`, `y`, `width` and `height` instead.
            let x = node.convert_user_length(AId::X, &new_state, Length::zero());
            let y = node.convert_user_length(AId::Y, &new_state, Length::zero());
            let (mut w, mut h) = use_node_size(node, &new_state);

            // If attributes `width` and/or `height` are provided on the `use` element,
            // then these values will override the corresponding attributes
            // on the `svg` in the generated tree.
            w = new_state.use_size.0.unwrap_or(w);
            h = new_state.use_size.1.unwrap_or(h);

            NonZeroRect::from_xywh(x, y, w, h).unwrap_or(new_state.view_box)
        }
    };

    if let Some(clip_rect) = get_clip_rect(node, node, state) {
        let mut g = clip_element(node, clip_rect, orig_ts, state, parent);
        convert_children(node, new_ts, &new_state, cache, &mut g);
    } else {
        orig_ts = orig_ts.pre_concat(new_ts);
        convert_children(node, orig_ts, &new_state, cache, parent);
    }
}

fn clip_element(
    node: SvgNode,
    clip_rect: NonZeroRect,
    transform: Transform,
    state: &converter::State,
    parent: &mut Node,
) -> Node {
    // We can't set `clip-path` on the element itself,
    // because it will be affected by a possible transform.
    // So we have to create an additional group.

    // Emulate a new viewport via clipPath.
    //
    // From:
    // <defs/>
    // <elem/>
    //
    // To:
    // <defs>
    //   <clipPath id="clipPath1">
    //     <rect/>
    //   </clipPath>
    // </defs>
    // <g clip-path="ulr(#clipPath1)">
    //   <elem/>
    // </g>

    let clip_path = usvg_tree::ClipPath::default();

    let mut path = Path::new(Rc::new(tiny_skia_path::PathBuilder::from_rect(
        clip_rect.to_rect(),
    )));
    path.fill = Some(usvg_tree::Fill::default());
    clip_path.root.append_kind(NodeKind::Path(path));

    // Nodes generated by markers must not have an ID. Otherwise we would have duplicates.
    let id = if state.parent_markers.is_empty() {
        node.element_id().to_string()
    } else {
        String::new()
    };

    parent.append_kind(NodeKind::Group(Group {
        id,
        transform,
        clip_path: Some(Rc::new(clip_path)),
        ..Group::default()
    }))
}

fn convert_children(
    node: SvgNode,
    transform: Transform,
    state: &converter::State,
    cache: &mut converter::Cache,
    parent: &mut Node,
) {
    let required = !transform.is_identity();
    let mut parent = match converter::convert_group(node, state, required, cache, parent) {
        converter::GroupKind::Create(g) => {
            if let NodeKind::Group(ref mut g) = *g.borrow_mut() {
                g.transform = transform;
            }

            g
        }
        converter::GroupKind::Skip => parent.clone(),
        converter::GroupKind::Ignore => return,
    };

    if state.parent_clip_path.is_some() {
        converter::convert_clip_path_elements(node, state, cache, &mut parent);
    } else {
        converter::convert_children(node, state, cache, &mut parent);
    }
}

fn get_clip_rect(
    use_node: SvgNode,
    symbol_node: SvgNode,
    state: &converter::State,
) -> Option<NonZeroRect> {
    // No need to clip elements with overflow:visible.
    if matches!(
        symbol_node.attribute(AId::Overflow),
        Some("visible") | Some("auto")
    ) {
        return None;
    }

    // A nested `svg` with only the `viewBox` attribute and no "rectangle" (x, y, width, height)
    // should not be clipped.
    if use_node.tag_name() == Some(EId::Svg) {
        // Nested `svg` referenced by `use` still should be clipped, but by `use` bounds.
        if state.use_size.0.is_none() && state.use_size.1.is_none() {
            if !(use_node.has_attribute(AId::Width) && use_node.has_attribute(AId::Height)) {
                return None;
            }
        }
    }

    let (x, y, mut w, mut h) = {
        let x = use_node.convert_user_length(AId::X, state, Length::zero());
        let y = use_node.convert_user_length(AId::Y, state, Length::zero());
        let (w, h) = use_node_size(use_node, state);
        (x, y, w, h)
    };

    if use_node.tag_name() == Some(EId::Svg) {
        // If attributes `width` and/or `height` are provided on the `use` element,
        // then these values will override the corresponding attributes
        // on the `svg` in the generated tree.
        w = state.use_size.0.unwrap_or(w);
        h = state.use_size.1.unwrap_or(h);
    }

    if !w.is_valid_length() || !h.is_valid_length() {
        return None;
    }

    NonZeroRect::from_xywh(x, y, w, h)
}

fn use_node_size(node: SvgNode, state: &converter::State) -> (f32, f32) {
    let def = Length::new(100.0, LengthUnit::Percent);
    let w = node.convert_user_length(AId::Width, state, def);
    let h = node.convert_user_length(AId::Height, state, def);
    (w, h)
}

fn viewbox_transform(
    node: SvgNode,
    linked: SvgNode,
    state: &converter::State,
) -> Option<Transform> {
    let (mut w, mut h) = use_node_size(node, state);

    if node.tag_name() == Some(EId::Svg) {
        // If attributes `width` and/or `height` are provided on the `use` element,
        // then these values will override the corresponding attributes
        // on the `svg` in the generated tree.
        w = state.use_size.0.unwrap_or(w);
        h = state.use_size.1.unwrap_or(h);
    }

    let size = Size::from_wh(w, h)?;
    let vb = linked.parse_viewbox()?;
    let aspect = linked
        .attribute(AId::PreserveAspectRatio)
        .unwrap_or_default();

    Some(usvg_tree::utils::view_box_to_transform(vb, aspect, size))
}
