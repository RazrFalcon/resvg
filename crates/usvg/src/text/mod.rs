// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::num::NonZeroU16;
use crate::{Font, Text};

mod flatten;

/// Provides access to the layout of a text node.
pub mod layout;
mod fontdb;

/// Convert a text into its paths. This is done in two steps:
/// 1. We convert the text into glyphs and position them according to the rules specified in the
/// SVG specifiation. While doing so, we also calculate the text bbox (which is not based on the
/// outlines of a glyph, but instead the glyph metrics as well as decoration spans).
/// 2. We convert all of the positioned glyphs into outlines.
pub(crate) fn convert<T, RF: ResolvedFont<T>>(text: &mut Text, font_provider: &impl FontProvider<T, RF>) -> Option<()> {
    let (text_fragments, bbox) = layout::layout_text(text, font_provider)?;
    text.layouted = text_fragments;
    text.bounding_box = bbox.to_rect();
    text.abs_bounding_box = bbox.transform(text.abs_transform)?.to_rect();

    let (group, stroke_bbox) = flatten::flatten(text, font_provider)?;
    text.flattened = Box::new(group);
    text.stroke_bounding_box = stroke_bbox.to_rect();
    text.abs_stroke_bounding_box = stroke_bbox.transform(text.abs_transform)?.to_rect();

    Some(())
}

pub trait FontProvider<ID, RF: ResolvedFont<ID>> {
    fn with_face_data<P, T>(&self, id: ID, p: P) -> Option<T>
        where
            P: FnOnce(&[u8], u32) -> T;

    fn resolve_font(&self, font: &Font) -> Option<RF>;

    fn find_font_for_char(
        &self,
        c: char,
        exclude_fonts: &[ID],
    ) -> Option<RF>;
}

pub trait ResolvedFont<ID> {
    fn id(&self) -> ID;
    fn units_per_em(&self) -> NonZeroU16;
    fn ascent(&self) -> i16;
    fn descent(&self) -> i16;
    fn x_height(&self) -> NonZeroU16;
    fn underline_position(&self) -> i16;
    fn underline_thickness(&self) -> NonZeroU16;
    fn line_through_position(&self) -> i16;
    fn subscript_offset(&self) -> i16;
    fn superscript_offset(&self) -> i16;
}