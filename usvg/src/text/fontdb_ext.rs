// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::convert::TryFrom;
use std::num::NonZeroU16;

use fontdb::{ID, Database};
use ttf_parser::GlyphId;

use crate::PathData;


pub trait DatabaseExt {
    fn load_font(&self, id: ID) -> Option<Font>;
    fn outline(&self, id: ID, glyph_id: GlyphId) -> Option<PathData>;
    fn has_char(&self, id: ID, c: char) -> bool;
}

impl DatabaseExt for Database {
    #[inline(never)]
    fn load_font(&self, id: ID) -> Option<Font> {
        self.with_face_data(id, |data, face_index| -> Option<Font> {
            let font = ttf_parser::Face::from_slice(data, face_index).ok()?;

            let units_per_em = NonZeroU16::new(font.units_per_em()?)?;

            let ascent = font.ascender();
            let descent = font.descender();

            let x_height = font.x_height().and_then(|x| u16::try_from(x).ok()).and_then(NonZeroU16::new);
            let x_height = match x_height {
                Some(height) => height,
                None => {
                    // If not set - fallback to height * 45%.
                    // 45% is what Firefox uses.
                    u16::try_from((f32::from(ascent - descent) * 0.45) as i32).ok()
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
                    let thickness = u16::try_from(metrics.thickness).ok()
                        .and_then(NonZeroU16::new)
                        // `ttf_parser` guarantees that units_per_em is >= 16
                        .unwrap_or_else(|| NonZeroU16::new(units_per_em.get() / 12).unwrap());

                    (metrics.position, thickness)
                }
                None => {
                    (
                        -(units_per_em.get() as i16) / 9,
                        NonZeroU16::new(units_per_em.get() / 12).unwrap(),
                    )
                }
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

            Some(Font {
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
    fn outline(&self, id: ID, glyph_id: GlyphId) -> Option<PathData> {
        self.with_face_data(id, |data, face_index| -> Option<PathData> {
            let font = ttf_parser::Face::from_slice(data, face_index).ok()?;

            let mut builder = PathBuilder { path: PathData::with_capacity(16) };
            font.outline_glyph(glyph_id, &mut builder)?;
            Some(builder.path)
        })?
    }

    #[inline(never)]
    fn has_char(&self, id: ID, c: char) -> bool {
        let res = self.with_face_data(id, |font_data, face_index| -> Option<bool> {
            let font = ttf_parser::Face::from_slice(font_data, face_index).ok()?;
            font.glyph_index(c)?;
            Some(true)
        });

       res == Some(Some(true))
    }
}


#[derive(Clone, Copy)]
pub struct Font {
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

impl Font {
    #[inline]
    pub fn scale(&self, font_size: f64) -> f64 {
        font_size / self.units_per_em.get() as f64
    }

    #[inline]
    pub fn ascent(&self, font_size: f64) -> f64 {
        self.ascent as f64 * self.scale(font_size)
    }

    #[inline]
    pub fn descent(&self, font_size: f64) -> f64 {
        self.descent as f64 * self.scale(font_size)
    }

    #[inline]
    pub fn height(&self, font_size: f64) -> f64 {
        self.ascent(font_size) - self.descent(font_size)
    }

    #[inline]
    pub fn x_height(&self, font_size: f64) -> f64 {
        self.x_height.get() as f64 * self.scale(font_size)
    }

    #[inline]
    pub fn underline_position(&self, font_size: f64) -> f64 {
        self.underline_position as f64 * self.scale(font_size)
    }

    #[inline]
    pub fn underline_thickness(&self, font_size: f64) -> f64 {
        self.underline_thickness.get() as f64 * self.scale(font_size)
    }

    #[inline]
    pub fn line_through_position(&self, font_size: f64) -> f64 {
        self.line_through_position as f64 * self.scale(font_size)
    }

    #[inline]
    pub fn subscript_offset(&self, font_size: f64) -> f64 {
        self.subscript_offset as f64 * self.scale(font_size)
    }

    #[inline]
    pub fn superscript_offset(&self, font_size: f64) -> f64 {
        self.superscript_offset as f64 * self.scale(font_size)
    }
}


struct PathBuilder {
    path: PathData,
}

impl ttf_parser::OutlineBuilder for PathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path.push_move_to(x as f64, y as f64);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.path.push_line_to(x as f64, y as f64);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.path.push_quad_to(
            x1 as f64, y1 as f64,
            x as f64, y as f64,
        );
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.path.push_curve_to(
            x1 as f64, y1 as f64,
            x2 as f64, y2 as f64,
            x as f64, y as f64
        );
    }

    fn close(&mut self) {
        self.path.push_close_path();
    }
}
