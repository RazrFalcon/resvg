// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::cmp;
use std::path::PathBuf;
use std::rc::Rc;

// external
use svgdom;

mod fk {
    pub use font_kit::family_name::FamilyName;
    pub use font_kit::font::Font;
    pub use font_kit::handle::Handle;
    pub use font_kit::properties::*;
    pub use font_kit::source::SystemSource;
}

// self
use crate::tree;
use crate::convert::prelude::*;
use crate::convert::{
    style,
    units,
};


/// A read-only text index in bytes.
///
/// Guarantee to be on a char boundary and in text bounds.
#[derive(Clone, Copy, PartialEq)]
pub struct ByteIndex(usize);

impl ByteIndex {
    pub fn new(i: usize) -> Self {
        ByteIndex(i)
    }

    pub fn value(&self) -> usize {
        self.0
    }
}


#[derive(Clone, Copy, PartialEq)]
pub enum TextAnchor {
    Start,
    Middle,
    End,
}

pub type Font = Rc<FontData>;

pub struct FontData {
    pub handle: fk::Font,
    pub path: PathBuf,
    pub index: u32,
    units_per_em: u32,
    ascent: f32,
    descent: f32,
    x_height: f32,
    underline_position: f32,
    underline_thickness: f32,
}

impl FontData {
    pub fn scale(&self, font_size: f64) -> f64 {
        let s = font_size / self.units_per_em as f64;
        debug_assert!(s.is_finite(), "units per em cannot be {}", self.units_per_em);
        s
    }

    pub fn ascent(&self, font_size: f64) -> f64 {
        self.ascent as f64 * self.scale(font_size)
    }

    pub fn descent(&self, font_size: f64) -> f64 {
        self.descent as f64 * self.scale(font_size)
    }

    pub fn x_height(&self, font_size: f64) -> f64 {
        self.x_height as f64 * self.scale(font_size)
    }

    pub fn underline_position(&self, font_size: f64) -> f64 {
        self.underline_position as f64 * self.scale(font_size)
    }

    pub fn underline_thickness(&self, font_size: f64) -> f64 {
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
pub struct TextChunk {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub anchor: TextAnchor,
    pub spans: Vec<TextSpan>,
    pub text: String,
}

impl TextChunk {
    pub fn span_at(&self, byte_offset: ByteIndex) -> Option<&TextSpan> {
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
pub struct TextSpan {
    pub start: usize,
    pub end: usize,
    pub fill: Option<tree::Fill>,
    pub stroke: Option<tree::Stroke>,
    pub font: Font,
    pub font_size: f64,
    pub decoration: TextDecoration,
    pub baseline_shift: f64,
    pub visibility: tree::Visibility,
    pub letter_spacing: f64,
    pub word_spacing: f64,
}

impl TextSpan {
    pub fn contains(&self, byte_offset: ByteIndex) -> bool {
        byte_offset.value() >= self.start && byte_offset.value() < self.end
    }
}


#[derive(Clone, Copy, PartialEq)]
pub enum WritingMode {
    LeftToRight,
    TopToBottom,
}


pub fn collect_text_chunks(
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

pub fn load_font(handle: &fk::Handle) -> Option<Font> {
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

    let x_height = if metrics.x_height != 0.0 {
        metrics.x_height
    } else {
        (metrics.ascent - metrics.descent) * 0.45
    };

    Some(Rc::new(FontData {
        handle: font,
        path,
        index,
        units_per_em: metrics.units_per_em,
        ascent: metrics.ascent,
        descent: metrics.descent,
        x_height,
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
pub struct CharacterPosition {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub dx: Option<f64>,
    pub dy: Option<f64>,
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
pub fn resolve_positions_list(
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
pub fn resolve_rotate_list(
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

#[derive(Clone)]
pub struct TextDecorationStyle {
    pub fill: Option<tree::Fill>,
    pub stroke: Option<tree::Stroke>,
}

#[derive(Clone)]
pub struct TextDecoration {
    pub underline: Option<TextDecorationStyle>,
    pub overline: Option<TextDecorationStyle>,
    pub line_through: Option<TextDecorationStyle>,
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
                // TODO: sub/super from font
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

fn count_chars(node: &svgdom::Node) -> usize {
    let mut total = 0;
    for child in node.descendants().filter(|n| n.is_text()) {
        total += child.text().chars().count();
    }

    total
}

/// Converts the writing mode.
///
/// According to the [SVG 2.0] spec, there are only two writing modes:
/// horizontal left-to-right and vertical right-to-left.
/// E.g:
///
/// - `lr`, `lr-tb`, `rl`, `rl-tb` => `horizontal-tb`
/// - `tb`, `tb-rl` => `vertical-rl`
///
/// Also, looks like no one really supports the `rl` and `rl-tb`, except `Batik`.
/// And I'm not sure if it's behaviour is correct.
///
/// So we will ignore it as well, mainly because I have no idea how exactly
/// it should affect the rendering.
///
/// [SVG 2.0]: https://www.w3.org/TR/SVG2/text.html#WritingModeProperty
pub fn convert_writing_mode(node: &svgdom::Node) -> WritingMode {
    // `writing-mode` can be set only on a `text` element.
    debug_assert!(node.is_tag_name(EId::Text));

    if let Some(n) = node.find_node_with_attribute(AId::WritingMode) {
        match n.attributes().get_str_or(AId::WritingMode, "lr-tb") {
            "tb" | "tb-rl" => WritingMode::TopToBottom,
            _ => WritingMode::LeftToRight,
        }
    } else {
        WritingMode::LeftToRight
    }
}
