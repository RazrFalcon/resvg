// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::cmp;
use std::mem;

// external
use svgdom;
use harfbuzz;
use unicode_bidi;
use unicode_script;

mod fk {
    pub use font_kit::hinting::HintingOptions as Hinting;
    pub use font_kit::font::Font;
}

// self
mod convert;
use tree;
use tree::prelude::*;
use super::prelude::*;
use self::convert::*;

// TODO: visibility on text and tspan
// TODO: group when Options::keep_named_groups is set
// TODO: `fill-rule` must be set to `nonzero` for text


type Range = std::ops::Range<usize>;

/// A glyph.
///
/// Basically, a glyph ID and it's metrics.
#[derive(Clone, Copy)]
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
struct OutlinedCluster {
    /// Position in bytes in the original string.
    ///
    /// We use it to match a cluster with a character in the text chunk and therefore with the style.
    byte_idx: ByteIndex,

    /// The cluster position in SVG coordinates.
    x: f64,

    /// The cluster position in SVG coordinates.
    y: f64,

    /// The rotation angle.
    rotate: f64,

    /// An advance along the X axis.
    ///
    /// Can be negative.
    advance: f64,

    /// Indicates that this cluster was affected by the relative shift (via dx/dy attributes)
    /// during the text layouting.
    ///
    /// Used during the `text-decoration` processing.
    has_relative_shift: bool,

    /// The actual outline.
    path: Vec<tree::PathSegment>,
}


/// A text decoration span.
///
/// Basically a horizontal line, that will be used for underline, overline and line-through.
/// It doesn't have a height, since it depends on the font metrics.
#[derive(Clone, Copy)]
struct DecorationSpan {
    x: f64,
    baseline: f64,
    width: f64,
    angle: f64,
}


pub fn convert(
    text_elem: &svgdom::Node,
    opt: &Options,
    mut parent: tree::Node,
    tree: &mut tree::Tree,
) {
    let pos_list = resolve_positions_list(text_elem);
    let rotate_list = resolve_rotate_list(text_elem);
    let text_ts = text_elem.attributes().get_transform(AId::Transform).unwrap_or_default();

    let mut chunks = collect_text_chunks(tree, &text_elem, &pos_list, opt);
    let mut char_offset = 0;
    let mut x = 0.0;
    let mut baseline = 0.0;
    for chunk in &mut chunks {
        x = chunk.x.unwrap_or(x);
        baseline = chunk.y.unwrap_or(baseline);

        let mut clusters = render_chunk(&chunk);
        scale_clusters(&chunk, &mut clusters);
        apply_letter_spacing(&chunk, &mut clusters);
        apply_word_spacing(&chunk, &mut clusters);
        resolve_clusters_positions(&chunk.text, char_offset, &pos_list, &rotate_list, &mut clusters);

        let width = clusters.iter().fold(0.0, |w, glyph| w + glyph.advance);

        x -= process_anchor(chunk.anchor, width);

        for span in &mut chunk.spans {
            let decoration_spans = collect_decoration_spans(span, &clusters);

            if let Some(decoration) = span.decoration.underline.take() {
                parent.append_kind(convert_decoration(
                    x, baseline - span.font.underline_position,
                    &span, decoration, &decoration_spans, text_ts,
                ));
            }

            if let Some(decoration) = span.decoration.overline.take() {
                // TODO: overline pos from font
                parent.append_kind(convert_decoration(
                    x, baseline - span.font.ascent,
                    &span, decoration, &decoration_spans, text_ts,
                ));
            }

            if let Some(path) = convert_span(x, baseline, span, &mut clusters, &text_ts) {
                parent.append_kind(path);
            }

            if let Some(decoration) = span.decoration.line_through.take() {
                // TODO: line-through pos from font
                parent.append_kind(convert_decoration(
                    x, baseline - span.font.ascent / 3.0,
                    &span, decoration, &decoration_spans, text_ts,
                ));
            }
        }

        char_offset += chunk.text.chars().count();
        x += width;
    }
}

fn convert_span(
    x: f64,
    baseline: f64,
    span: &mut TextSpan,
    clusters: &mut [OutlinedCluster],
    text_ts: &tree::Transform,
) -> Option<tree::NodeKind> {
    let mut segments = Vec::new();

    for cluster in clusters {
        if span.contains(cluster.byte_idx) {
            let mut path = mem::replace(&mut cluster.path, Vec::new());
            let mut transform = tree::Transform::new_translate(cluster.x, cluster.y);
            if !cluster.rotate.is_fuzzy_zero() {
                transform.rotate(cluster.rotate);
            }

            transform_path(&mut path, &transform);

            if !path.is_empty() {
                segments.extend_from_slice(&path);
            }
        }
    }

    if segments.is_empty() {
        return None;
    }

    let mut transform = text_ts.clone();
    transform.translate(x, baseline - span.baseline_shift);

    // TODO: fill and stroke with a gradient/pattern that has objectBoundingBox
    //       should use the text element bbox and not the tspan bbox.

    let mut path = tree::Path {
        id: String::new(),
        transform,
        visibility: span.visibility,
        fill: span.fill.take(),
        stroke: span.stroke.take(),
        segments,
    };

    mem::swap(&mut path.id, &mut span.id);

    Some(tree::NodeKind::Path(path))
}

fn font_eq(f1: &Font, f2: &Font) -> bool {
       f1.path  == f2.path
    && f1.index == f2.index
}

/// Converts a text chunk into a list of outlined clusters.
///
/// This function will do the BIDI reordering, text shaping and glyphs outlining,
/// but not the text layouting. So all glyphs are in 0x0 position.
fn render_chunk(
    chunk: &TextChunk,
) -> Vec<OutlinedCluster> {
    // Shape the text using all fonts in the chunk.
    //
    // I'm not sure if this can be optimized, but it's probably pretty expensive.
    let fonts = collect_unique_fonts(chunk);
    if fonts.is_empty() {
        return Vec::new();
    }

    let mut lists_of_glyphs = Vec::new();
    for font in &fonts {
        let glyphs = shape_text(&chunk.text, font);
        lists_of_glyphs.push(glyphs);
    }

    // Check that all glyphs lists have the same length.
    if !lists_of_glyphs.iter().all(|v| v.len() == lists_of_glyphs[0].len()) {
        warn!("Text layouting failed.");
        return Vec::new();
    }

    // Check that glyph clusters in all lists are the same.
    //
    // For example, if one font supports ligatures and the other don't.
    // Not sure what to do in this case, so we should stop here for now.
    for glyphs in lists_of_glyphs.iter().skip(1) {
        let iter = GlyphClusters::new(glyphs)
                       .zip(GlyphClusters::new(&lists_of_glyphs[0]));
        for (g1, g2) in iter {
            if g1 != g2 {
                warn!("Text layouting failed.");
                return Vec::new();
            }
        }
    }

    let mut clusters = Vec::new();
    for (range, byte_idx) in GlyphClusters::new(&lists_of_glyphs[0]) {
        if let Some(span) = chunk.span_at(byte_idx) {
            for (font, glyphs) in fonts.iter().zip(&lists_of_glyphs) {
                if font_eq(&span.font, font) {
                    clusters.push(outline_cluster(font, &glyphs[range]));
                    break;
                }
            }
        }
    }

    clusters
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


/// Returns a list of unique fonts in the specified chunk.
fn collect_unique_fonts(
    chunk: &TextChunk,
) -> Vec<Font> {
    let mut list = Vec::new();

    for span in &chunk.spans {
        let mut exists = false;
        for font in &list {
            if font_eq(font, &span.font) {
                exists = true;
                break;
            }
        }

        if !exists {
            list.push(span.font.clone());
        }
    }

    list
}

/// Converts a text into a list of glyph IDs.
///
/// This function will do the BIDI reordering and text shaping.
fn shape_text(
    text: &str,
    font: &Font,
) -> Vec<Glyph> {
    let font_data = try_opt!(font.font.copy_font_data(), Vec::new());
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

        // TODO: if the output has missing glyphs, we should fetch them from other fonts
        //       no idea how to implement this

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
            });
        }
    }

    glyphs
}

/// Outlines a glyph cluster.
///
/// Uses one or more `Glyph`s to construct an `OutlinedCluster`.
fn outline_cluster(
    font: &Font,
    glyphs: &[Glyph],
) -> OutlinedCluster {
    debug_assert!(!glyphs.is_empty());

    use lyon_path::builder::FlatPathBuilder;

    let mut path = Vec::new();
    let mut width = 0;
    let mut x = 0.0;

    for glyph in glyphs {
        let mut builder = svgdom_path_builder::Builder::new();
        let mut outline = match font.font.outline(glyph.id, fk::Hinting::None, &mut builder) {
            Ok(_) => {
                super::path::convert_path(builder.build())
            }
            Err(_) => {
                warn!("Glyph {} not found in the font.", glyph.id);
                Vec::new()
            }
        };

        if !outline.is_empty() {
            // By default, glyphs are upside-down, so we have to mirror them.
            let mut ts = svgdom::Transform::new_scale(1.0, -1.0);

            // Apply offset.
            //
            // The first glyph in the cluster will have an offset from 0x0,
            // but the later one will have an offset from the "current position".
            // So we have to keep an advance.
            // TODO: vertical advance?
            // TODO: should be done only inside a single text span
            ts.translate(x + glyph.dx as f64, glyph.dy as f64);

            transform_path(&mut outline, &ts);

            path.extend_from_slice(&outline);
        }

        x += glyph.width as f64;
        width = cmp::max(glyph.width, width);
    }

    OutlinedCluster {
        byte_idx: glyphs[0].byte_idx,
        x: 0.0,
        y: 0.0,
        advance: width as f64,
        rotate: 0.0,
        has_relative_shift: false,
        path,
    }
}

/// Applies the transform to the path segments.
fn transform_path(segments: &mut [tree::PathSegment], ts: &tree::Transform) {
    for seg in segments {
        match seg {
            tree::PathSegment::MoveTo { x, y } => {
                ts.apply_to(x, y);
            }
            tree::PathSegment::LineTo { x, y } => {
                ts.apply_to(x, y);
            }
            tree::PathSegment::CurveTo { x1, y1, x2, y2, x,  y } => {
                ts.apply_to(x1, y1);
                ts.apply_to(x2, y2);
                ts.apply_to(x, y);
            }
            tree::PathSegment::ClosePath => {}
        }
    }
}

fn resolve_clusters_positions(
    text: &str,
    offset: usize,
    pos_list: &PositionsList,
    rotate_list: &RotateList,
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

/// Scales clusters to the specified `font-size`.
fn scale_clusters(
    chunk: &TextChunk,
    clusters: &mut Vec<OutlinedCluster>,
) {
    for cluster in clusters {
        if let Some(span) = chunk.span_at(cluster.byte_idx) {
            let scale = span.font.size / span.font.units_per_em as f64;
            cluster.advance *= scale;
            transform_path(&mut cluster.path, &tree::Transform::new_scale(scale, scale));
        }
    }
}

/// Applies the `letter-spacing` property to a text chunk clusters.
///
/// [In the CSS spec](https://www.w3.org/TR/css-text-3/#letter-spacing-property).
fn apply_letter_spacing(
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
fn apply_word_spacing(
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

fn process_anchor(a: TextAnchor, text_width: f64) -> f64 {
    match a {
        TextAnchor::Start   => 0.0, // Nothing.
        TextAnchor::Middle  => text_width / 2.0,
        TextAnchor::End     => text_width,
    }
}

fn collect_decoration_spans(
    span: &TextSpan,
    clusters: &[OutlinedCluster],
) -> Vec<DecorationSpan> {
    let mut spans = Vec::new();

    let mut started = false;
    let mut x = 0.0;
    let mut y = 0.0;
    let mut width = 0.0;
    let mut angle = 0.0;
    for cluster in clusters {
        if span.contains(cluster.byte_idx) {
            if started && (cluster.has_relative_shift || !cluster.rotate.is_fuzzy_zero()) {
                started = false;
                spans.push(DecorationSpan { x, baseline: y, width, angle });
            }

            if !started {
                x = cluster.x;
                y = cluster.y;
                width = cluster.x + cluster.advance - x;
                angle = cluster.rotate;
                started = true;
            } else {
                width = cluster.x + cluster.advance - x;
            }
        } else if started {
            spans.push(DecorationSpan { x, baseline: y, width, angle });
            started = false;
        }
    }

    if started {
        spans.push(DecorationSpan { x, baseline: y, width, angle });
    }

    spans
}

fn convert_decoration(
    x: f64,
    baseline: f64,
    span: &TextSpan,
    mut decoration: TextDecorationStyle,
    decoration_spans: &[DecorationSpan],
    transform: tree::Transform,
) -> tree::NodeKind {
    debug_assert!(!decoration_spans.is_empty());

    let mut segments = Vec::new();
    for dec_span in decoration_spans {
        let tx = x + dec_span.x;
        let ty = baseline + dec_span.baseline - span.baseline_shift
                 - span.font.underline_thickness / 2.0;

        let rect = Rect::new(
            0.0,
            0.0,
            dec_span.width,
            span.font.underline_thickness,
        );

        let start_idx = segments.len();
        add_rect_to_path(rect, &mut segments);

        let mut ts = tree::Transform::new_translate(tx, ty);
        ts.rotate(dec_span.angle);
        transform_path(&mut segments[start_idx..], &ts);
    }

    tree::NodeKind::Path(tree::Path {
        id: String::new(),
        transform,
        visibility: span.visibility,
        fill: decoration.fill.take(),
        stroke: decoration.stroke.take(),
        segments,
    })
}

fn add_rect_to_path(rect: Rect, path: &mut Vec<tree::PathSegment>) {
    path.extend_from_slice(&[
        tree::PathSegment::MoveTo { x: rect.x,       y: rect.y },
        tree::PathSegment::LineTo { x: rect.right(), y: rect.y },
        tree::PathSegment::LineTo { x: rect.right(), y: rect.bottom() },
        tree::PathSegment::LineTo { x: rect.x,       y: rect.bottom() },
        tree::PathSegment::ClosePath,
    ]);
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
