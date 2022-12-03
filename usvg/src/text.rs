// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::rc::Rc;
use strict_num::NonZeroPositiveF64;

use crate::svgtree::{AId, EId};
use crate::{converter, style, svgtree, Node, NodeExt, NodeKind, SharedPathData, Units};
use crate::{PaintOrder, PathData, TextRendering, Transform, Visibility};
use svgtypes::{Length, LengthUnit};

/// A font stretch property.
#[allow(missing_docs)]
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub enum Stretch {
    UltraCondensed,
    ExtraCondensed,
    Condensed,
    SemiCondensed,
    Normal,
    SemiExpanded,
    Expanded,
    ExtraExpanded,
    UltraExpanded,
}

impl Default for Stretch {
    #[inline]
    fn default() -> Self {
        Stretch::Normal
    }
}

/// A font style property.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Style {
    /// A face that is neither italic not obliqued.
    Normal,
    /// A form that is generally cursive in nature.
    Italic,
    /// A typically-sloped version of the regular face.
    Oblique,
}

impl Default for Style {
    #[inline]
    fn default() -> Style {
        Style::Normal
    }
}

/// Text font properties.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Font {
    /// A list of family names.
    ///
    /// Never empty. Uses [`Options::font_family`](crate::Options::font_family) as fallback.
    pub families: Vec<String>,
    /// A font style.
    pub style: Style,
    /// A font stretch.
    pub stretch: Stretch,
    /// A font width.
    pub weight: u16,
}

/// A dominant baseline property.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DominantBaseline {
    Auto,
    UseScript,
    NoChange,
    ResetSize,
    Ideographic,
    Alphabetic,
    Hanging,
    Mathematical,
    Central,
    Middle,
    TextAfterEdge,
    TextBeforeEdge,
}

/// An alignment baseline property.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum AlignmentBaseline {
    Auto,
    Baseline,
    BeforeEdge,
    TextBeforeEdge,
    Middle,
    Central,
    AfterEdge,
    TextAfterEdge,
    Ideographic,
    Alphabetic,
    Hanging,
    Mathematical,
}

/// A baseline shift property.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BaselineShift {
    Baseline,
    Subscript,
    Superscript,
    Number(f64),
}

impl Default for BaselineShift {
    #[inline]
    fn default() -> BaselineShift {
        BaselineShift::Baseline
    }
}

/// A length adjust property.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LengthAdjust {
    Spacing,
    SpacingAndGlyphs,
}

/// A text span decoration style.
///
/// In SVG, text decoration and text it's applied to can have different styles.
/// So you can have black text and green underline.
///
/// Also, in SVG you can specify text decoration stroking.
#[derive(Clone, Debug)]
pub struct TextDecorationStyle {
    /// A fill style.
    pub fill: Option<style::Fill>,
    /// A stroke style.
    pub stroke: Option<style::Stroke>,
}

/// A text span decoration.
#[derive(Clone, Debug)]
pub struct TextDecoration {
    /// An optional underline and its style.
    pub underline: Option<TextDecorationStyle>,
    /// An optional overline and its style.
    pub overline: Option<TextDecorationStyle>,
    /// An optional line-through and its style.
    pub line_through: Option<TextDecorationStyle>,
}

/// A text style span.
///
/// Spans do not overlap inside a text chunk.
#[derive(Clone, Debug)]
pub struct TextSpan {
    /// A span start in UTF-8 codepoints.
    ///
    /// Offset is relative to the parent text chunk and not the parent text element.
    pub start: usize,
    /// A span end in UTF-8 codepoints.
    ///
    /// Offset is relative to the parent text chunk and not the parent text element.
    pub end: usize,
    /// A fill style.
    pub fill: Option<style::Fill>,
    /// A stroke style.
    pub stroke: Option<style::Stroke>,
    /// A paint order style.
    pub paint_order: PaintOrder,
    /// A font.
    pub font: Font,
    /// A font size.
    pub font_size: NonZeroPositiveF64,
    /// Indicates that small caps should be used.
    ///
    /// Set by `font-variant="small-caps"`
    pub small_caps: bool,
    /// Indicates that a kerning should be applied.
    ///
    /// Supports both `kerning` and `font-kerning` properties.
    pub apply_kerning: bool,
    /// A span decorations.
    pub decoration: TextDecoration,
    /// A span dominant baseline.
    pub dominant_baseline: DominantBaseline,
    /// A span alignment baseline.
    pub alignment_baseline: AlignmentBaseline,
    /// A list of all baseline shift that should be applied to this span.
    ///
    /// Ordered from `text` element down to the actual `span` element.
    pub baseline_shift: Vec<BaselineShift>,
    /// A visibility property.
    pub visibility: Visibility,
    /// A letter spacing property.
    pub letter_spacing: f64,
    /// A word spacing property.
    pub word_spacing: f64,
    /// A text length property.
    pub text_length: Option<f64>,
    /// A length adjust property.
    pub length_adjust: LengthAdjust,
}

/// A text chunk anchor property.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TextAnchor {
    Start,
    Middle,
    End,
}

/// A path used by text-on-path.
#[derive(Clone, Debug)]
pub struct TextPath {
    /// A text offset in SVG coordinates.
    ///
    /// Percentage values already resolved.
    pub start_offset: f64,

    /// A path.
    pub path: Rc<PathData>,
}

/// A text chunk flow property.
#[derive(Clone, Debug)]
pub enum TextFlow {
    /// A linear layout.
    ///
    /// Includes left-to-right, right-to-left and top-to-bottom.
    Linear,
    /// A text-on-path layout.
    Path(Rc<TextPath>),
}

/// A text chunk.
///
/// Text alignment and BIDI reordering can only be done inside a text chunk.
#[derive(Clone, Debug)]
pub struct TextChunk {
    /// An absolute X axis offset.
    pub x: Option<f64>,
    /// An absolute Y axis offset.
    pub y: Option<f64>,
    /// A text anchor.
    pub anchor: TextAnchor,
    /// A list of text chunk style spans.
    pub spans: Vec<TextSpan>,
    /// A text chunk flow.
    pub text_flow: TextFlow,
    /// A text chunk actual text.
    pub text: String,
}

/// A text character position.
///
/// _Character_ is a Unicode codepoint.
#[derive(Clone, Copy, Debug)]
pub struct CharacterPosition {
    /// An absolute X axis position.
    pub x: Option<f64>,
    /// An absolute Y axis position.
    pub y: Option<f64>,
    /// A relative X axis offset.
    pub dx: Option<f64>,
    /// A relative Y axis offset.
    pub dy: Option<f64>,
}

/// A writing mode.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum WritingMode {
    LeftToRight,
    TopToBottom,
}

/// A text element.
///
/// `text` element in SVG.
#[derive(Clone, Debug)]
pub struct Text {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Isn't automatically generated.
    /// Can be empty.
    pub id: String,

    /// Element transform.
    pub transform: Transform,

    /// Rendering mode.
    ///
    /// `text-rendering` in SVG.
    pub rendering_mode: TextRendering,

    /// A list of character positions.
    ///
    /// One position for each Unicode codepoint. Aka `char` in Rust and not UTF-8 byte.
    pub positions: Vec<CharacterPosition>,

    /// A list of rotation angles.
    ///
    /// One angle for each Unicode codepoint. Aka `char` in Rust and not UTF-8 byte.
    pub rotate: Vec<f64>,

    /// A writing mode.
    pub writing_mode: WritingMode,

    /// A list of text chunks.
    pub chunks: Vec<TextChunk>,
}

// Converter.

impl_enum_default!(TextAnchor, Start);

impl_enum_from_str!(TextAnchor,
    "start" => TextAnchor::Start,
    "middle" => TextAnchor::Middle,
    "end" => TextAnchor::End
);

impl_enum_default!(AlignmentBaseline, Auto);

impl_enum_from_str!(AlignmentBaseline,
    "auto" => AlignmentBaseline::Auto,
    "baseline" => AlignmentBaseline::Baseline,
    "before-edge" => AlignmentBaseline::BeforeEdge,
    "text-before-edge" => AlignmentBaseline::TextBeforeEdge,
    "middle" => AlignmentBaseline::Middle,
    "central" => AlignmentBaseline::Central,
    "after-edge" => AlignmentBaseline::AfterEdge,
    "text-after-edge" => AlignmentBaseline::TextAfterEdge,
    "ideographic" => AlignmentBaseline::Ideographic,
    "alphabetic" => AlignmentBaseline::Alphabetic,
    "hanging" => AlignmentBaseline::Hanging,
    "mathematical" => AlignmentBaseline::Mathematical
);

impl_enum_default!(DominantBaseline, Auto);

impl_enum_from_str!(DominantBaseline,
    "auto" => DominantBaseline::Auto,
    "use-script" => DominantBaseline::UseScript,
    "no-change" => DominantBaseline::NoChange,
    "reset-size" => DominantBaseline::ResetSize,
    "ideographic" => DominantBaseline::Ideographic,
    "alphabetic" => DominantBaseline::Alphabetic,
    "hanging" => DominantBaseline::Hanging,
    "mathematical" => DominantBaseline::Mathematical,
    "central" => DominantBaseline::Central,
    "middle" => DominantBaseline::Middle,
    "text-after-edge" => DominantBaseline::TextAfterEdge,
    "text-before-edge" => DominantBaseline::TextBeforeEdge
);

impl_enum_default!(LengthAdjust, Spacing);

impl_enum_from_str!(LengthAdjust,
    "spacing" => LengthAdjust::Spacing,
    "spacingAndGlyphs" => LengthAdjust::SpacingAndGlyphs
);

impl crate::svgtree::EnumFromStr for Style {
    fn enum_from_str(s: &str) -> Option<Self> {
        match s {
            "normal" => Some(Style::Normal),
            "italic" => Some(Style::Italic),
            "oblique" => Some(Style::Oblique),
            _ => None,
        }
    }
}

pub(crate) fn convert(
    text_node: svgtree::Node,
    state: &converter::State,
    cache: &mut converter::Cache,
    parent: &mut Node,
) {
    let pos_list = resolve_positions_list(text_node, state);
    let rotate_list = resolve_rotate_list(text_node);
    let writing_mode = convert_writing_mode(text_node);

    let chunks = collect_text_chunks(text_node, &pos_list, state, cache);

    let rendering_mode: TextRendering = text_node
        .find_attribute(AId::TextRendering)
        .unwrap_or(state.opt.text_rendering);

    let text = Text {
        id: text_node.element_id().to_string(),
        transform: Transform::default(),
        rendering_mode,
        positions: pos_list,
        rotate: rotate_list,
        writing_mode,
        chunks,
    };
    parent.append_kind(NodeKind::Text(text));
}

struct IterState {
    chars_count: usize,
    chunk_bytes_count: usize,
    split_chunk: bool,
    text_flow: TextFlow,
    chunks: Vec<TextChunk>,
}

fn collect_text_chunks(
    text_node: svgtree::Node,
    pos_list: &[CharacterPosition],
    state: &converter::State,
    cache: &mut converter::Cache,
) -> Vec<TextChunk> {
    let mut iter_state = IterState {
        chars_count: 0,
        chunk_bytes_count: 0,
        split_chunk: false,
        text_flow: TextFlow::Linear,
        chunks: Vec::new(),
    };

    collect_text_chunks_impl(
        text_node,
        text_node,
        pos_list,
        state,
        cache,
        &mut iter_state,
    );

    iter_state.chunks
}

fn collect_text_chunks_impl(
    text_node: svgtree::Node,
    parent: svgtree::Node,
    pos_list: &[CharacterPosition],
    state: &converter::State,
    cache: &mut converter::Cache,
    iter_state: &mut IterState,
) {
    for child in parent.children() {
        if child.is_element() {
            if child.has_tag_name(EId::TextPath) {
                if !parent.has_tag_name(EId::Text) {
                    // `textPath` can be set only as a direct `text` element child.
                    iter_state.chars_count += count_chars(child);
                    continue;
                }

                match resolve_text_flow(child, state) {
                    Some(v) => {
                        iter_state.text_flow = v;
                    }
                    None => {
                        // Skip an invalid text path and all it's children.
                        // We have to update the chars count,
                        // because `pos_list` was calculated including this text path.
                        iter_state.chars_count += count_chars(child);
                        continue;
                    }
                }

                iter_state.split_chunk = true;
            }

            collect_text_chunks_impl(text_node, child, pos_list, state, cache, iter_state);

            iter_state.text_flow = TextFlow::Linear;

            // Next char after `textPath` should be split too.
            if child.has_tag_name(EId::TextPath) {
                iter_state.split_chunk = true;
            }

            continue;
        }

        if !parent.is_visible_element(state.opt) {
            iter_state.chars_count += child.text().chars().count();
            continue;
        }

        let anchor = parent.find_attribute(AId::TextAnchor).unwrap_or_default();

        // TODO: what to do when <= 0? UB?
        let font_size = crate::units::resolve_font_size(parent, state);
        let font_size = match NonZeroPositiveF64::new(font_size) {
            Some(n) => n,
            None => {
                // Skip this span.
                iter_state.chars_count += child.text().chars().count();
                continue;
            }
        };

        let font = convert_font(parent, state);

        let raw_paint_order: svgtypes::PaintOrder = parent
            .find_attribute(svgtree::AId::PaintOrder)
            .unwrap_or_default();
        let paint_order = crate::converter::svg_paint_order_to_usvg(raw_paint_order);

        let mut dominant_baseline = parent
            .find_attribute(AId::DominantBaseline)
            .unwrap_or_default();

        // `no-change` means "use parent".
        if dominant_baseline == DominantBaseline::NoChange {
            dominant_baseline = parent
                .parent_element()
                .unwrap()
                .find_attribute(AId::DominantBaseline)
                .unwrap_or_default();
        }

        let mut apply_kerning = true;
        if parent.resolve_length(AId::Kerning, state, -1.0) == 0.0 {
            apply_kerning = false;
        } else if parent.find_attribute::<&str>(AId::FontKerning) == Some("none") {
            apply_kerning = false;
        }

        let mut text_length =
            parent.try_convert_length(AId::TextLength, Units::UserSpaceOnUse, state);
        // Negative values should be ignored.
        if let Some(n) = text_length {
            if n < 0.0 {
                text_length = None;
            }
        }

        let span = TextSpan {
            start: 0,
            end: 0,
            fill: style::resolve_fill(parent, true, state, cache),
            stroke: style::resolve_stroke(parent, true, state, cache),
            paint_order,
            font,
            font_size,
            small_caps: parent.find_attribute(AId::FontVariant) == Some("small-caps"),
            apply_kerning,
            decoration: resolve_decoration(text_node, parent, state, cache),
            visibility: parent.find_attribute(AId::Visibility).unwrap_or_default(),
            dominant_baseline,
            alignment_baseline: parent
                .find_attribute(AId::AlignmentBaseline)
                .unwrap_or_default(),
            baseline_shift: convert_baseline_shift(parent, state),
            letter_spacing: parent.resolve_length(AId::LetterSpacing, state, 0.0),
            word_spacing: parent.resolve_length(AId::WordSpacing, state, 0.0),
            text_length,
            length_adjust: parent.find_attribute(AId::LengthAdjust).unwrap_or_default(),
        };

        let mut is_new_span = true;
        for c in child.text().chars() {
            let char_len = c.len_utf8();

            // Create a new chunk if:
            // - this is the first span (yes, position can be None)
            // - text character has an absolute coordinate assigned to it (via x/y attribute)
            // - `c` is the first char of the `textPath`
            // - `c` is the first char after `textPath`
            let is_new_chunk = pos_list[iter_state.chars_count].x.is_some()
                || pos_list[iter_state.chars_count].y.is_some()
                || iter_state.split_chunk
                || iter_state.chunks.is_empty();

            iter_state.split_chunk = false;

            if is_new_chunk {
                iter_state.chunk_bytes_count = 0;

                let mut span2 = span.clone();
                span2.start = 0;
                span2.end = char_len;

                iter_state.chunks.push(TextChunk {
                    x: pos_list[iter_state.chars_count].x,
                    y: pos_list[iter_state.chars_count].y,
                    anchor,
                    spans: vec![span2],
                    text_flow: iter_state.text_flow.clone(),
                    text: c.to_string(),
                });
            } else if is_new_span {
                // Add this span to the last text chunk.
                let mut span2 = span.clone();
                span2.start = iter_state.chunk_bytes_count;
                span2.end = iter_state.chunk_bytes_count + char_len;

                if let Some(chunk) = iter_state.chunks.last_mut() {
                    chunk.text.push(c);
                    chunk.spans.push(span2);
                }
            } else {
                // Extend the last span.
                if let Some(chunk) = iter_state.chunks.last_mut() {
                    chunk.text.push(c);
                    if let Some(span) = chunk.spans.last_mut() {
                        debug_assert_ne!(span.end, 0);
                        span.end += char_len;
                    }
                }
            }

            is_new_span = false;
            iter_state.chars_count += 1;
            iter_state.chunk_bytes_count += char_len;
        }
    }
}

fn resolve_text_flow(node: svgtree::Node, state: &converter::State) -> Option<TextFlow> {
    let linked_node = node.attribute::<svgtree::Node>(AId::Href)?;

    let path = match linked_node.tag_name()? {
        EId::Rect | EId::Circle | EId::Ellipse | EId::Line | EId::Polyline | EId::Polygon => {
            crate::shapes::convert(linked_node, state)?
        }
        EId::Path => linked_node.attribute::<SharedPathData>(AId::D)?,
        _ => return None,
    };

    // The reference path's transform needs to be applied
    let path = if let Some(node_transform) = linked_node.attribute::<Transform>(AId::Transform) {
        let mut path_copy = path.as_ref().clone();
        path_copy.transform(node_transform);
        Rc::new(path_copy)
    } else {
        path
    };

    let start_offset: Length = node.attribute(AId::StartOffset).unwrap_or_default();
    let start_offset = if start_offset.unit == LengthUnit::Percent {
        // 'If a percentage is given, then the `startOffset` represents
        // a percentage distance along the entire path.'
        let path_len = path.length();
        path_len * (start_offset.number / 100.0)
    } else {
        node.resolve_length(AId::StartOffset, state, 0.0)
    };

    Some(TextFlow::Path(Rc::new(TextPath { start_offset, path })))
}

fn convert_font(node: svgtree::Node, state: &converter::State) -> Font {
    let style: Style = node.find_attribute(AId::FontStyle).unwrap_or_default();
    let stretch = conv_font_stretch(node);
    let weight = resolve_font_weight(node);

    let font_family = if let Some(n) = node.find_node_with_attribute(AId::FontFamily) {
        n.attribute::<&str>(AId::FontFamily).unwrap_or("")
    } else {
        ""
    };

    let mut families = Vec::new();
    for mut family in font_family.split(',') {
        // TODO: to a proper parser

        if family.starts_with('\'') {
            family = &family[1..];
        }

        if family.ends_with('\'') {
            family = &family[..family.len() - 1];
        }

        family = family.trim();

        families.push(family.to_string());
    }

    if families.is_empty() {
        families.push(state.opt.font_family.clone())
    }

    Font {
        families,
        style,
        stretch,
        weight,
    }
}

// TODO: properly resolve narrower/wider
fn conv_font_stretch(node: svgtree::Node) -> Stretch {
    if let Some(n) = node.find_node_with_attribute(AId::FontStretch) {
        match n.attribute(AId::FontStretch).unwrap_or("") {
            "narrower" | "condensed" => Stretch::Condensed,
            "ultra-condensed" => Stretch::UltraCondensed,
            "extra-condensed" => Stretch::ExtraCondensed,
            "semi-condensed" => Stretch::SemiCondensed,
            "semi-expanded" => Stretch::SemiExpanded,
            "wider" | "expanded" => Stretch::Expanded,
            "extra-expanded" => Stretch::ExtraExpanded,
            "ultra-expanded" => Stretch::UltraExpanded,
            _ => Stretch::Normal,
        }
    } else {
        Stretch::Normal
    }
}

fn resolve_font_weight(node: svgtree::Node) -> u16 {
    fn bound(min: usize, val: usize, max: usize) -> usize {
        std::cmp::max(min, std::cmp::min(max, val))
    }

    let nodes: Vec<_> = node.ancestors().collect();
    let mut weight = 400;
    for n in nodes.iter().rev().skip(1) {
        // skip Root
        weight = match n.attribute(AId::FontWeight).unwrap_or("") {
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

    weight as u16
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
    text_node: svgtree::Node,
    state: &converter::State,
) -> Vec<CharacterPosition> {
    // Allocate a list that has all characters positions set to `None`.
    let total_chars = count_chars(text_node);
    let mut list = vec![
        CharacterPosition {
            x: None,
            y: None,
            dx: None,
            dy: None,
        };
        total_chars
    ];

    let mut offset = 0;
    for child in text_node.descendants() {
        if child.is_element() {
            let child_chars = count_chars(child);
            macro_rules! push_list {
                ($aid:expr, $field:ident) => {
                    if let Some(num_list) = crate::units::convert_list(child, $aid, state) {
                        // Note that we are using not the total count,
                        // but the amount of characters in the current `tspan` (with children).
                        let len = std::cmp::min(num_list.len(), child_chars);
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
fn resolve_rotate_list(text_node: svgtree::Node) -> Vec<f64> {
    // Allocate a list that has all characters angles set to `0.0`.
    let mut list = vec![0.0; count_chars(text_node)];
    let mut last = 0.0;
    let mut offset = 0;
    for child in text_node.descendants() {
        if child.is_element() {
            if let Some(rotate) = child.attribute::<&Vec<f64>>(AId::Rotate) {
                for i in 0..count_chars(child) {
                    if let Some(a) = rotate.get(i).cloned() {
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

/// Resolves node's `text-decoration` property.
///
/// `text` and `tspan` can point to the same node.
fn resolve_decoration(
    text_node: svgtree::Node,
    tspan: svgtree::Node,
    state: &converter::State,
    cache: &mut converter::Cache,
) -> TextDecoration {
    // TODO: explain the algorithm

    let text_dec = conv_text_decoration(text_node);
    let tspan_dec = conv_text_decoration2(tspan);

    let mut gen_style = |in_tspan: bool, in_text: bool| {
        let n = if in_tspan {
            tspan
        } else if in_text {
            text_node
        } else {
            return None;
        };

        Some(TextDecorationStyle {
            fill: style::resolve_fill(n, true, state, cache),
            stroke: style::resolve_stroke(n, true, state, cache),
        })
    };

    TextDecoration {
        underline: gen_style(tspan_dec.has_underline, text_dec.has_underline),
        overline: gen_style(tspan_dec.has_overline, text_dec.has_overline),
        line_through: gen_style(tspan_dec.has_line_through, text_dec.has_line_through),
    }
}

struct TextDecorationTypes {
    has_underline: bool,
    has_overline: bool,
    has_line_through: bool,
}

/// Resolves the `text` node's `text-decoration` property.
fn conv_text_decoration(text_node: svgtree::Node) -> TextDecorationTypes {
    fn find_decoration(node: svgtree::Node, value: &str) -> bool {
        node.ancestors().any(|n| {
            if let Some(str_value) = n.attribute::<&str>(AId::TextDecoration) {
                str_value.split(' ').any(|v| v == value)
            } else {
                false
            }
        })
    }

    TextDecorationTypes {
        has_underline: find_decoration(text_node, "underline"),
        has_overline: find_decoration(text_node, "overline"),
        has_line_through: find_decoration(text_node, "line-through"),
    }
}

/// Resolves the default `text-decoration` property.
fn conv_text_decoration2(tspan: svgtree::Node) -> TextDecorationTypes {
    let s = tspan.attribute(AId::TextDecoration);
    TextDecorationTypes {
        has_underline: s == Some("underline"),
        has_overline: s == Some("overline"),
        has_line_through: s == Some("line-through"),
    }
}

fn convert_baseline_shift(node: svgtree::Node, state: &converter::State) -> Vec<BaselineShift> {
    let mut shift = Vec::new();
    let nodes: Vec<_> = node
        .ancestors()
        .take_while(|n| !n.has_tag_name(EId::Text))
        .collect();
    for n in nodes {
        if let Some(len) = n.attribute::<Length>(AId::BaselineShift) {
            if len.unit == LengthUnit::Percent {
                let n = crate::units::resolve_font_size(n, state) * (len.number / 100.0);
                shift.push(BaselineShift::Number(n));
            } else {
                let n = crate::units::convert_length(
                    len,
                    n,
                    AId::BaselineShift,
                    Units::ObjectBoundingBox,
                    state,
                );
                shift.push(BaselineShift::Number(n));
            }
        } else if let Some(s) = n.attribute(AId::BaselineShift) {
            match s {
                "sub" => shift.push(BaselineShift::Subscript),
                "super" => shift.push(BaselineShift::Superscript),
                _ => shift.push(BaselineShift::Baseline),
            }
        }
    }

    if shift
        .iter()
        .all(|base| matches!(base, BaselineShift::Baseline))
    {
        shift.clear();
    }

    shift
}

fn count_chars(node: svgtree::Node) -> usize {
    node.descendants()
        .filter(|n| n.is_text())
        .fold(0, |w, n| w + n.text().chars().count())
}

/// Converts the writing mode.
///
/// [SVG 2] references [CSS Writing Modes Level 3] for the definition of the
/// 'writing-mode' property, there are only two writing modes:
/// horizontal left-to-right and vertical right-to-left.
///
/// That specification introduces new values for the property. The SVG 1.1
/// values are obsolete but must still be supported by converting the specified
/// values to computed values as follows:
///
/// - `lr`, `lr-tb`, `rl`, `rl-tb` => `horizontal-tb`
/// - `tb`, `tb-rl` => `vertical-rl`
///
/// The current `vertical-lr` behaves exactly the same as `vertical-rl`.
///
/// Also, looks like no one really supports the `rl` and `rl-tb`, except `Batik`.
/// And I'm not sure if its behaviour is correct.
///
/// So we will ignore it as well, mainly because I have no idea how exactly
/// it should affect the rendering.
///
/// [SVG 2]: https://www.w3.org/TR/SVG2/text.html#WritingModeProperty
/// [CSS Writing Modes Level 3]: https://www.w3.org/TR/css-writing-modes-3/#svg-writing-mode-css
fn convert_writing_mode(text_node: svgtree::Node) -> WritingMode {
    if let Some(n) = text_node.find_node_with_attribute(AId::WritingMode) {
        match n.attribute(AId::WritingMode).unwrap_or("lr-tb") {
            "tb" | "tb-rl" | "vertical-rl" | "vertical-lr" => WritingMode::TopToBottom,
            _ => WritingMode::LeftToRight,
        }
    } else {
        WritingMode::LeftToRight
    }
}
