// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{Font, Text};
use ::fontdb::{Database, ID};
use std::num::NonZeroU16;

mod flatten;

mod fontdb;
/// Provides access to the layout of a text node.
pub mod layout;

/// Convert a text into its paths. This is done in two steps:
/// 1. We convert the text into glyphs and position them according to the rules specified in the
/// SVG specifiation. While doing so, we also calculate the text bbox (which is not based on the
/// outlines of a glyph, but instead the glyph metrics as well as decoration spans).
/// 2. We convert all of the positioned glyphs into outlines.
pub(crate) fn convert(text: &mut Text, font_provider: &dyn FontProvider) -> Option<()> {
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

pub trait FontProvider {
    fn resolve_font(&self, font: &Font) -> Option<ResolvedFont>;

    fn find_font_for_char(&self, c: char, exclude_fonts: &[ID]) -> Option<ResolvedFont>;

    fn fontdb(&self) -> &Database;
}

#[derive(Clone, Copy, Debug)]
pub struct ResolvedFont {
    pub id: ID,
    pub units_per_em: NonZeroU16,
    pub ascent: i16,
    pub descent: i16,
    pub x_height: NonZeroU16,
    pub underline_position: i16,
    pub underline_thickness: NonZeroU16,
    pub line_through_position: i16,
    pub subscript_offset: i16,
    pub superscript_offset: i16,
}
