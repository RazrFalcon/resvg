// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgtypes::{Length, LengthUnit as Unit};

use crate::{NodeKind, OptionLog, Rect, Tree, Units, converter};
use crate::svgtree::{self, AId, EId};

/// A mask element.
///
/// `mask` element in SVG.
#[derive(Clone, Debug)]
pub struct Mask {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Can't be empty.
    pub id: String,

    /// Coordinate system units.
    ///
    /// `maskUnits` in SVG.
    pub units: Units,

    /// Content coordinate system units.
    ///
    /// `maskContentUnits` in SVG.
    pub content_units: Units,

    /// Mask rectangle.
    ///
    /// `x`, `y`, `width` and `height` in SVG.
    pub rect: Rect,

    /// Additional mask.
    ///
    /// `mask` in SVG.
    pub mask: Option<String>,
}

pub(crate) fn convert(
    node: svgtree::Node,
    state: &converter::State,
    id_generator: &mut converter::NodeIdGenerator,
    tree: &mut Tree,
) -> Option<String> {
    // A `mask` attribute must reference a `mask` element.
    if !node.has_tag_name(EId::Mask) {
        return None;
    }

    // Check if this element was already converted.
    if let Some(id) = node.attribute(AId::Id) {
        if tree.defs_by_id(id).is_some() {
            return Some(id.to_string());
        }
    }

    let units = node.attribute(AId::MaskUnits).unwrap_or(Units::ObjectBoundingBox);
    let content_units = node.attribute(AId::MaskContentUnits).unwrap_or(Units::UserSpaceOnUse);

    let rect = Rect::new(
        node.convert_length(AId::X, units, state, Length::new(-10.0, Unit::Percent)),
        node.convert_length(AId::Y, units, state, Length::new(-10.0, Unit::Percent)),
        node.convert_length(AId::Width, units, state, Length::new(120.0, Unit::Percent)),
        node.convert_length(AId::Height, units, state, Length::new(120.0, Unit::Percent)),
    );
    let rect = rect.log_none(|| log::warn!("Mask '{}' has an invalid size. Skipped.", node.element_id()))?;

    // Resolve linked mask.
    let mut mask = None;
    if let Some(link) = node.attribute::<svgtree::Node>(AId::Mask) {
        mask = convert(link, state, id_generator, tree);

        // Linked `mask` must be valid.
        if mask.is_none() {
            return None;
        }
    }

    let mut mask = tree.append_to_defs(NodeKind::Mask(Mask {
        id: node.element_id().to_string(),
        units,
        content_units,
        rect,
        mask,
    }));

    converter::convert_children(node, state, id_generator, &mut mask, tree);

    if mask.has_children() {
        Some(node.element_id().to_string())
    } else {
        // A mask without children is invalid.
        mask.detach();
        None
    }
}

