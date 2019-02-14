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
    pub letter_spacing: f64,
    pub word_spacing: f64,
}

#[derive(Clone, Copy)]
pub struct CharacterPosition {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub dx: Option<f64>,
    pub dy: Option<f64>,
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
}

impl TextSpan {
    pub fn contains(&self, byte_offset: usize) -> bool {
        byte_offset >= self.start && byte_offset < self.end
    }
}

pub type PositionsList = Vec<CharacterPosition>;
pub type RotateList = Vec<f64>;


pub fn collect_text_chunks(
    tree: &tree::Tree,
    text_elem: &svgdom::Node,
    pos_list: &PositionsList,
) -> Vec<TextChunk> {
    let mut chunks = Vec::new();
    let mut char_idx = 0;
    let mut chunk_byte_idx = 0;
    for child in text_elem.descendants().filter(|n| n.is_text()) {
        let text_parent = child.parent().unwrap();
        let attrs = text_parent.attributes();
        let baseline_shift = text_parent.attributes().get_number_or(AId::BaselineShift, 0.0);
        let anchor = convert_text_anchor(&text_parent);

        let font = match resolve_font(&attrs) {
            Some(v) => v,
            None => {
                // Skip this span.
                char_idx += child.text().chars().count();
                continue;
            }
        };

        let span = TextSpan {
            start: 0,
            end: 0,
            id: text_parent.id().clone(),
            fill: super::super::fill::convert(tree, &attrs, true),
            stroke: super::super::stroke::convert(tree, &attrs, true),
            font,
            decoration: resolve_decoration(tree, text_elem, &text_parent),
            visibility: super::super::convert_visibility(&attrs),
            baseline_shift,
        };

        let mut is_new_span = true;
        for c in child.text().chars() {
            // Create a new chunk if:
            // - this is the first span
            // - text character has an absolute coordinate assigned to it (via x/y attribute)
            if pos_list[char_idx].x.is_some() || pos_list[char_idx].y.is_some() || chunks.is_empty() {
                chunk_byte_idx = 0;

                let mut span2 = span.clone();
                span2.start = 0;
                span2.end = c.len_utf8();

                chunks.push(TextChunk {
                    x: pos_list[char_idx].x,
                    y: pos_list[char_idx].y,
                    anchor,
                    spans: vec![span2],
                    text: c.to_string(),
                });
            } else if is_new_span {
                let mut span2 = span.clone();
                span2.start = chunk_byte_idx;
                span2.end = chunk_byte_idx + c.len_utf8();

                if let Some(chunk) = chunks.last_mut() {
                    chunk.text.push(c);
                    chunk.spans.push(span2);
                }
            } else {
                if let Some(chunk) = chunks.last_mut() {
                    chunk.text.push(c);
                    if let Some(span) = chunk.spans.last_mut() {
                        debug_assert_ne!(span.end, 0);
                        span.end += c.len_utf8();
                    }
                }
            }

            is_new_span = false;
            char_idx += 1;
            chunk_byte_idx += c.len_utf8();
        }
    }

    chunks
}

fn resolve_font(
    attrs: &svgdom::Attributes,
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

    let mut font_list = Vec::new();
    let font_family = attrs.get_str_or(AId::FontFamily, "");
    for family in font_family.split(',') {
        let family = family.replace('\'', "");

        let name = match family.as_ref() {
            "serif"      => fk::FamilyName::Serif,
            "sans-serif" => fk::FamilyName::SansSerif,
            "monospace"  => fk::FamilyName::Monospace,
            "cursive"    => fk::FamilyName::Cursive,
            "fantasy"    => fk::FamilyName::Fantasy,
            _            => fk::FamilyName::Title(family)
        };

        font_list.push(name);
    }

    let properties = fk::Properties { style, weight, stretch };
    let handle = match fk::SystemSource::new().select_best_match(&font_list, &properties) {
        Ok(v) => v,
        Err(_) => {
            // TODO: Select any font?
            warn!("No match for {:?} font-family.", font_family);
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
            warn!("Failed to load font for {:?} font-family.", font_family);
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
        ascent: metrics.ascent as f64 * scale,
        underline_position: metrics.underline_position as f64 * scale,
        underline_thickness: metrics.underline_thickness as f64 * scale,
        letter_spacing: attrs.get_number_or(AId::LetterSpacing, 0.0),
        word_spacing: attrs.get_number_or(AId::WordSpacing, 0.0),
    }))
}

fn convert_text_anchor(node: &svgdom::Node) -> TextAnchor {
    match node.attributes().get_str_or(AId::TextAnchor, "start") {
        "middle" => TextAnchor::Middle,
        "end"    => TextAnchor::End,
        _        => TextAnchor::Start,
    }
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
/// is a Unicode code point. But it's not final, not in the spec, and may change any time.
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
pub fn resolve_rotate(text_elem: &svgdom::Node) -> RotateList {
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

// TODO: explain how it works
fn resolve_decoration(
    tree: &tree::Tree,
    text: &svgdom::Node,
    tspan: &svgdom::Node
) -> TextDecoration {
    let text_dec = conv_text_decoration(text);
    let tspan_dec = conv_tspan_decoration(tspan);

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
            fill: super::super::fill::convert(tree, attrs, true),
            stroke: super::super::stroke::convert(tree, attrs, true),
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

// 'text-decoration' defined on the 'text' element
// should be generated by 'preproc::prepare_text::prepare_text_decoration'.
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

// 'text-decoration' in 'tspan' does not depend on parent elements.
fn conv_tspan_decoration(tspan: &svgdom::Node) -> TextDecorationTypes {
    let attrs = tspan.attributes();

    let has_attr = |decoration_id: &str| {
        if let Some(id) = attrs.get_str(AId::TextDecoration) {
            if id == decoration_id {
                return true;
            }
        }

        false
    };

    TextDecorationTypes {
        has_underline: has_attr("underline"),
        has_overline: has_attr("overline"),
        has_line_through: has_attr("line-through"),
    }
}
