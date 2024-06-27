// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt::Debug;

use fontdb::{Database, ID};
use svgtypes::FontFamily;

use self::layout::DatabaseExt;
use crate::{Font, FontStretch, FontStyle, Text};

mod flatten;

mod colr;
/// Provides access to the layout of a text node.
pub mod layout;

/// A font resolver for `<text>` elements.
///
/// This type can be useful if you want to have an alternative font handling to
/// the default one. By default, only fonts specified upfront in
/// [`Options::fontdb`](crate::Options::fontdb) will be used. This type allows
/// you to load additional fonts on-demand and customize the font selection
/// process.
pub trait FontResolver: Debug + Sync {
    /// Provide access to the font [`Database`]
    fn fontdb(&self) -> &Database;

    /// Resolver function that will be used when selecting a specific font
    /// for a generic [`Font`] specification.
    ///
    /// This function receives a font specification (families + a style, weight,
    /// stretch triple) and a font database and should return the ID of the font
    /// that shall be used (if any).
    ///
    /// In the basic case, the function will search the existing fonts in the
    /// database to find a good match, e.g. via
    /// [`Database::query`](fontdb::Database::query). This is what the [default
    /// implementation](FontResolver::default_font_selector) does.
    ///
    /// Users with more complex requirements can mutate the database to load
    /// additional fonts dynamically. To perform mutation, it is recommended to call
    /// `Arc::make_mut` on the provided database. (This call is not done outside of
    /// the callback to not needless clone an underlying shared database if no
    /// mutation will be performed.) It is important that the database is only
    /// mutated additively. Removing fonts or replacing the entire database will
    /// break things.
    fn select_font(&self, font: &Font) -> Option<ID>;

    /// Resolver function that will be used when selecting a fallback font for a
    /// character.
    ///
    /// This function receives a specific character, a list of already used fonts,
    /// and a font database. It should return the ID of a font that
    /// - is not any of the already used fonts
    /// - is as close as possible to the first already used font (if any)
    /// - supports the given character
    ///
    /// The function can search the existing database, but can also load additional
    /// fonts dynamically. See the documentation of [`FontSelectionFn`] for more
    /// details.
    fn select_fallback(&self, c: char, exclude_fonts: &[ID]) -> Option<ID>;
}

/// Default font resolver
///
/// The default font selector forwards to
/// [`query`](fontdb::Database::query) on the font database specified in the
/// [`Options`](crate::Options).
///
/// The default fallback selector searches through the entire `fontdb`
/// to find a font that has the correct style and supports the character.
#[derive(Clone, Debug, Default)]
pub struct DefaultFontResolver {
    fontdb: Database,
}

impl DefaultFontResolver {
    /// Construct from an existing [`Database`]
    pub fn new(fontdb: Database) -> Self {
        DefaultFontResolver { fontdb }
    }

    /// Construct, loading system fonts
    #[cfg(feature = "system-fonts")]
    pub fn with_system_fonts() -> Self {
        let mut fontdb = fontdb::Database::new();
        fontdb.load_system_fonts();
        DefaultFontResolver { fontdb }
    }

    /// Deconstruct, taking the [`Database`]
    pub fn take_db(self) -> Database {
        self.fontdb
    }
}

impl FontResolver for DefaultFontResolver {
    fn fontdb(&self) -> &Database {
        &self.fontdb
    }

    fn select_font(&self, font: &Font) -> Option<ID> {
        let mut name_list = Vec::new();
        for family in &font.families {
            name_list.push(match family {
                FontFamily::Serif => fontdb::Family::Serif,
                FontFamily::SansSerif => fontdb::Family::SansSerif,
                FontFamily::Cursive => fontdb::Family::Cursive,
                FontFamily::Fantasy => fontdb::Family::Fantasy,
                FontFamily::Monospace => fontdb::Family::Monospace,
                FontFamily::Named(s) => fontdb::Family::Name(s),
            });
        }

        // Use the default font as fallback.
        name_list.push(fontdb::Family::Serif);

        let stretch = match font.stretch {
            FontStretch::UltraCondensed => fontdb::Stretch::UltraCondensed,
            FontStretch::ExtraCondensed => fontdb::Stretch::ExtraCondensed,
            FontStretch::Condensed => fontdb::Stretch::Condensed,
            FontStretch::SemiCondensed => fontdb::Stretch::SemiCondensed,
            FontStretch::Normal => fontdb::Stretch::Normal,
            FontStretch::SemiExpanded => fontdb::Stretch::SemiExpanded,
            FontStretch::Expanded => fontdb::Stretch::Expanded,
            FontStretch::ExtraExpanded => fontdb::Stretch::ExtraExpanded,
            FontStretch::UltraExpanded => fontdb::Stretch::UltraExpanded,
        };

        let style = match font.style {
            FontStyle::Normal => fontdb::Style::Normal,
            FontStyle::Italic => fontdb::Style::Italic,
            FontStyle::Oblique => fontdb::Style::Oblique,
        };

        let query = fontdb::Query {
            families: &name_list,
            weight: fontdb::Weight(font.weight),
            stretch,
            style,
        };

        let id = self.fontdb.query(&query);
        if id.is_none() {
            log::warn!(
                "No match for '{}' font-family.",
                font.families
                    .iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        id
    }

    fn select_fallback(&self, c: char, exclude_fonts: &[ID]) -> Option<ID> {
        let base_font_id = exclude_fonts[0];

        // Iterate over fonts and check if any of them support the specified char.
        for face in self.fontdb.faces() {
            // Ignore fonts, that were used for shaping already.
            if exclude_fonts.contains(&face.id) {
                continue;
            }

            // Check that the new face has the same style.
            let base_face = self.fontdb.face(base_font_id)?;
            if base_face.style != face.style
                && base_face.weight != face.weight
                && base_face.stretch != face.stretch
            {
                continue;
            }

            if !self.fontdb.has_char(face.id, c) {
                continue;
            }

            let base_family = base_face
                .families
                .iter()
                .find(|f| f.1 == fontdb::Language::English_UnitedStates)
                .unwrap_or(&base_face.families[0]);

            let new_family = face
                .families
                .iter()
                .find(|f| f.1 == fontdb::Language::English_UnitedStates)
                .unwrap_or(&base_face.families[0]);

            log::warn!("Fallback from {} to {}.", base_family.0, new_family.0);
            return Some(face.id);
        }

        None
    }
}

/// Convert a text into its paths. This is done in two steps:
/// 1. We convert the text into glyphs and position them according to the rules specified in the
/// SVG specifiation. While doing so, we also calculate the text bbox (which is not based on the
/// outlines of a glyph, but instead the glyph metrics as well as decoration spans).
/// 2. We convert all of the positioned glyphs into outlines.
pub(crate) fn convert(text: &mut Text, resolver: &dyn FontResolver) -> Option<()> {
    let (text_fragments, bbox) = layout::layout_text(text, resolver)?;
    text.layouted = text_fragments;
    text.bounding_box = bbox.to_rect();
    text.abs_bounding_box = bbox.transform(text.abs_transform)?.to_rect();

    let (group, stroke_bbox) = flatten::flatten(text, resolver.fontdb())?;
    text.flattened = Box::new(group);
    text.stroke_bounding_box = stroke_bbox.to_rect();
    text.abs_stroke_bounding_box = stroke_bbox.transform(text.abs_transform)?.to_rect();

    Some(())
}
