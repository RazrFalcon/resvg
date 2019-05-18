// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom;
use harfbuzz;
use unicode_bidi;
use unicode_script;

mod fk {
    pub use font_kit::handle::Handle;
    pub use font_kit::hinting::HintingOptions as Hinting;
    pub use font_kit::source::SystemSource;
}

// self
use crate::tree;
use crate::convert::prelude::*;
use super::convert::{
    ByteIndex,
    CharacterPosition,
    Font,
    TextChunk,
};

type Range = std::ops::Range<usize>;


/// A glyph.
///
/// Basically, a glyph ID and it's metrics.
#[derive(Clone)]
struct Glyph {
    /// The glyph ID in the font.
    id: u32,

    /// Position in bytes in the original string.
    ///
    /// We use it to match a glyph with a character in the text chunk and therefore with the style.
    byte_idx: ByteIndex,

    /// The glyph offset in font units.
    dx: i32,

    /// The glyph offset in font units.
    dy: i32,

    /// The glyph width / X-advance in font units.
    width: i32,

    /// Reference to the source font.
    ///
    /// Each glyph can have it's own source font.
    font: Font,
}

impl Glyph {
    fn is_missing(&self) -> bool {
        self.id == 0
    }
}


/// An outlined cluster.
///
/// Cluster/grapheme is a single, unbroken, renderable character.
/// It can be positioned, rotated, spaced, etc.
///
/// Let's say we have `й` which is *CYRILLIC SMALL LETTER I* and *COMBINING BREVE*.
///
/// It consists of two code points, will be shaped (via harfbuzz) as two glyphs in one cluster,
/// and then will be combined into the one `OutlinedCluster`.
#[derive(Clone)]
pub struct OutlinedCluster {
    /// Position in bytes in the original string.
    ///
    /// We use it to match a cluster with a character in the text chunk and therefore with the style.
    pub byte_idx: ByteIndex,

    /// The cluster position in SVG coordinates.
    pub x: f64,

    /// The cluster position in SVG coordinates.
    pub y: f64,

    /// The rotation angle.
    pub rotate: f64,

    /// An advance along the X axis.
    ///
    /// Can be negative.
    pub advance: f64,

    /// Indicates that this cluster was affected by the relative shift (via dx/dy attributes)
    /// during the text layouting.
    ///
    /// Used during the `text-decoration` processing.
    pub has_relative_shift: bool,

    /// The actual outline.
    pub path: Vec<tree::PathSegment>,
}


/// An iterator over glyph clusters.
///
/// Input:  0 2 2 2 3 4 4 5 5
/// Result: 0 1     4 5   7
struct GlyphClusters<'a> {
    data: &'a [Glyph],
    idx: usize,
}

impl<'a> GlyphClusters<'a> {
    fn new(data: &'a [Glyph]) -> Self {
        GlyphClusters { data, idx: 0 }
    }
}

impl<'a> Iterator for GlyphClusters<'a> {
    type Item = (Range, ByteIndex);

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx == self.data.len() {
            return None;
        }

        let start = self.idx;
        let cluster = self.data[self.idx].byte_idx;
        for g in &self.data[self.idx..] {
            if g.byte_idx != cluster {
                break;
            }

            self.idx += 1;
        }

        Some((start..self.idx, cluster))
    }
}


/// Converts a text chunk into a list of outlined clusters.
///
/// This function will do the BIDI reordering, text shaping and glyphs outlining,
/// but not the text layouting. So all clusters are in the 0x0 position.
pub fn render_chunk(
    chunk: &TextChunk,
    state: &State,
) -> Vec<OutlinedCluster> {
    let mut glyphs = Vec::new();
    for span in &chunk.spans {
        let tmp_glyphs = shape_text(&chunk.text, &span.font, state);

        // Do nothing with the first run.
        if glyphs.is_empty() {
            glyphs = tmp_glyphs;
            continue;
        }

        // We assume, that shaping with an any font will produce the same amount of glyphs.
        // Otherwise an error.
        if glyphs.len() != tmp_glyphs.len() {
            warn!("Text layouting failed.");
            return Vec::new();
        }

        // Copy span's glyphs.
        for (i, glyph) in tmp_glyphs.iter().enumerate() {
            if span.contains(glyph.byte_idx) {
                glyphs[i] = glyph.clone();
            }
        }
    }

    // Convert glyphs to clusters.
    let mut clusters = Vec::new();
    for (range, byte_idx) in GlyphClusters::new(&glyphs) {
        if let Some(span) = chunk.span_at(byte_idx) {
            clusters.push(outline_cluster(&glyphs[range], span.font_size));
        }
    }

    clusters
}

/// Text shaping with font fallback.
fn shape_text(
    text: &str,
    font: &Font,
    state: &State,
) -> Vec<Glyph> {
    let mut glyphs = shape_text_with_font(text, font);

    // Remember all fonts used for shaping.
    let mut used_fonts = vec![font.clone()];

    let mut all_fonts = Vec::new();

    // Loop until all glyphs become resolved or until no more fonts are left.
    'outer: loop {
        let mut missing = None;
        for glyph in &glyphs {
            if glyph.is_missing() {
                missing = byte_to_char(text, glyph.byte_idx);
                break;
            }
        }

        if all_fonts.is_empty() {
            all_fonts = match fk::SystemSource::new().all_fonts() {
                Ok(v) => v,
                Err(_) => break 'outer,
            }
        }

        if let Some(c) = missing {
            let fallback_font = match find_font_for_char(c, &used_fonts, state) {
                Some(v) => v,
                None => break 'outer,
            };

            // Shape again, using a new font.
            let fallback_glyphs = shape_text_with_font(text, &fallback_font);

            if glyphs.len() != fallback_glyphs.len() {
                break 'outer;
            }

            // Copy new glyphs.
            for i in 0..glyphs.len() {
                if glyphs[i].is_missing() && !fallback_glyphs[i].is_missing() {
                    glyphs[i] = fallback_glyphs[i].clone();
                }
            }

            // Remember this font.
            used_fonts.push(fallback_font);
        } else {
            break 'outer;
        }
    }

    // Warn about missing glyphs.
    for glyph in &glyphs {
        if glyph.is_missing() {
            if let Some(c) = byte_to_char(text, glyph.byte_idx) {
                warn!("No fonts with a {}/U+{:X} character were found.", c, c as u32);
            }
        }
    }

    glyphs
}

/// Converts a text into a list of glyph IDs.
///
/// This function will do the BIDI reordering and text shaping.
fn shape_text_with_font(
    text: &str,
    font: &Font,
) -> Vec<Glyph> {
    let font_data = try_opt!(font.handle.copy_font_data(), Vec::new());
    let hb_face = harfbuzz::Face::from_bytes(&font_data, font.index);
    let hb_font = harfbuzz::Font::new(hb_face);

    let bidi_info = unicode_bidi::BidiInfo::new(text, Some(unicode_bidi::Level::ltr()));
    let paragraph = &bidi_info.paragraphs[0];
    let line = paragraph.range.clone();

    let mut glyphs = Vec::new();

    let (levels, runs) = bidi_info.visual_runs(&paragraph, line);
    for run in runs.iter() {
        let sub_text = &text[run.clone()];
        if sub_text.is_empty() {
            continue;
        }

        let hb_direction = if levels[run.start].is_rtl() {
            harfbuzz::Direction::Rtl
        } else {
            harfbuzz::Direction::Ltr
        };

        let buffer = harfbuzz::UnicodeBuffer::new()
            .add_str(sub_text)
            .set_direction(hb_direction);

        // TODO: feature smcp / small caps
        //       simply setting the `smcp` doesn't work for some reasons

        let output = harfbuzz::shape(&hb_font, buffer, &[]);

        let positions = output.get_glyph_positions();
        let infos = output.get_glyph_infos();

        for (pos, info) in positions.iter().zip(infos) {
            let idx = run.start + info.cluster as usize;
            debug_assert!(text.get(idx..).is_some());

            glyphs.push(Glyph {
                byte_idx: ByteIndex::new(idx),
                id: info.codepoint,
                dx: pos.x_offset,
                dy: pos.y_offset,
                width: pos.x_advance,
                font: font.clone(),
            });
        }
    }

    glyphs
}

/// Outlines a glyph cluster.
///
/// Uses one or more `Glyph`s to construct an `OutlinedCluster`.
fn outline_cluster(
    glyphs: &[Glyph],
    font_size: f64,
) -> OutlinedCluster {
    debug_assert!(!glyphs.is_empty());

    use lyon_path::builder::FlatPathBuilder;

    let mut path = Vec::new();
    let mut advance = 0.0;
    let mut x = 0.0;

    for glyph in glyphs {
        let mut builder = svgdom_path_builder::Builder::new();
        let mut outline = match glyph.font.handle.outline(glyph.id, fk::Hinting::None, &mut builder) {
            Ok(_) => {
                crate::convert::path::convert_path(builder.build())
            }
            Err(_) => {
                // Technically unreachable.
                warn!("Glyph {} not found in the font.", glyph.id);
                Vec::new()
            }
        };

        let sx = glyph.font.scale(font_size);

        if !outline.is_empty() {
            // By default, glyphs are upside-down, so we have to mirror them.
            let mut ts = svgdom::Transform::new_scale(1.0, -1.0);

            // Scale to font-size.
            ts.scale(sx, sx);

            // Apply offset.
            //
            // The first glyph in the cluster will have an offset from 0x0,
            // but the later one will have an offset from the "current position".
            // So we have to keep an advance.
            // TODO: vertical advance?
            // TODO: should be done only inside a single text span
            ts.translate(x + glyph.dx as f64, glyph.dy as f64);

            super::transform_path(&mut outline, &ts);

            path.extend_from_slice(&outline);
        }

        x += glyph.width as f64;

        let glyph_width = glyph.width as f64 * sx;
        if glyph_width > advance {
            advance = glyph_width;
        }
    }

    OutlinedCluster {
        byte_idx: glyphs[0].byte_idx,
        x: 0.0,
        y: 0.0,
        advance,
        rotate: 0.0,
        has_relative_shift: false,
        path,
    }
}

/// Finds a font with a specified char.
///
/// This is a rudimentary font fallback algorithm.
fn find_font_for_char(
    c: char,
    exclude_fonts: &[Font],
    state: &State,
) -> Option<Font> {
    let mut cache = state.font_cache.borrow_mut();
    cache.init();

    // Iterate over fonts and check if any of the support the specified char.
    for handle in cache.fonts() {
        let (path, index) = match handle {
            fk::Handle::Path { ref path, font_index } => {
                (path, *font_index)
            }
            _ => continue,
        };

        // Ignore fonts, that were used for shaping already.
        let exclude = exclude_fonts
            .iter()
            .find(|f| f.path == *path && f.index == index)
            .is_some();

        if exclude {
            continue;
        }

        // TODO: match font style too

        if let Some(font) = super::load_font(handle) {
            if font.handle.glyph_for_char(c).is_some() {
                return Some(font);
            }
        }
    }

    None
}

pub fn resolve_clusters_positions(
    text: &str,
    offset: usize,
    pos_list: &[CharacterPosition],
    rotate_list: &[f64],
    clusters: &mut Vec<OutlinedCluster>,
) {
    let mut x = 0.0;
    let mut y = 0.0;

    for cluster in clusters {
        cluster.x = x;
        cluster.y = y;

        let cp = offset + byte_to_code_point(text, cluster.byte_idx);
        if let Some(pos) = pos_list.get(cp) {
            cluster.x += pos.dx.unwrap_or(0.0);
            cluster.y += pos.dy.unwrap_or(0.0);
            cluster.has_relative_shift = pos.dx.is_some() || pos.dy.is_some();
        }

        if let Some(angle) = rotate_list.get(cp).cloned() {
            cluster.rotate = angle;
        }

        x = cluster.x + cluster.advance;
        y = cluster.y;
    }
}

/// Applies the `letter-spacing` property to a text chunk clusters.
///
/// [In the CSS spec](https://www.w3.org/TR/css-text-3/#letter-spacing-property).
pub fn apply_letter_spacing(
    chunk: &TextChunk,
    clusters: &mut Vec<OutlinedCluster>,
) {
    // At least one span should have a non-zero spacing.
    if !chunk.spans.iter().any(|span| !span.letter_spacing.is_fuzzy_zero()) {
        return;
    }

    for cluster in clusters {
        if let Some(c) = byte_to_char(&chunk.text, cluster.byte_idx) {
            // Spacing must be applied only to characters that belongs to the script
            // that supports spacing.
            // We are checking only the first code point, since it should be enough.
            let script = unicode_script::get_script(c);
            if script_supports_letter_spacing(script) {
                if let Some(span) = chunk.span_at(cluster.byte_idx) {
                    // Technically, we should ignore spacing on the last character,
                    // but it doesn't affect us in any way, so we are ignoring this.
                    cluster.advance += span.letter_spacing;

                    // If the cluster advance became negative - clear it.
                    // This is an UB and we can do whatever we want, so we mimic the Chrome behavior.
                    if !(cluster.advance > 0.0) {
                        cluster.advance = 0.0;
                        cluster.path.clear();
                    }
                }
            }
        }
    }
}

/// Checks that selected script supports letter spacing.
///
/// [In the CSS spec](https://www.w3.org/TR/css-text-3/#cursive-tracking).
///
/// The list itself is from: https://github.com/harfbuzz/harfbuzz/issues/64
fn script_supports_letter_spacing(script: unicode_script::Script) -> bool {
    use unicode_script::Script;

    match script {
          Script::Arabic
        | Script::Syriac
        | Script::Nko
        | Script::Manichaean
        | Script::Psalter_Pahlavi
        | Script::Mandaic
        | Script::Mongolian
        | Script::Phags_Pa
        | Script::Devanagari
        | Script::Bengali
        | Script::Gurmukhi
        | Script::Modi
        | Script::Sharada
        | Script::Syloti_Nagri
        | Script::Tirhuta
        | Script::Ogham => false,
        _ => true,
    }
}

/// Applies the `word-spacing` property to a text chunk clusters.
///
/// [In the CSS spec](https://www.w3.org/TR/css-text-3/#propdef-word-spacing).
pub fn apply_word_spacing(
    chunk: &TextChunk,
    clusters: &mut Vec<OutlinedCluster>,
) {
    // At least one span should have a non-zero spacing.
    if !chunk.spans.iter().any(|span| !span.word_spacing.is_fuzzy_zero()) {
        return;
    }

    for cluster in clusters {
        if let Some(c) = byte_to_char(&chunk.text, cluster.byte_idx) {
            if is_word_separator_characters(c) {
                if let Some(span) = chunk.span_at(cluster.byte_idx) {
                    // Technically, word spacing 'should be applied half on each
                    // side of the character', but it doesn't affect us in any way,
                    // so we are ignoring this.
                    cluster.advance += span.word_spacing;

                    // After word spacing, `advance` can be negative.
                }
            }
        }
    }
}

/// Checks that the selected character is a word separator.
///
/// According to: https://www.w3.org/TR/css-text-3/#word-separator
fn is_word_separator_characters(c: char) -> bool {
    match c as u32 {
        0x0020 | 0x00A0 | 0x1361 | 0x010100 | 0x010101 | 0x01039F | 0x01091F => true,
        _ => false,
    }
}

/// Converts byte position into a code point position.
fn byte_to_code_point(text: &str, byte: ByteIndex) -> usize {
    text.char_indices().take_while(|(i, _)| *i != byte.value()).count()
}

/// Converts byte position into a character.
fn byte_to_char(text: &str, byte: ByteIndex) -> Option<char> {
    text[byte.value()..].chars().next()
}

/// Implements an ability to outline a glyph directly into the `svgdom::Path`.
mod svgdom_path_builder {
    use lyon_geom::math::*;
    use lyon_path::builder::{FlatPathBuilder, PathBuilder};

    pub struct Builder {
        path: svgdom::Path,
        current_position: Point,
        first_position: Point,
    }

    impl Builder {
        pub fn new() -> Self {
            Builder {
                path: svgdom::Path::new(),
                current_position: Point::new(0.0, 0.0),
                first_position: Point::new(0.0, 0.0),
            }
        }
    }

    impl FlatPathBuilder for Builder {
        type PathType = svgdom::Path;

        fn move_to(&mut self, to: Point) {
            self.first_position = to;
            self.current_position = to;
            self.path.push(svgdom::PathSegment::MoveTo { abs: true, x: to.x as f64, y: to.y as f64 });
        }

        fn line_to(&mut self, to: Point) {
            self.current_position = to;
            self.path.push(svgdom::PathSegment::LineTo { abs: true, x: to.x as f64, y: to.y as f64 });
        }

        fn close(&mut self) {
            self.current_position = self.first_position;
            self.path.push(svgdom::PathSegment::ClosePath { abs: true });
        }

        fn build(self) -> Self::PathType {
            self.path
        }

        fn build_and_reset(&mut self) -> Self::PathType {
            let p = self.path.clone();
            self.path.clear();
            self.current_position = Point::new(0.0, 0.0);
            self.first_position = Point::new(0.0, 0.0);
            p
        }

        fn current_position(&self) -> Point {
            self.current_position
        }
    }

    impl PathBuilder for Builder {
        fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) {
            self.current_position = to;
            self.path.push(svgdom::PathSegment::Quadratic {
                abs: true,
                x1: ctrl.x as f64,
                y1: ctrl.y as f64,
                x: to.x as f64,
                y: to.y as f64,
            });
        }

        fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) {
            self.current_position = to;
            self.path.push(svgdom::PathSegment::CurveTo {
                abs: true,
                x1: ctrl1.x as f64,
                y1: ctrl1.y as f64,
                x2: ctrl2.x as f64,
                y2: ctrl2.y as f64,
                x: to.x as f64,
                y: to.y as f64,
            });
        }

        fn arc(&mut self, center: Point, radii: Vector, sweep_angle: Angle, x_rotation: Angle) {
            let arc = lyon_geom::arc::Arc {
                start_angle: (self.current_position() - center).angle_from_x_axis() - x_rotation,
                center, radii, sweep_angle, x_rotation,
            };
            let arc = arc.to_svg_arc();

            self.path.push(svgdom::PathSegment::EllipticalArc {
                abs: true,
                rx: arc.radii.x as f64,
                ry: arc.radii.y as f64,
                x_axis_rotation: arc.x_rotation.to_degrees() as f64,
                large_arc: arc.flags.large_arc,
                sweep: arc.flags.sweep,
                x: arc.to.x as f64,
                y: arc.to.y as f64,
            });
        }
    }
}
