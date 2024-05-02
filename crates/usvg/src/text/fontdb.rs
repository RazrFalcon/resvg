use std::num::NonZeroU16;
use fontdb::{Database, ID};
use rustybuzz::ttf_parser;
use svgtypes::FontFamily;
use crate::{Font, FontProvider, FontStretch, FontStyle, ResolvedFont};

impl FontProvider<ID, FontDBResolvedFont> for Database {
    fn with_face_data<P, T>(&self, id: fontdb::ID, p: P) -> Option<T> where P: FnOnce(&[u8], u32) -> T {
        self.with_face_data(id, p)
    }

    fn resolve_font(&self, font: &Font) -> Option<FontDBResolvedFont> {
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

    fn find_font_for_char(&self, c: char, exclude_fonts: &[ID]) -> Option<FontDBResolvedFont> {
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
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct FontDBResolvedFont {
    pub id: ID,

    units_per_em: NonZeroU16,

    // All values below are in font units.
    ascent: i16,
    descent: i16,
    x_height: NonZeroU16,

    underline_position: i16,
    underline_thickness: NonZeroU16,

    // line-through thickness should be the the same as underline thickness
    // according to the TrueType spec:
    // https://docs.microsoft.com/en-us/typography/opentype/spec/os2#ystrikeoutsize
    line_through_position: i16,

    subscript_offset: i16,
    superscript_offset: i16,
}

impl ResolvedFont<ID> for FontDBResolvedFont {
    fn id(&self) -> ID {
        self.id
    }

    fn units_per_em(&self) -> NonZeroU16 {
        self.units_per_em
    }

    fn ascent(&self) -> i16 {
        self.ascent
    }

    fn descent(&self) -> i16 {
        self.descent
    }

    fn x_height(&self) -> NonZeroU16 {
        self.x_height
    }

    fn underline_position(&self) -> i16 {
        self.underline_position
    }

    fn underline_thickness(&self) -> NonZeroU16 {
       self.underline_thickness
    }

    fn line_through_position(&self) -> i16 {
        self.line_through_position
    }

    fn subscript_offset(&self) -> i16 {
        self.subscript_offset
    }

    fn superscript_offset(&self) -> i16 {
        self.superscript_offset
    }
}

pub(crate) trait DatabaseExt {
    fn load_font(&self, id: ID) -> Option<FontDBResolvedFont>;
    fn has_char(&self, id: ID, c: char) -> bool;
}

impl DatabaseExt for Database {
    #[inline(never)]
    fn load_font(&self, id: ID) -> Option<FontDBResolvedFont> {
        self.with_face_data(id, |data, face_index| -> Option<FontDBResolvedFont> {
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

            Some(FontDBResolvedFont {
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
