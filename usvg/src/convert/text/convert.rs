// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::cmp;
use std::rc::Rc;

// external
mod fk {
    pub use font_kit::source::SystemSource;
    pub use font_kit::properties::*;
    pub use font_kit::family_name::FamilyName;
    pub use font_kit::font::Font;
    pub use font_kit::handle::Handle;
}

// self
use tree;
use super::super::prelude::*;
use super::super::{fill, stroke};


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
    pub font: fk::Font,
    pub path: String,
    pub index: u32,
    pub size: f64,
    pub units_per_em: u32,
    pub ascent: f64,
    pub underline_position: f64,
    pub underline_thickness: f64,
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
    pub id: String,
    pub fill: Option<tree::Fill>,
    pub stroke: Option<tree::Stroke>,
    pub font: Font,
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

pub type PositionsList = Vec<CharacterPosition>;
pub type RotateList = Vec<f64>;


pub fn collect_text_chunks(
    tree: &tree::Tree,
    text_elem: &svgdom::Node,
    pos_list: &PositionsList,
    opt: &Options,
) -> Vec<TextChunk> {
    let mut chunks = Vec::new();
    let mut chars_count = 0;
    let mut chunk_bytes_count = 0;
    for child in text_elem.descendants().filter(|n| n.is_text()) {
        let ref parent = child.parent().unwrap();
        let ref attrs = parent.attributes();
        let anchor = convert_text_anchor(parent);

        let font = match resolve_font(attrs, opt) {
            Some(v) => v,
            None => {
                // Skip this span.
                chars_count += child.text().chars().count();
                continue;
            }
        };

        let span = TextSpan {
            start: 0,
            end: 0,
            id: parent.id().clone(),
            fill: fill::convert(tree, attrs, true),
            stroke: stroke::convert(tree, attrs, true),
            font,
            decoration: resolve_decoration(tree, text_elem, parent),
            visibility: super::super::convert_visibility(attrs),
            baseline_shift: parent.attributes().get_number_or(AId::BaselineShift, 0.0),
            letter_spacing: attrs.get_number_or(AId::LetterSpacing, 0.0),
            word_spacing: attrs.get_number_or(AId::WordSpacing, 0.0),
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
    attrs: &svgdom::Attributes,
    opt: &Options,
) -> Option<Font> {
    let size = attrs.get_number_or(AId::FontSize, 0.0);
    if !(size > 0.0) {
        return None;
    }

    let style = attrs.get_str_or(AId::FontStyle, "normal");
    let style = match style {
        "italic"  => fk::Style::Italic,
        "oblique" => fk::Style::Oblique,
        _         => fk::Style::Normal,
    };

    let weight = attrs.get_str_or(AId::FontWeight, "normal");
    let weight = match weight {
        "bold"   => fk::Weight::BOLD,
        "100"    => fk::Weight::THIN,
        "200"    => fk::Weight::EXTRA_LIGHT,
        "300"    => fk::Weight::LIGHT,
        "400"    => fk::Weight::NORMAL,
        "500"    => fk::Weight::MEDIUM,
        "600"    => fk::Weight::SEMIBOLD,
        "700"    => fk::Weight::BOLD,
        "800"    => fk::Weight::EXTRA_BOLD,
        "900"    => fk::Weight::BLACK,
        "bolder" | "lighter" => {
            warn!("'bolder' and 'lighter' font-weight must be already resolved.");
            fk::Weight::NORMAL
        }
        _ => fk::Weight::NORMAL,
    };

    let stretch = attrs.get_str_or(AId::FontStretch, "normal");
    let stretch = match stretch {
        "ultra-condensed"        => fk::Stretch::ULTRA_CONDENSED,
        "extra-condensed"        => fk::Stretch::EXTRA_CONDENSED,
        "narrower" | "condensed" => fk::Stretch::CONDENSED,
        "semi-condensed"         => fk::Stretch::SEMI_CONDENSED,
        "semi-expanded"          => fk::Stretch::SEMI_EXPANDED,
        "wider" | "expanded"     => fk::Stretch::EXPANDED,
        "extra-expanded"         => fk::Stretch::EXTRA_EXPANDED,
        "ultra-expanded"         => fk::Stretch::ULTRA_EXPANDED,
        _                        => fk::Stretch::NORMAL,
    };

    let mut name_list = Vec::new();
    let font_family = attrs.get_str_or(AId::FontFamily, "");
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

    // Add the default font, if not set, as a possible fallback.
    let default_font = fk::FamilyName::Title(opt.font_family.clone());
    if !name_list.contains(&default_font) {
        name_list.push(default_font);
    }

    // Add `serif`, if not set, as a possible fallback.
    if !name_list.contains(&fk::FamilyName::Serif) {
        name_list.push(fk::FamilyName::Serif);
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

            warn!("No match for {:?} font-family.", families.join(", "));
            return None;
        }
    };

    let (path, index) = match handle {
        fk::Handle::Path { ref path, font_index } => {
            (path.to_str().unwrap().to_owned(), font_index)
        }
        _ => return None,
    };

    // TODO: font caching
    let font = match handle.load() {
        Ok(v) => v,
        Err(_) => {
            warn!("Failed to load '{}'.", path);
            return None;
        }
    };

    let metrics = font.metrics();
    let scale = size / metrics.units_per_em as f64;

    Some(Rc::new(FontData {
        font,
        path,
        index,
        size,
        units_per_em: metrics.units_per_em,
        // TODO: do not scale
        ascent: metrics.ascent as f64 * scale,
        underline_position: metrics.underline_position as f64 * scale,
        underline_thickness: metrics.underline_thickness as f64 * scale,
    }))
}

fn convert_text_anchor(node: &svgdom::Node) -> TextAnchor {
    match node.attributes().get_str_or(AId::TextAnchor, "start") {
        "middle" => TextAnchor::Middle,
        "end"    => TextAnchor::End,
        _        => TextAnchor::Start,
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
pub fn resolve_positions_list(text_elem: &svgdom::Node) -> PositionsList {
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
            let ref attrs = child.attributes();

            macro_rules! push_list {
                ($aid:expr, $field:ident) => {
                    if let Some(num_list) = attrs.get_number_list($aid) {
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
pub fn resolve_rotate_list(text_elem: &svgdom::Node) -> RotateList {
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
    tree: &tree::Tree,
    text: &svgdom::Node,
    tspan: &svgdom::Node
) -> TextDecoration {
    let text_dec = conv_text_decoration(text);
    let tspan_dec = conv_text_decoration2(tspan);

    let gen_style = |in_tspan: bool, in_text: bool| {
        let n = if in_tspan {
            tspan.clone()
        } else if in_text {
            text.clone()
        } else {
            return None;
        };

        let ref attrs = n.attributes();
        Some(TextDecorationStyle {
            fill: fill::convert(tree, attrs, true),
            stroke: stroke::convert(tree, attrs, true),
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

/// Resolves the `text` node `text-decoration` property.
///
/// See `preproc::prepare_text::prepare_text_decoration` for details.
fn conv_text_decoration(node: &svgdom::Node) -> TextDecorationTypes {
    debug_assert!(node.is_tag_name(EId::Text));

    let attrs = node.attributes();
    let text = attrs.get_str_or(AId::TextDecoration, "");
    TextDecorationTypes {
        has_underline: text.contains("underline"),
        has_overline: text.contains("overline"),
        has_line_through: text.contains("line-through"),
    }
}

/// Resolves the default `text-decoration` property.
///
/// Unlike the `conv_text_decoration`, can containt only one value.
fn conv_text_decoration2(tspan: &svgdom::Node) -> TextDecorationTypes {
    let attrs = tspan.attributes();
    TextDecorationTypes {
        has_underline:    attrs.get_str(AId::TextDecoration) == Some("underline"),
        has_overline:     attrs.get_str(AId::TextDecoration) == Some("overline"),
        has_line_through: attrs.get_str(AId::TextDecoration) == Some("line-through"),
    }
}
