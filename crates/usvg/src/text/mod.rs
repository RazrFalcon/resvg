// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::Text;

mod layout;
mod outline;

pub use layout::{PositionedGlyph, Span, TextFragment};

pub(crate) fn convert(text: &mut Text, fontdb: &fontdb::Database) -> Option<()> {
    let (text_fragments, bbox) = layout::layout_text(text, fontdb)?;

    text.layouted = text_fragments;
    text.bounding_box = bbox.to_rect();
    text.abs_bounding_box = bbox.transform(text.abs_transform)?.to_rect();

    let (group, stroke_bbox) = outline::convert(text, fontdb)?;

    text.flattened = Box::new(group);
    text.stroke_bounding_box = stroke_bbox.to_rect();
    text.abs_stroke_bounding_box = stroke_bbox.transform(text.abs_transform)?.to_rect();

    Some(())
}
