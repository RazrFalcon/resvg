use crate::{Font, FontProvider, FontStretch, FontStyle, ResolvedFont};
use fontdb::{Database, ID};
use rustybuzz::ttf_parser;
use std::num::NonZeroU16;
use svgtypes::FontFamily;

impl FontProvider for Database {
    fn resolve_font(&self, font: &Font) -> Option<ResolvedFont> {
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

        let id = self.query(&query);
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

        self.load_font(id?)
    }

    fn find_font_for_char(&self, c: char, exclude_fonts: &[ID]) -> Option<ResolvedFont> {
        let base_font_id = exclude_fonts[0];

        // Iterate over fonts and check if any of them support the specified char.
        for face in self.faces() {
            // Ignore fonts, that were used for shaping already.
            if exclude_fonts.contains(&face.id) {
                continue;
            }

            // Check that the new face has the same style.
            let base_face = self.face(base_font_id)?;
            if base_face.style != face.style
                && base_face.weight != face.weight
                && base_face.stretch != face.stretch
            {
                continue;
            }

            if !self.has_char(face.id, c) {
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
            return self.load_font(face.id);
        }

        None
    }

    fn fontdb(&self) -> &Database {
        &self
    }
}

pub(crate) trait DatabaseExt {
    fn load_font(&self, id: ID) -> Option<ResolvedFont>;
    fn has_char(&self, id: ID, c: char) -> bool;
}

impl DatabaseExt for Database {
    #[inline(never)]
    fn load_font(&self, id: ID) -> Option<ResolvedFont> {
        self.with_face_data(id, |data, face_index| -> Option<ResolvedFont> {
            let font = ttf_parser::Face::parse(data, face_index).ok()?;

            let units_per_em = NonZeroU16::new(font.units_per_em())?;

            let ascent = font.ascender();
            let descent = font.descender();

            let x_height = font
                .x_height()
                .and_then(|x| u16::try_from(x).ok())
                .and_then(NonZeroU16::new);
            let x_height = match x_height {
                Some(height) => height,
                None => {
                    // If not set - fallback to height * 45%.
                    // 45% is what Firefox uses.
                    u16::try_from((f32::from(ascent - descent) * 0.45) as i32)
                        .ok()
                        .and_then(NonZeroU16::new)?
                }
            };

            let line_through = font.strikeout_metrics();
            let line_through_position = match line_through {
                Some(metrics) => metrics.position,
                None => x_height.get() as i16 / 2,
            };

            let (underline_position, underline_thickness) = match font.underline_metrics() {
                Some(metrics) => {
                    let thickness = u16::try_from(metrics.thickness)
                        .ok()
                        .and_then(NonZeroU16::new)
                        // `ttf_parser` guarantees that units_per_em is >= 16
                        .unwrap_or_else(|| NonZeroU16::new(units_per_em.get() / 12).unwrap());

                    (metrics.position, thickness)
                }
                None => (
                    -(units_per_em.get() as i16) / 9,
                    NonZeroU16::new(units_per_em.get() / 12).unwrap(),
                ),
            };

            // 0.2 and 0.4 are generic offsets used by some applications (Inkscape/librsvg).
            let mut subscript_offset = (units_per_em.get() as f32 / 0.2).round() as i16;
            let mut superscript_offset = (units_per_em.get() as f32 / 0.4).round() as i16;
            if let Some(metrics) = font.subscript_metrics() {
                subscript_offset = metrics.y_offset;
            }

            if let Some(metrics) = font.superscript_metrics() {
                superscript_offset = metrics.y_offset;
            }

            Some(ResolvedFont {
                id,
                units_per_em,
                ascent,
                descent,
                x_height,
                underline_position,
                underline_thickness,
                line_through_position,
                subscript_offset,
                superscript_offset,
            })
        })?
    }

    #[inline(never)]
    fn has_char(&self, id: ID, c: char) -> bool {
        let res = self.with_face_data(id, |font_data, face_index| -> Option<bool> {
            let font = ttf_parser::Face::parse(font_data, face_index).ok()?;
            font.glyph_index(c)?;
            Some(true)
        });

        res == Some(Some(true))
    }
}
