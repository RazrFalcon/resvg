// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::cmp;
use std::mem;
use std::path::PathBuf;
use std::rc::Rc;

// external
use svgdom;
use harfbuzz;
use unicode_bidi;
use unicode_script;

mod fk {
    pub use font_kit::family_name::FamilyName;
    pub use font_kit::font::Font;
    pub use font_kit::handle::Handle;
    pub use font_kit::hinting::HintingOptions as Hinting;
    pub use font_kit::properties::*;
    pub use font_kit::source::SystemSource;
}

// self
use crate::tree;
use crate::tree::prelude::*;
use super::prelude::*;
use super::{
    style,
    units,
};


// TODO: visibility on text and tspan
// TODO: group when Options::keep_named_groups is set


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


/// A read-only text index in bytes.
///
/// Guarantee to be on a char boundary and in text bounds.
#[derive(Clone, Copy, PartialEq)]
struct ByteIndex(usize);

impl ByteIndex {
    fn new(i: usize) -> Self {
        ByteIndex(i)
    }

    fn value(&self) -> usize {
        self.0
    }
}


#[derive(Clone, Copy, PartialEq)]
enum TextAnchor {
    Start,
    Middle,
    End,
}

type Font = Rc<FontData>;

struct FontData {
    handle: fk::Font,
    path: PathBuf,
    index: u32,
    units_per_em: u32,
    ascent: f32,
    underline_position: f32,
    underline_thickness: f32,
}

impl FontData {
    fn scale(&self, font_size: f64) -> f64 {
        let s = font_size / self.units_per_em as f64;
        debug_assert!(s.is_finite(), "units per em cannot be {}", self.units_per_em);
        s
    }

    fn ascent(&self, font_size: f64) -> f64 {
        self.ascent as f64 * self.scale(font_size)
    }

    fn underline_position(&self, font_size: f64) -> f64 {
        self.underline_position as f64 * self.scale(font_size)
    }

    fn underline_thickness(&self, font_size: f64) -> f64 {
        self.underline_thickness as f64 * self.scale(font_size)
    }
}

impl std::fmt::Debug for FontData {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "FontData({}:{})", self.path.display(), self.index)
    }
}


/// A text chunk.
///
/// Text alignment and BIDI reordering can be done only inside a text chunk.
struct TextChunk {
    x: Option<f64>,
    y: Option<f64>,
    anchor: TextAnchor,
    spans: Vec<TextSpan>,
    text: String,
}

impl TextChunk {
    fn span_at(&self, byte_offset: ByteIndex) -> Option<&TextSpan> {
        for span in &self.spans {
            if span.contains(byte_offset) {
                return Some(span);
            }
        }

        None
    }
}


/// Spans do not overlap.
#[derive(Clone)]
struct TextSpan {
    start: usize,
    end: usize,
    fill: Option<tree::Fill>,
    stroke: Option<tree::Stroke>,
    font: Font,
    font_size: f64,
    decoration: TextDecoration,
    baseline_shift: f64,
    visibility: tree::Visibility,
    letter_spacing: f64,
    word_spacing: f64,
}

impl TextSpan {
    fn contains(&self, byte_offset: ByteIndex) -> bool {
        byte_offset.value() >= self.start && byte_offset.value() < self.end
    }
}


pub fn convert(
    node: &svgdom::Node,
    state: &State,
    parent: &mut tree::Node,
    tree: &mut tree::Tree,
) {
    let pos_list = resolve_positions_list(node, state);
    let rotate_list = resolve_rotate_list(node);
    let text_ts = node.attributes().get_transform(AId::Transform);

    let mut chunks = collect_text_chunks(node, &pos_list, state, tree);
    let mut char_offset = 0;
    let mut x = 0.0;
    let mut baseline = 0.0;
    for chunk in &mut chunks {
        x = chunk.x.unwrap_or(x);
        baseline = chunk.y.unwrap_or(baseline);

        let mut clusters = render_chunk(&chunk, state);
        apply_letter_spacing(&chunk, &mut clusters);
        apply_word_spacing(&chunk, &mut clusters);
        resolve_clusters_positions(&chunk.text, char_offset, &pos_list, &rotate_list, &mut clusters);

        let width = clusters.iter().fold(0.0, |w, glyph| w + glyph.advance);

        x -= process_anchor(chunk.anchor, width);

        for span in &mut chunk.spans {
            let decoration_spans = collect_decoration_spans(span, &clusters);

            if let Some(decoration) = span.decoration.underline.take() {
                parent.append_kind(convert_decoration(
                    x, baseline - span.font.underline_position(span.font_size),
                    &span, decoration, &decoration_spans, text_ts,
                ));
            }

            if let Some(decoration) = span.decoration.overline.take() {
                // TODO: overline pos from font
                parent.append_kind(convert_decoration(
                    x, baseline - span.font.ascent(span.font_size),
                    &span, decoration, &decoration_spans, text_ts,
                ));
            }

            if let Some(path) = convert_span(x, baseline, span, &mut clusters, &text_ts) {
                parent.append_kind(path);
            }

            if let Some(decoration) = span.decoration.line_through.take() {
                // TODO: line-through pos from font
                parent.append_kind(convert_decoration(
                    x, baseline - span.font.ascent(span.font_size) / 3.0,
                    &span, decoration, &decoration_spans, text_ts,
                ));
            }
        }

        char_offset += chunk.text.chars().count();
        x += width;
    }
}

fn collect_text_chunks(
    text_elem: &svgdom::Node,
    pos_list: &[CharacterPosition],
    state: &State,
    tree: &mut tree::Tree,
) -> Vec<TextChunk> {
    let mut chunks = Vec::new();
    let mut chars_count = 0;
    let mut chunk_bytes_count = 0;
    for child in text_elem.descendants().filter(|n| n.is_text()) {
        let ref parent = child.parent().unwrap();
        let anchor = conv_text_anchor(parent);

        // TODO: what to do when <= 0? UB?
        let font_size = units::resolve_font_size(parent, state);
        if !(font_size > 0.0) {
            // Skip this span.
            chars_count += child.text().chars().count();
            continue;
        }

        let font = match resolve_font(parent, state) {
            Some(v) => v,
            None => {
                // Skip this span.
                chars_count += child.text().chars().count();
                continue;
            }
        };

        let letter_spacing = parent.resolve_length(AId::LetterSpacing, state, 0.0);
        let word_spacing = parent.resolve_length(AId::WordSpacing, state, 0.0);

        let span = TextSpan {
            start: 0,
            end: 0,
            fill: style::resolve_fill(parent, true, state, tree),
            stroke: style::resolve_stroke(parent, true, state, tree),
            font,
            font_size,
            decoration: resolve_decoration(text_elem, parent, state, tree),
            visibility: parent.find_enum(AId::Visibility),
            baseline_shift: resolve_baseline_shift(parent, state),
            letter_spacing,
            word_spacing,
        };

        let mut is_new_span = true;
        for c in child.text().chars() {
            let char_len = c.len_utf8();

            // Create a new chunk if:
            // - this is the first span (yes, position can be None)
            // - text character has an absolute coordinate assigned to it (via x/y attribute)
            let is_new_chunk =    pos_list[chars_count].x.is_some()
                               || pos_list[chars_count].y.is_some()
                               || chunks.is_empty();

            if is_new_chunk {
                chunk_bytes_count = 0;

                let mut span2 = span.clone();
                span2.start = 0;
                span2.end = char_len;

                chunks.push(TextChunk {
                    x: pos_list[chars_count].x,
                    y: pos_list[chars_count].y,
                    anchor,
                    spans: vec![span2],
                    text: c.to_string(),
                });
            } else if is_new_span {
                // Add this span to the last text chunk.
                let mut span2 = span.clone();
                span2.start = chunk_bytes_count;
                span2.end = chunk_bytes_count + char_len;

                if let Some(chunk) = chunks.last_mut() {
                    chunk.text.push(c);
                    chunk.spans.push(span2);
                }
            } else {
                // Extend the last span.
                if let Some(chunk) = chunks.last_mut() {
                    chunk.text.push(c);
                    if let Some(span) = chunk.spans.last_mut() {
                        debug_assert_ne!(span.end, 0);
                        span.end += char_len;
                    }
                }
            }

            is_new_span = false;
            chars_count += 1;
            chunk_bytes_count += char_len;
        }
    }

    chunks
}

fn resolve_font(
    node: &svgdom::Node,
    state: &State,
) -> Option<Font> {
    let style = conv_font_style(node);
    let stretch = conv_font_stretch(node);
    let weight = resolve_font_weight(node);

    let font_family = if let Some(n) = node.find_node_with_attribute(AId::FontFamily) {
        n.attributes().get_str_or(AId::FontFamily, &state.opt.font_family).to_owned()
    } else {
        state.opt.font_family.to_owned()
    };

    let mut name_list = Vec::new();
    for family in font_family.split(',') {
        // TODO: to a proper parser
        let family = family.replace('\'', "");
        let family = family.trim();

        let name = match family {
            "serif"      => fk::FamilyName::Serif,
            "sans-serif" => fk::FamilyName::SansSerif,
            "monospace"  => fk::FamilyName::Monospace,
            "cursive"    => fk::FamilyName::Cursive,
            "fantasy"    => fk::FamilyName::Fantasy,
            _            => fk::FamilyName::Title(family.to_string())
        };

        name_list.push(name);
    }

    let properties = fk::Properties { style, weight, stretch };
    let handle = match fk::SystemSource::new().select_best_match(&name_list, &properties) {
        Ok(v) => v,
        Err(_) => {
            let mut families = Vec::new();
            for name in name_list {
                families.push(match name {
                    fk::FamilyName::Serif           => "serif".to_string(),
                    fk::FamilyName::SansSerif       => "sans-serif".to_string(),
                    fk::FamilyName::Monospace       => "monospace".to_string(),
                    fk::FamilyName::Cursive         => "cursive".to_string(),
                    fk::FamilyName::Fantasy         => "fantasy".to_string(),
                    fk::FamilyName::Title(ref name) => name.clone(),
                });
            }

            warn!("No match for '{}' font-family.", families.join(", "));
            return None;
        }
    };

    load_font(&handle)
}

fn load_font(handle: &fk::Handle) -> Option<Font> {
    let (path, index) = match handle {
        fk::Handle::Path { ref path, font_index } => {
            (path.clone(), *font_index)
        }
        _ => return None,
    };

    let font = match handle.load() {
        Ok(v) => v,
        Err(_) => {
            warn!("Failed to load '{}'.", path.display());
            return None;
        }
    };

    let metrics = font.metrics();

    // Some fonts can have `units_per_em` set to zero, which will break out calculations.
    if metrics.units_per_em == 0 {
        return None;
    }

    Some(Rc::new(FontData {
        handle: font,
        path,
        index,
        units_per_em: metrics.units_per_em,
        ascent: metrics.ascent,
        underline_position: metrics.underline_position,
        underline_thickness: metrics.underline_thickness,
    }))
}

fn conv_font_style(node: &svgdom::Node) -> fk::Style {
    if let Some(n) = node.find_node_with_attribute(AId::FontStyle) {
        match n.attributes().get_str_or(AId::FontStyle, "") {
            "italic"  => fk::Style::Italic,
            "oblique" => fk::Style::Oblique,
            _         => fk::Style::Normal,
        }
    } else {
        fk::Style::Normal
    }
}

fn conv_font_stretch(node: &svgdom::Node) -> fk::Stretch {
    if let Some(n) = node.find_node_with_attribute(AId::FontStretch) {
        match n.attributes().get_str_or(AId::FontStretch, "") {
            "narrower" | "condensed" => fk::Stretch::CONDENSED,
            "ultra-condensed"        => fk::Stretch::ULTRA_CONDENSED,
            "extra-condensed"        => fk::Stretch::EXTRA_CONDENSED,
            "semi-condensed"         => fk::Stretch::SEMI_CONDENSED,
            "semi-expanded"          => fk::Stretch::SEMI_EXPANDED,
            "wider" | "expanded"     => fk::Stretch::EXPANDED,
            "extra-expanded"         => fk::Stretch::EXTRA_EXPANDED,
            "ultra-expanded"         => fk::Stretch::ULTRA_EXPANDED,
            _                        => fk::Stretch::NORMAL,
        }
    } else {
        fk::Stretch::NORMAL
    }
}

fn conv_text_anchor(node: &svgdom::Node) -> TextAnchor {
    if let Some(n) = node.find_node_with_attribute(AId::TextAnchor) {
        match n.attributes().get_str_or(AId::TextAnchor, "") {
            "middle" => TextAnchor::Middle,
            "end"    => TextAnchor::End,
            _        => TextAnchor::Start,
        }
    } else {
        TextAnchor::Start
    }
}

#[derive(Clone, Copy)]
struct CharacterPosition {
    x: Option<f64>,
    y: Option<f64>,
    dx: Option<f64>,
    dy: Option<f64>,
}

/// Resolves text's character positions.
///
/// This includes: x, y, dx, dy.
///
/// # The character
///
/// The first problem with this task is that the *character* itself
/// is basically undefined in the SVG spec. Sometimes it's an *XML character*,
/// sometimes a *glyph*, and sometimes just a *character*.
///
/// There is an ongoing [discussion](https://github.com/w3c/svgwg/issues/537)
/// on the SVG working group that addresses this by stating that a character
/// is a Unicode code point. But it's not final.
///
/// Also, according to the SVG 2 spec, *character* is *a Unicode code point*.
///
/// Anyway, we treat a character as a Unicode code point.
///
/// # Algorithm
///
/// To resolve positions, we have to iterate over descendant nodes and
/// if the current node is a `tspan` and has x/y/dx/dy attribute,
/// than the positions from this attribute should be assigned to the characters
/// of this `tspan` and it's descendants.
///
/// Positions list can have more values than characters in the `tspan`,
/// so we have to clamp it, because values should not overlap, e.g.:
///
/// (we ignore whitespaces for example purposes,
/// so the `text` content is `Text` and not `T ex t`)
///
/// ```text
/// <text>
///   a
///   <tspan x="10 20 30">
///     bc
///   </tspan>
///   d
/// </text>
/// ```
///
/// In this example, the `d` position should not be set to `30`.
/// And the result should be: `[None, 10, 20, None]`
///
/// Another example:
///
/// ```text
/// <text>
///   <tspan x="100 110 120 130">
///     a
///     <tspan x="50">
///       bc
///     </tspan>
///   </tspan>
///   d
/// </text>
/// ```
///
/// The result should be: `[100, 50, 120, None]`
fn resolve_positions_list(
    text_elem: &svgdom::Node,
    state: &State,
) -> Vec<CharacterPosition> {
    // Allocate a list that has all characters positions set to `None`.
    let total_chars = count_chars(text_elem);
    let mut list = vec![CharacterPosition {
        x: None,
        y: None,
        dx: None,
        dy: None,
    }; total_chars];

    let mut offset = 0;
    for child in text_elem.descendants() {
        if child.is_element() {
            let child_chars = count_chars(&child);
            macro_rules! push_list {
                ($aid:expr, $field:ident) => {
                    if let Some(num_list) = units::convert_list(&child, $aid, state) {
                        // Note that we are using not the total count,
                        // but the amount of characters in the current `tspan` (with children).
                        let len = cmp::min(num_list.len(), child_chars);
                        for i in 0..len {
                            list[offset + i].$field = Some(num_list[i]);
                        }
                    }
                };
            }

            push_list!(AId::X, x);
            push_list!(AId::Y, y);
            push_list!(AId::Dx, dx);
            push_list!(AId::Dy, dy);
        } else if child.is_text() {
            // Advance the offset.
            offset += child.text().chars().count();
        }
    }

    list
}

/// Resolves characters rotation.
///
/// The algorithm is well explained
/// [in the SVG spec](https://www.w3.org/TR/SVG11/text.html#TSpanElement) (scroll down a bit).
///
/// ![](https://www.w3.org/TR/SVG11/images/text/tspan05-diagram.png)
///
/// Note: this algorithm differs from the position resolving one.
fn resolve_rotate_list(
    text_elem: &svgdom::Node,
) -> Vec<f64> {
    // Allocate a list that has all characters angles set to `0.0`.
    let mut list = vec![0.0; count_chars(text_elem)];
    let mut last = 0.0;
    let mut offset = 0;
    for child in text_elem.descendants() {
        if child.is_element() {
            if let Some(num_list) = child.attributes().get_number_list(AId::Rotate) {
                for i in 0..count_chars(&child) {
                    if let Some(a) = num_list.get(i).cloned() {
                        list[offset + i] = a;
                        last = a;
                    } else {
                        // If the rotate list doesn't specify the rotation for
                        // this character - use the last one.
                        list[offset + i] = last;
                    }
                }
            }
        } else if child.is_text() {
            // Advance the offset.
            offset += child.text().chars().count();
        }
    }

    list
}

fn count_chars(node: &svgdom::Node) -> usize {
    let mut total = 0;
    for child in node.descendants().filter(|n| n.is_text()) {
        total += child.text().chars().count();
    }

    total
}

#[derive(Clone)]
struct TextDecorationStyle {
    fill: Option<tree::Fill>,
    stroke: Option<tree::Stroke>,
}

#[derive(Clone)]
struct TextDecoration {
    underline: Option<TextDecorationStyle>,
    overline: Option<TextDecorationStyle>,
    line_through: Option<TextDecorationStyle>,
}

/// Resolves node's `text-decoration` property.
///
/// `text` and `tspan` can point to the same node.
fn resolve_decoration(
    text: &svgdom::Node,
    tspan: &svgdom::Node,
    state: &State,
    tree: &mut tree::Tree,
) -> TextDecoration {
    // TODO: explain the algorithm

    let text_dec = conv_text_decoration(text);
    let tspan_dec = conv_text_decoration2(tspan);

    let mut gen_style = |in_tspan: bool, in_text: bool| {
        let n = if in_tspan {
            tspan.clone()
        } else if in_text {
            text.clone()
        } else {
            return None;
        };

        Some(TextDecorationStyle {
            fill: style::resolve_fill(&n, true, state, tree),
            stroke: style::resolve_stroke(&n, true, state, tree),
        })
    };

    TextDecoration {
        underline:    gen_style(tspan_dec.has_underline,    text_dec.has_underline),
        overline:     gen_style(tspan_dec.has_overline,     text_dec.has_overline),
        line_through: gen_style(tspan_dec.has_line_through, text_dec.has_line_through),
    }
}

struct TextDecorationTypes {
    has_underline: bool,
    has_overline: bool,
    has_line_through: bool,
}

/// Resolves the `text` node's `text-decoration` property.
fn conv_text_decoration(node: &svgdom::Node) -> TextDecorationTypes {
    debug_assert!(node.is_tag_name(EId::Text));

    fn find_decoration(node: &svgdom::Node, value: &str) -> bool {
        node.ancestors().any(|n| n.attributes().get_str(AId::TextDecoration) == Some(value))
    }

    TextDecorationTypes {
        has_underline: find_decoration(node, "underline"),
        has_overline: find_decoration(node, "overline"),
        has_line_through: find_decoration(node, "line-through"),
    }
}

/// Resolves the default `text-decoration` property.
fn conv_text_decoration2(tspan: &svgdom::Node) -> TextDecorationTypes {
    let attrs = tspan.attributes();
    TextDecorationTypes {
        has_underline:    attrs.get_str(AId::TextDecoration) == Some("underline"),
        has_overline:     attrs.get_str(AId::TextDecoration) == Some("overline"),
        has_line_through: attrs.get_str(AId::TextDecoration) == Some("line-through"),
    }
}

fn resolve_baseline_shift(
    node: &svgdom::Node,
    state: &State,
) -> f64 {
    let mut shift = 0.0;
    let nodes: Vec<_> = node.ancestors().take_while(|n| !n.is_tag_name(EId::Text)).collect();
    for n in nodes.iter().rev() {
        match n.attributes().get_value(AId::BaselineShift) {
            Some(AValue::String(ref s)) => {
                match s.as_str() {
                    "baseline" => {}
                    "sub" => shift += units::resolve_font_size(&n, state) * -0.2,
                    "super" => shift += units::resolve_font_size(&n, state) * 0.4,
                    _ => {}
                }
            }
            Some(AValue::Length(len)) => {
                if len.unit == Unit::Percent {
                    shift += units::resolve_font_size(&n, state) * (len.num / 100.0);
                } else {
                    shift += units::convert_length(*len, &n, AId::BaselineShift,
                                                   tree::Units::ObjectBoundingBox, state);
                }
            }
            _ => {}
        }
    }

    shift
}

fn resolve_font_weight(
    node: &svgdom::Node,
) -> fk::Weight {
    fn bound(min: usize, val: usize, max: usize) -> usize {
        cmp::max(min, cmp::min(max, val))
    }

    let nodes: Vec<_> = node.ancestors().collect();
    let mut weight = 400;
    for n in nodes.iter().rev().skip(1) { // skip Root
        weight = match n.attributes().get_str_or(AId::FontWeight, "") {
            "normal" => 400,
            "bold" => 700,
            "100" => 100,
            "200" => 200,
            "300" => 300,
            "400" => 400,
            "500" => 500,
            "600" => 600,
            "700" => 700,
            "800" => 800,
            "900" => 900,
            "bolder" => {
                // By the CSS2 spec the default value should be 400
                // so `bolder` will result in 500.
                // But Chrome and Inkscape will give us 700.
                // Have no idea is it a bug or something, but
                // we will follow such behavior for now.
                let step = if weight == 400 { 300 } else { 100 };

                bound(100, weight + step, 900)
            }
            "lighter" => {
                // By the CSS2 spec the default value should be 400
                // so `lighter` will result in 300.
                // But Chrome and Inkscape will give us 200.
                // Have no idea is it a bug or something, but
                // we will follow such behavior for now.
                let step = if weight == 400 { 200 } else { 100 };

                bound(100, weight - step, 900)
            }
            _ => weight,
        };
    }

    fk::Weight(weight as f32)
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

    let mut fill = span.fill.take();
    if let Some(ref mut fill) = fill {
        // fill-rule on text must always be `nonzero`,
        // otherwise overlapped characters will be clipped.
        fill.rule = tree::FillRule::NonZero;
    }

    let path = tree::Path {
        id: String::new(),
        transform,
        visibility: span.visibility,
        fill,
        stroke: span.stroke.take(),
        rendering_mode: tree::ShapeRendering::default(),
        segments,
    };

    Some(tree::NodeKind::Path(path))
}

/// Converts a text chunk into a list of outlined clusters.
///
/// This function will do the BIDI reordering, text shaping and glyphs outlining,
/// but not the text layouting. So all clusters are in the 0x0 position.
fn render_chunk(
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
                super::path::convert_path(builder.build())
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

            transform_path(&mut outline, &ts);

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
                 - span.font.underline_thickness(span.font_size) / 2.0;

        let rect = Rect::new(
            0.0,
            0.0,
            dec_span.width,
            span.font.underline_thickness(span.font_size),
        ).unwrap();

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
        rendering_mode: tree::ShapeRendering::default(),
        segments,
    })
}

fn add_rect_to_path(rect: Rect, path: &mut Vec<tree::PathSegment>) {
    path.extend_from_slice(&[
        tree::PathSegment::MoveTo { x: rect.x(),     y: rect.y() },
        tree::PathSegment::LineTo { x: rect.right(), y: rect.y() },
        tree::PathSegment::LineTo { x: rect.right(), y: rect.bottom() },
        tree::PathSegment::LineTo { x: rect.x(),     y: rect.bottom() },
        tree::PathSegment::ClosePath,
    ]);
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

        if let Some(font) = load_font(handle) {
            if font.handle.glyph_for_char(c).is_some() {
                return Some(font);
            }
        }
    }

    None
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
