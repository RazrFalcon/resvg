// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::svgtree::{self, EId, AId};
use crate::{converter, Tree, NodeKind, Units, Transform};


/// A clip-path element.
///
/// `clipPath` element in SVG.
#[derive(Clone, Debug)]
pub struct ClipPath {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Can't be empty.
    pub id: String,

    /// Coordinate system units.
    ///
    /// `clipPathUnits` in SVG.
    pub units: Units,

    /// Clip path transform.
    ///
    /// `transform` in SVG.
    pub transform: Transform,

    /// Additional clip path.
    ///
    /// `clip-path` in SVG.
    pub clip_path: Option<String>,
}

impl Default for ClipPath {
    fn default() -> Self {
        ClipPath {
            id: String::new(),
            units: Units::UserSpaceOnUse,
            transform: Transform::default(),
            clip_path: None,
        }
    }
}


pub(crate) fn convert(
    node: svgtree::Node,
    state: &converter::State,
    id_generator: &mut converter::NodeIdGenerator,
    tree: &mut Tree,
) -> Option<String> {
    // A `clip-path` attribute must reference a `clipPath` element.
    if !node.has_tag_name(EId::ClipPath) {
        return None;
    }

    if !node.has_valid_transform(AId::Transform) {
        return None;
    }

    // Check if this element was already converted.
    if let Some(id) = node.attribute(AId::Id) {
        if tree.defs_by_id(id).is_some() {
            return Some(id.to_string());
        }
    }

    // Resolve linked clip path.
    let mut clip_path = None;
    if let Some(link) = node.attribute::<svgtree::Node>(AId::ClipPath) {
        clip_path = convert(link, state, id_generator, tree);

        // Linked `clipPath` must be valid.
        if clip_path.is_none() {
            return None;
        }
    }

    let units = node.attribute(AId::ClipPathUnits).unwrap_or(Units::UserSpaceOnUse);
    let mut clip = tree.append_to_defs(
        NodeKind::ClipPath(ClipPath {
            id: node.element_id().to_string(),
            units,
            transform: node.attribute(AId::Transform).unwrap_or_default(),
            clip_path,
        })
    );

    let mut clip_state = state.clone();
    clip_state.parent_clip_path = Some(node);
    converter::convert_clip_path_elements(node, &clip_state, id_generator, &mut clip, tree);

    if clip.has_children() {
        Some(node.element_id().to_string())
    } else {
        // A clip path without children is invalid.
        clip.detach();
        None
    }
}
