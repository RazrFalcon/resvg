// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::cmp;

// external
use svgdom;

// self
use tree;
use tree::prelude::*;
use super::prelude::*;
use super::{
    style,
    units,
};


pub fn convert(
    node: &svgdom::Node,
    state: &State,
    parent: &mut tree::Node,
    tree: &mut tree::Tree,
) {
    let pos_list = resolve_positions_list(node, state);
    let rotate_list = resolve_rotate_list(node);
    let text_ts = node.attributes().get_transform(AId::Transform);

    let chunks = collect_text_chunks(node, &pos_list, &rotate_list, state, tree);
    if chunks.is_empty() {
        return;
    }

    let rotate = if rotate_list.iter().any(|v| !v.is_fuzzy_zero()) {
        Some(rotate_list)
    } else {
        None
    };

    let rendering_mode = node.find_enum(AId::TextRendering)
                             .unwrap_or(state.opt.text_rendering);

    parent.append_kind(tree::NodeKind::Text(tree::Text {
        id: node.id().clone(),
        transform: text_ts,
        rotate,
        rendering_mode,
        chunks,
    }));
}


type PositionsList = Vec<CharacterPosition>;
type RotateList = Vec<f64>;


fn collect_text_chunks(
    text_elem: &svgdom::Node,
    pos_list: &PositionsList,
    rotate_list: &RotateList,
    state: &State,
    tree: &mut tree::Tree,
) -> Vec<tree::TextChunk> {
    // Update state for a proper fill-rule resolving.
    let mut text_state = state.clone();
    text_state.current_root = text_elem.clone();

    let mut chunks = Vec::new();
    let mut chars_count = 0;
    for child in text_elem.descendants().filter(|n| n.is_text()) {
        let ref parent = child.parent().unwrap();

        if !style::is_visible_element(parent, state.opt) {
            chars_count += child.text().chars().count();
            continue;
        }

        let font = match resolve_font(parent, state) {
            Some(v) => v,
            None => {
                chars_count += child.text().chars().count();
                continue;
            }
        };

        let anchor = convert_text_anchor(parent);
        let span = tree::TextSpan {
            visibility: super::convert_visibility(parent),
            fill: style::resolve_fill(parent, true, &text_state, tree),
            stroke: style::resolve_stroke(parent, true, state, tree),
            font,
            baseline_shift: resolve_baseline_shift(parent, state),
            decoration: resolve_decoration(text_elem, parent, state, tree),
            text: String::new(),
        };

        let mut is_new_span = true;
        for c in child.text().chars() {
            // Create a new chunk if:
            // - this is the first span (yes, position can be None)
            // - text character has an absolute coordinate assigned to it (via x/y attribute)
            // TODO: technically, only x and y should affect text chunk creation, but
            //       resvg doesn't support this yet.
            let is_new_chunk =    pos_list[chars_count].x.is_some()
                               || pos_list[chars_count].y.is_some()
                               || pos_list[chars_count].dx.is_some()
                               || pos_list[chars_count].dy.is_some()
                               || !rotate_list[chars_count].is_fuzzy_zero()
                               || chunks.is_empty();

            if is_new_chunk {
                let mut span2 = span.clone();
                span2.text.push(c);

                chunks.push(tree::TextChunk {
                    x: pos_list[chars_count].x,
                    y: pos_list[chars_count].y,
                    dx: pos_list[chars_count].dx,
                    dy: pos_list[chars_count].dy,
                    anchor,
                    spans: vec![span2],
                });
            } else if is_new_span {
                // Add this span to the last text chunk.
                if let Some(chunk) = chunks.last_mut() {
                    let mut span2 = span.clone();
                    span2.text.push(c);

                    chunk.spans.push(span2);
                }
            } else {
                // Extend the last span.
                if let Some(chunk) = chunks.last_mut() {
                    if let Some(span) = chunk.spans.last_mut() {
                        span.text.push(c);
                    }
                }
            }

            is_new_span = false;
            chars_count += 1;
        }
    }

    chunks
}

fn resolve_font(
    node: &svgdom::Node,
    state: &State,
) -> Option<tree::Font> {
    let style = node.find_str(AId::FontStyle, "normal", |value| {
        match value {
            "italic" =>  tree::FontStyle::Italic,
            "oblique" => tree::FontStyle::Oblique,
            _ =>         tree::FontStyle::Normal,
        }
    });

    let variant = node.find_str(AId::FontVariant, "normal", |value| {
        match value {
            "small-caps" => tree::FontVariant::SmallCaps,
            _ =>            tree::FontVariant::Normal,
        }
    });

    let weight = resolve_font_weight(node);

    let stretch = node.find_str(AId::FontStretch, "normal", |value| {
        match value {
            "wider" =>           tree::FontStretch::Wider,
            "narrower" =>        tree::FontStretch::Narrower,
            "ultra-condensed" => tree::FontStretch::UltraCondensed,
            "extra-condensed" => tree::FontStretch::ExtraCondensed,
            "condensed" =>       tree::FontStretch::Condensed,
            "semi-condensed" =>  tree::FontStretch::SemiCondensed,
            "semi-expanded" =>   tree::FontStretch::SemiExpanded,
            "expanded" =>        tree::FontStretch::Expanded,
            "extra-expanded" =>  tree::FontStretch::ExtraExpanded,
            "ultra-expanded" =>  tree::FontStretch::UltraExpanded,
            _ =>                 tree::FontStretch::Normal,
        }
    });

    let letter_spacing = node.resolve_length(AId::LetterSpacing, state, 0.0);
    let letter_spacing = if letter_spacing.is_fuzzy_zero() { None } else { Some(letter_spacing) };

    let word_spacing = node.resolve_length(AId::WordSpacing, state, 0.0);
    let word_spacing = if word_spacing.is_fuzzy_zero() { None } else { Some(word_spacing) };

    // TODO: what to do when <= 0? UB?
    let size = units::resolve_font_size(node, state);
    if !(size > 0.0) {
        return None;
    }
    let size = tree::FontSize::new(size);

    let family = if let Some(n) = node.find_node_with_attribute(AId::FontFamily) {
        n.attributes().get_str_or(AId::FontFamily, &state.opt.font_family).to_owned()
    } else {
        state.opt.font_family.to_owned()
    };

    Some(tree::Font {
        family,
        size,
        style,
        variant,
        weight,
        stretch,
        letter_spacing,
        word_spacing,
    })
}

fn convert_text_anchor(node: &svgdom::Node) -> tree::TextAnchor {
    node.find_str(AId::TextAnchor, "start", |value| {
        match value {
            "middle" => tree::TextAnchor::Middle,
            "end"    => tree::TextAnchor::End,
            _        => tree::TextAnchor::Start,
        }
    })
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
) -> PositionsList {
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
) -> RotateList {
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

/// Resolves node's `text-decoration` property.
///
/// `text` and `tspan` can point to the same node.
fn resolve_decoration(
    text: &svgdom::Node,
    tspan: &svgdom::Node,
    state: &State,
    tree: &mut tree::Tree,
) -> tree::TextDecoration {
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

        Some(tree::TextDecorationStyle {
            fill: style::resolve_fill(&n, true, state, tree),
            stroke: style::resolve_stroke(&n, true, state, tree),
        })
    };

    tree::TextDecoration {
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
) -> tree::FontWeight {
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

    match weight {
        100 => tree::FontWeight::W100,
        200 => tree::FontWeight::W200,
        300 => tree::FontWeight::W300,
        400 => tree::FontWeight::W400,
        500 => tree::FontWeight::W500,
        600 => tree::FontWeight::W600,
        700 => tree::FontWeight::W700,
        800 => tree::FontWeight::W800,
        900 => tree::FontWeight::W900,
        _ => tree::FontWeight::W400,
    }
}
