// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgtypes::{Length, LengthUnit as Unit};

use crate::svgtree::{self, AId, EId};
use crate::{Color, NodeExt, NodeKind, NormalizedValue, Opacity, OptionLog, PositiveNumber, Tree, Units, converter};
use crate::geom::{FuzzyEq, FuzzyZero, IsValidLength, Line, Rect, Transform, ViewBox};


/// A spread method.
///
/// `spreadMethod` attribute in the SVG.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SpreadMethod {
    Pad,
    Reflect,
    Repeat,
}

impl_enum_default!(SpreadMethod, Pad);

impl_enum_from_str!(SpreadMethod,
    "pad"       => SpreadMethod::Pad,
    "reflect"   => SpreadMethod::Reflect,
    "repeat"    => SpreadMethod::Repeat
);


/// A generic gradient.
#[derive(Clone, Debug)]
pub struct BaseGradient {
    /// Coordinate system units.
    ///
    /// `gradientUnits` in SVG.
    pub units: Units,

    /// Gradient transform.
    ///
    /// `gradientTransform` in SVG.
    pub transform: Transform,

    /// Gradient spreading method.
    ///
    /// `spreadMethod` in SVG.
    pub spread_method: SpreadMethod,

    /// A list of `stop` elements.
    pub stops: Vec<Stop>,
}


/// A linear gradient.
///
/// `linearGradient` element in SVG.
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct LinearGradient {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Can't be empty.
    pub id: String,

    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,

    /// Base gradient data.
    pub base: BaseGradient,
}

impl std::ops::Deref for LinearGradient {
    type Target = BaseGradient;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}


/// A radial gradient.
///
/// `radialGradient` element in SVG.
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct RadialGradient {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Can't be empty.
    pub id: String,

    pub cx: f64,
    pub cy: f64,
    pub r: PositiveNumber,
    pub fx: f64,
    pub fy: f64,

    /// Base gradient data.
    pub base: BaseGradient,
}

impl std::ops::Deref for RadialGradient {
    type Target = BaseGradient;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

/// An alias to `NormalizedValue`.
pub type StopOffset = NormalizedValue;

/// Gradient's stop element.
///
/// `stop` element in SVG.
#[derive(Clone, Copy, Debug)]
pub struct Stop {
    /// Gradient stop offset.
    ///
    /// `offset` in SVG.
    pub offset: StopOffset,

    /// Gradient stop color.
    ///
    /// `stop-color` in SVG.
    pub color: Color,

    /// Gradient stop opacity.
    ///
    /// `stop-opacity` in SVG.
    pub opacity: Opacity,
}


/// A pattern element.
///
/// `pattern` element in SVG.
#[derive(Clone, Debug)]
pub struct Pattern {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Can't be empty.
    pub id: String,

    /// Coordinate system units.
    ///
    /// `patternUnits` in SVG.
    pub units: Units,

    // TODO: should not be accessible when `viewBox` is present.
    /// Content coordinate system units.
    ///
    /// `patternContentUnits` in SVG.
    pub content_units: Units,

    /// Pattern transform.
    ///
    /// `patternTransform` in SVG.
    pub transform: Transform,

    /// Pattern rectangle.
    ///
    /// `x`, `y`, `width` and `height` in SVG.
    pub rect: Rect,

    /// Pattern viewbox.
    pub view_box: Option<ViewBox>,
}


pub(crate) enum ServerOrColor {
    Server {
        id: String,
        units: Units,
    },
    Color {
        color: Color,
        opacity: Opacity,
    },
}

pub(crate) fn convert(
    node: svgtree::Node,
    state: &converter::State,
    id_generator: &mut converter::NodeIdGenerator,
    tree: &mut Tree,
) -> Option<ServerOrColor> {
    // Check for existing.
    if let Some(existing_node) = tree.defs_by_id(node.element_id()) {
        return Some(ServerOrColor::Server {
            id: node.element_id().to_string(),
            units: existing_node.units()?,
        });
    }

    // Unwrap is safe, because we already checked for is_paint_server().
    match node.tag_name().unwrap() {
        EId::LinearGradient => convert_linear(node, state, tree),
        EId::RadialGradient => convert_radial(node, state, tree),
        EId::Pattern => convert_pattern(node, state, id_generator, tree),
        _ => unreachable!(),
    }
}

#[inline(never)]
fn convert_linear(
    node: svgtree::Node,
    state: &converter::State,
    tree: &mut Tree,
) -> Option<ServerOrColor> {
    let stops = convert_stops(find_gradient_with_stops(node)?);
    if stops.len() < 2 {
        return stops_to_color(&stops);
    }

    let units = convert_units(node, AId::GradientUnits, Units::ObjectBoundingBox);
    let transform = resolve_attr(node, AId::GradientTransform)
        .attribute(AId::GradientTransform).unwrap_or_default();

    tree.append_to_defs(
        NodeKind::LinearGradient(LinearGradient {
            id: node.element_id().to_string(),
            x1: resolve_number(node, AId::X1, units, state, Length::zero()),
            y1: resolve_number(node, AId::Y1, units, state, Length::zero()),
            x2: resolve_number(node, AId::X2, units, state, Length::new(100.0, Unit::Percent)),
            y2: resolve_number(node, AId::Y2, units, state, Length::zero()),
            base: BaseGradient {
                units,
                transform,
                spread_method: convert_spread_method(node),
                stops,
            }
        })
    );

    Some(ServerOrColor::Server {
        id: node.element_id().to_string(),
        units,
    })
}

#[inline(never)]
fn convert_radial(
    node: svgtree::Node,
    state: &converter::State,
    tree: &mut Tree,
) -> Option<ServerOrColor> {
    let stops = convert_stops(find_gradient_with_stops(node)?);
    if stops.len() < 2 {
        return stops_to_color(&stops);
    }

    let units = convert_units(node, AId::GradientUnits, Units::ObjectBoundingBox);
    let r = resolve_number(node, AId::R, units, state, Length::new(50.0, Unit::Percent));

    // 'A value of zero will cause the area to be painted as a single color
    // using the color and opacity of the last gradient stop.'
    //
    // https://www.w3.org/TR/SVG11/pservers.html#RadialGradientElementRAttribute
    if !r.is_valid_length() {
        let stop = stops.last().unwrap();
        return Some(ServerOrColor::Color {
            color: stop.color,
            opacity: stop.opacity,
        });
    }

    let spread_method = convert_spread_method(node);
    let cx = resolve_number(node, AId::Cx, units, state, Length::new(50.0, Unit::Percent));
    let cy = resolve_number(node, AId::Cy, units, state, Length::new(50.0, Unit::Percent));
    let fx = resolve_number(node, AId::Fx, units, state, Length::new_number(cx));
    let fy = resolve_number(node, AId::Fy, units, state, Length::new_number(cy));
    let (fx, fy) = prepare_focal(cx, cy, r, fx, fy);
    let transform = resolve_attr(node, AId::GradientTransform)
        .attribute(AId::GradientTransform).unwrap_or_default();

    tree.append_to_defs(
        NodeKind::RadialGradient(RadialGradient {
            id: node.element_id().to_string(),
            cx,
            cy,
            r: r.into(),
            fx,
            fy,
            base: BaseGradient {
                units,
                transform,
                spread_method,
                stops,
            }
        })
    );

    Some(ServerOrColor::Server {
        id: node.element_id().to_string(),
        units,
    })
}

#[inline(never)]
fn convert_pattern(
    node: svgtree::Node,
    state: &converter::State,
    id_generator: &mut converter::NodeIdGenerator,
    tree: &mut Tree,
) -> Option<ServerOrColor> {
    let node_with_children = find_pattern_with_children(node)?;

    let view_box = {
        let n1 = resolve_attr(node, AId::ViewBox);
        let n2 = resolve_attr(node, AId::PreserveAspectRatio);
        n1.get_viewbox().map(|vb|
            ViewBox {
                rect: vb,
                aspect: n2.attribute(AId::PreserveAspectRatio).unwrap_or_default(),
            }
        )
    };

    let units = convert_units(node, AId::PatternUnits, Units::ObjectBoundingBox);
    let content_units = convert_units(node, AId::PatternContentUnits, Units::UserSpaceOnUse);

    let transform = resolve_attr(node, AId::PatternTransform)
        .attribute(AId::PatternTransform).unwrap_or_default();

    let rect = Rect::new(
        resolve_number(node, AId::X, units, state, Length::zero()),
        resolve_number(node, AId::Y, units, state, Length::zero()),
        resolve_number(node, AId::Width, units, state, Length::zero()),
        resolve_number(node, AId::Height, units, state, Length::zero()),
    );
    let rect = rect.log_none(|| log::warn!("Pattern '{}' has an invalid size. Skipped.", node.element_id()))?;

    let mut patt = tree.append_to_defs(NodeKind::Pattern(Pattern {
        id: node.element_id().to_string(),
        units,
        content_units,
        transform,
        rect,
        view_box,
    }));

    converter::convert_children(node_with_children, state, id_generator, &mut patt, tree);

    if !patt.has_children() {
        return None;
    }

    Some(ServerOrColor::Server {
        id: node.element_id().to_string(),
        units,
    })
}

fn convert_spread_method(node: svgtree::Node) -> SpreadMethod {
    let node = resolve_attr(node, AId::SpreadMethod);
    node.attribute(AId::SpreadMethod).unwrap_or_default()
}

pub(crate) fn convert_units(
    node: svgtree::Node,
    name: AId,
    def: Units,
) -> Units {
    let node = resolve_attr(node, name);
    node.attribute(name).unwrap_or(def)
}

fn find_gradient_with_stops(node: svgtree::Node) -> Option<svgtree::Node> {
    for link_id in node.href_iter() {
        let link = node.document().get(link_id);
        if !link.tag_name().unwrap().is_gradient() {
            log::warn!(
                "Gradient '{}' cannot reference '{}' via 'xlink:href'.",
                node.element_id(), link.tag_name().unwrap()
            );
            return None;
        }

        if link.children().any(|n| n.has_tag_name(EId::Stop)) {
            return Some(link);
        }
    }

    None
}

fn find_pattern_with_children(node: svgtree::Node) -> Option<svgtree::Node> {
    for link_id in node.href_iter() {
        let link = node.document().get(link_id);
        if !link.has_tag_name(EId::Pattern) {
            log::warn!(
                "Pattern '{}' cannot reference '{}' via 'xlink:href'.",
                node.element_id(), link.tag_name().unwrap()
            );
            return None;
        }

        if link.has_children() {
            return Some(link);
        }
    }

    None
}

fn convert_stops(grad: svgtree::Node) -> Vec<Stop> {
    let mut stops = Vec::new();

    {
        let mut prev_offset = Length::zero();
        for stop in grad.children() {
            if !stop.has_tag_name(EId::Stop) {
                log::warn!("Invalid gradient child: '{:?}'.", stop.tag_name().unwrap());
                continue;
            }

            // `number` can be either a number or a percentage.
            let offset = stop.attribute(AId::Offset).unwrap_or(prev_offset);
            let offset = match offset.unit {
                Unit::None => offset.number,
                Unit::Percent => offset.number / 100.0,
                _ => prev_offset.number,
            };
            let offset = crate::utils::f64_bound(0.0, offset, 1.0);
            prev_offset = Length::new_number(offset);

            let color = match stop.attribute(AId::StopColor) {
                Some(&svgtree::AttributeValue::CurrentColor) => {
                    stop.find_attribute(AId::Color).unwrap_or_else(Color::black)
                }
                Some(&svgtree::AttributeValue::Color(c)) => {
                    c
                }
                _ => {
                    svgtypes::Color::black()
                }
            };

            stops.push(Stop {
                offset: offset.into(),
                color,
                opacity: stop.attribute(AId::StopOpacity).unwrap_or_default(),
            });
        }
    }

    // Remove stops with equal offset.
    //
    // Example:
    // offset="0.5"
    // offset="0.7"
    // offset="0.7" <-- this one should be removed
    // offset="0.7"
    // offset="0.9"
    if stops.len() >= 3 {
        let mut i = 0;
        while i < stops.len() - 2 {
            let offset1 = stops[i + 0].offset.value();
            let offset2 = stops[i + 1].offset.value();
            let offset3 = stops[i + 2].offset.value();

            if offset1.fuzzy_eq(&offset2) && offset2.fuzzy_eq(&offset3) {
                // Remove offset in the middle.
                stops.remove(i + 1);
            } else {
                i += 1;
            }
        }
    }

    // Remove zeros.
    //
    // From:
    // offset="0.0"
    // offset="0.0"
    // offset="0.7"
    //
    // To:
    // offset="0.0"
    // offset="0.00000001"
    // offset="0.7"
    if stops.len() >= 2 {
        let mut i = 0;
        while i < stops.len() - 1 {
            let offset1 = stops[i + 0].offset.value();
            let offset2 = stops[i + 1].offset.value();

            if offset1.is_fuzzy_zero() && offset2.is_fuzzy_zero() {
                stops[i + 1].offset = (offset1 + f64::EPSILON).into();
            }

            i += 1;
        }
    }

    // Shift equal offsets.
    //
    // From:
    // offset="0.5"
    // offset="0.7"
    // offset="0.7"
    //
    // To:
    // offset="0.5"
    // offset="0.699999999"
    // offset="0.7"
    {
        let mut i = 1;
        while i < stops.len() {
            let offset1 = stops[i - 1].offset.value();
            let offset2 = stops[i - 0].offset.value();

            // Next offset must be smaller then previous.
            if offset1 > offset2 || offset1.fuzzy_eq(&offset2) {
                // Make previous offset a bit smaller.
                let new_offset = offset1 - f64::EPSILON;
                stops[i - 1].offset = crate::utils::f64_bound(0.0, new_offset, 1.0).into();
                stops[i - 0].offset = offset1.into();
            }

            i += 1;
        }
    }

    stops
}

#[inline(never)]
pub(crate) fn resolve_number(
    node: svgtree::Node, name: AId, units: Units, state: &converter::State, def: Length
) -> f64 {
    resolve_attr(node, name).convert_length(name, units, state, def)
}

fn resolve_attr(
    node: svgtree::Node,
    name: AId,
) -> svgtree::Node {
    if node.has_attribute(name) {
        return node;
    }

    match node.tag_name().unwrap() {
        EId::LinearGradient => resolve_lg_attr(node, name),
        EId::RadialGradient => resolve_rg_attr(node, name),
        EId::Pattern => resolve_pattern_attr(node, name),
        EId::Filter => resolve_filter_attr(node, name),
        _ => node,
    }
}

fn resolve_lg_attr(
    node: svgtree::Node,
    name: AId,
) -> svgtree::Node {
    for link_id in node.href_iter() {
        let link = node.document().get(link_id);
        let tag_name = match link.tag_name() {
            Some(v) => v,
            None => return node,
        };

        match (name, tag_name) {
            // Coordinates can be resolved only from
            // ref element with the same type.
              (AId::X1, EId::LinearGradient)
            | (AId::Y1, EId::LinearGradient)
            | (AId::X2, EId::LinearGradient)
            | (AId::Y2, EId::LinearGradient)
            // Other attributes can be resolved
            // from any kind of gradient.
            | (AId::GradientUnits, EId::LinearGradient)
            | (AId::GradientUnits, EId::RadialGradient)
            | (AId::SpreadMethod, EId::LinearGradient)
            | (AId::SpreadMethod, EId::RadialGradient)
            | (AId::GradientTransform, EId::LinearGradient)
            | (AId::GradientTransform, EId::RadialGradient) => {
                if link.has_attribute(name) {
                    return link;
                }
            }
            _ => break,
        }
    }

    node
}

fn resolve_rg_attr(
    node: svgtree::Node,
    name: AId,
) -> svgtree::Node {
    for link_id in node.href_iter() {
        let link = node.document().get(link_id);
        let tag_name = match link.tag_name() {
            Some(v) => v,
            None => return node,
        };

        match (name, tag_name) {
            // Coordinates can be resolved only from
            // ref element with the same type.
              (AId::Cx, EId::RadialGradient)
            | (AId::Cy, EId::RadialGradient)
            | (AId::R,  EId::RadialGradient)
            | (AId::Fx, EId::RadialGradient)
            | (AId::Fy, EId::RadialGradient)
            // Other attributes can be resolved
            // from any kind of gradient.
            | (AId::GradientUnits, EId::LinearGradient)
            | (AId::GradientUnits, EId::RadialGradient)
            | (AId::SpreadMethod, EId::LinearGradient)
            | (AId::SpreadMethod, EId::RadialGradient)
            | (AId::GradientTransform, EId::LinearGradient)
            | (AId::GradientTransform, EId::RadialGradient) => {
                if link.has_attribute(name) {
                    return link;
                }
            }
            _ => break,
        }
    }

    node
}

fn resolve_pattern_attr(
    node: svgtree::Node,
    name: AId,
) -> svgtree::Node {
    for link_id in node.href_iter() {
        let link = node.document().get(link_id);
        let tag_name = match link.tag_name() {
            Some(v) => v,
            None => return node,
        };

        if tag_name != EId::Pattern {
            break;
        }

        if link.has_attribute(name) {
            return link;
        }
    }

    node
}

fn resolve_filter_attr(
    node: svgtree::Node,
    aid: AId,
) -> svgtree::Node {
    for link_id in node.href_iter() {
        let link = node.document().get(link_id);
        let tag_name = match link.tag_name() {
            Some(v) => v,
            None => return node,
        };

        if tag_name != EId::Filter {
            break;
        }

        if link.has_attribute(aid) {
            return link;
        }
    }

    node
}

/// Prepares the radial gradient focal radius.
///
/// According to the SVG spec:
///
/// If the point defined by `fx` and `fy` lies outside the circle defined by
/// `cx`, `cy` and `r`, then the user agent shall set the focal point to the
/// intersection of the line from (`cx`, `cy`) to (`fx`, `fy`) with the circle
/// defined by `cx`, `cy` and `r`.
fn prepare_focal(cx: f64, cy: f64, r: f64, fx: f64, fy: f64) -> (f64, f64) {
    let max_r = r - r * 0.001;

    let mut line = Line::new(cx, cy, fx, fy);

    if line.length() > max_r {
        line.set_length(max_r);
    }

    (line.x2, line.y2)
}

fn stops_to_color(stops: &[Stop]) -> Option<ServerOrColor> {
    if stops.is_empty() {
        None
    } else {
        Some(ServerOrColor::Color {
            color: stops[0].color,
            opacity: stops[0].opacity,
        })
    }
}
