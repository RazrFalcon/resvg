// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::rc::Rc;
use std::str::FromStr;

use usvg_tree::{ClipPath, Group, Node, NodeKind, Transform, Units};

use crate::converter;
use crate::svgtree::{AId, EId, SvgNode};

pub(crate) fn convert(
    node: SvgNode,
    state: &converter::State,
    cache: &mut converter::Cache,
) -> Option<Rc<ClipPath>> {
    // A `clip-path` attribute must reference a `clipPath` element.
    if node.tag_name() != Some(EId::ClipPath) {
        return None;
    }

    // The whole clip path should be ignored when a transform is invalid.
    let transform = resolve_clip_path_transform(node, state)?;

    // Check if this element was already converted.
    if let Some(clip) = cache.clip_paths.get(node.element_id()) {
        return Some(clip.clone());
    }

    // Resolve linked clip path.
    let mut clip_path = None;
    if let Some(link) = node.attribute::<SvgNode>(AId::ClipPath) {
        clip_path = convert(link, state, cache);

        // Linked `clipPath` must be valid.
        if clip_path.is_none() {
            return None;
        }
    }

    let units = node
        .attribute(AId::ClipPathUnits)
        .unwrap_or(Units::UserSpaceOnUse);

    let mut clip = ClipPath {
        id: node.element_id().to_string(),
        units,
        transform,
        clip_path,
        root: Node::new(NodeKind::Group(Group::default())),
    };

    let mut clip_state = state.clone();
    clip_state.parent_clip_path = Some(node);
    converter::convert_clip_path_elements(node, &clip_state, cache, &mut clip.root);

    if clip.root.has_children() {
        let clip = Rc::new(clip);
        cache
            .clip_paths
            .insert(node.element_id().to_string(), clip.clone());
        Some(clip)
    } else {
        // A clip path without children is invalid.
        None
    }
}

fn resolve_clip_path_transform(node: SvgNode, state: &converter::State) -> Option<Transform> {
    // Do not use Node::attribute::<Transform>, because it will always
    // return a valid transform.

    let value: &str = match node.attribute(AId::Transform) {
        Some(v) => v,
        None => return Some(Transform::default()),
    };

    let ts = match svgtypes::Transform::from_str(value) {
        Ok(v) => v,
        Err(_) => {
            log::warn!("Failed to parse {} value: '{}'.", AId::Transform, value);
            return None;
        }
    };

    let ts = Transform::from_row(
        ts.a as f32,
        ts.b as f32,
        ts.c as f32,
        ts.d as f32,
        ts.e as f32,
        ts.f as f32,
    );

    if ts.is_valid() {
        Some(node.resolve_transform(AId::Transform, state))
    } else {
        None
    }
}
