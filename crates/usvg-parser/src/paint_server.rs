// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::rc::Rc;
use std::str::FromStr;

use strict_num::PositiveF32;
use svgtypes::{Length, LengthUnit as Unit};
use usvg_tree::*;

use crate::converter::{self, SvgColorExt};
use crate::svgtree::{AId, EId, SvgNode};
use crate::OptionLog;

pub(crate) enum ServerOrColor {
    Server(Paint),
    Color { color: Color, opacity: Opacity },
}

pub(crate) fn convert(
    node: SvgNode,
    state: &converter::State,
    cache: &mut converter::Cache,
) -> Option<ServerOrColor> {
    // Check for existing.
    if let Some(paint) = cache.paint.get(node.element_id()) {
        return Some(ServerOrColor::Server(paint.clone()));
    }

    // Unwrap is safe, because we already checked for is_paint_server().
    let paint = match node.tag_name().unwrap() {
        EId::LinearGradient => convert_linear(node, state),
        EId::RadialGradient => convert_radial(node, state),
        EId::Pattern => convert_pattern(node, state, cache),
        _ => unreachable!(),
    };

    if let Some(ServerOrColor::Server(ref paint)) = paint {
        cache
            .paint
            .insert(node.element_id().to_string(), paint.clone());
    }

    paint
}

#[inline(never)]
fn convert_linear(node: SvgNode, state: &converter::State) -> Option<ServerOrColor> {
    let stops = convert_stops(find_gradient_with_stops(node)?);
    if stops.len() < 2 {
        return stops_to_color(&stops);
    }

    let units = convert_units(node, AId::GradientUnits, Units::ObjectBoundingBox);
    let transform = node.resolve_transform(AId::GradientTransform, state);

    let gradient = LinearGradient {
        id: node.element_id().to_string(),
        x1: resolve_number(node, AId::X1, units, state, Length::zero()),
        y1: resolve_number(node, AId::Y1, units, state, Length::zero()),
        x2: resolve_number(
            node,
            AId::X2,
            units,
            state,
            Length::new(100.0, Unit::Percent),
        ),
        y2: resolve_number(node, AId::Y2, units, state, Length::zero()),
        base: BaseGradient {
            units,
            transform,
            spread_method: convert_spread_method(node),
            stops,
        },
    };

    Some(ServerOrColor::Server(Paint::LinearGradient(Rc::new(
        gradient,
    ))))
}

#[inline(never)]
fn convert_radial(node: SvgNode, state: &converter::State) -> Option<ServerOrColor> {
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
    let cx = resolve_number(
        node,
        AId::Cx,
        units,
        state,
        Length::new(50.0, Unit::Percent),
    );
    let cy = resolve_number(
        node,
        AId::Cy,
        units,
        state,
        Length::new(50.0, Unit::Percent),
    );
    let fx = resolve_number(node, AId::Fx, units, state, Length::new_number(cx as f64));
    let fy = resolve_number(node, AId::Fy, units, state, Length::new_number(cy as f64));
    let transform = node.resolve_transform(AId::GradientTransform, state);

    let gradient = RadialGradient {
        id: node.element_id().to_string(),
        cx,
        cy,
        r: PositiveF32::new(r).unwrap(),
        fx,
        fy,
        base: BaseGradient {
            units,
            transform,
            spread_method,
            stops,
        },
    };

    Some(ServerOrColor::Server(Paint::RadialGradient(Rc::new(
        gradient,
    ))))
}

#[inline(never)]
fn convert_pattern(
    node: SvgNode,
    state: &converter::State,
    cache: &mut converter::Cache,
) -> Option<ServerOrColor> {
    let node_with_children = find_pattern_with_children(node)?;

    let view_box = {
        let n1 = resolve_attr(node, AId::ViewBox);
        let n2 = resolve_attr(node, AId::PreserveAspectRatio);
        n1.parse_viewbox().map(|vb| ViewBox {
            rect: vb,
            aspect: n2.attribute(AId::PreserveAspectRatio).unwrap_or_default(),
        })
    };

    let units = convert_units(node, AId::PatternUnits, Units::ObjectBoundingBox);
    let content_units = convert_units(node, AId::PatternContentUnits, Units::UserSpaceOnUse);

    let transform = node.resolve_transform(AId::PatternTransform, state);

    let rect = NonZeroRect::from_xywh(
        resolve_number(node, AId::X, units, state, Length::zero()),
        resolve_number(node, AId::Y, units, state, Length::zero()),
        resolve_number(node, AId::Width, units, state, Length::zero()),
        resolve_number(node, AId::Height, units, state, Length::zero()),
    );
    let rect = rect.log_none(|| {
        log::warn!(
            "Pattern '{}' has an invalid size. Skipped.",
            node.element_id()
        )
    })?;

    let mut patt = Pattern {
        id: node.element_id().to_string(),
        units,
        content_units,
        transform,
        rect,
        view_box,
        root: Node::new(NodeKind::Group(Group::default())),
    };

    converter::convert_children(node_with_children, state, cache, &mut patt.root);

    if !patt.root.has_children() {
        return None;
    }

    Some(ServerOrColor::Server(Paint::Pattern(Rc::new(patt))))
}

fn convert_spread_method(node: SvgNode) -> SpreadMethod {
    let node = resolve_attr(node, AId::SpreadMethod);
    node.attribute(AId::SpreadMethod).unwrap_or_default()
}

pub(crate) fn convert_units(node: SvgNode, name: AId, def: Units) -> Units {
    let node = resolve_attr(node, name);
    node.attribute(name).unwrap_or(def)
}

fn find_gradient_with_stops<'a, 'input: 'a>(
    node: SvgNode<'a, 'input>,
) -> Option<SvgNode<'a, 'input>> {
    for link in node.href_iter() {
        if !link.tag_name().unwrap().is_gradient() {
            log::warn!(
                "Gradient '{}' cannot reference '{}' via 'xlink:href'.",
                node.element_id(),
                link.tag_name().unwrap()
            );
            return None;
        }

        if link.children().any(|n| n.tag_name() == Some(EId::Stop)) {
            return Some(link);
        }
    }

    None
}

fn find_pattern_with_children<'a, 'input: 'a>(
    node: SvgNode<'a, 'input>,
) -> Option<SvgNode<'a, 'input>> {
    for link in node.href_iter() {
        if link.tag_name() != Some(EId::Pattern) {
            log::warn!(
                "Pattern '{}' cannot reference '{}' via 'xlink:href'.",
                node.element_id(),
                link.tag_name().unwrap()
            );
            return None;
        }

        if link.has_children() {
            return Some(link);
        }
    }

    None
}

fn convert_stops(grad: SvgNode) -> Vec<Stop> {
    let mut stops = Vec::new();

    {
        let mut prev_offset = Length::zero();
        for stop in grad.children() {
            if stop.tag_name() != Some(EId::Stop) {
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
            prev_offset = Length::new_number(offset);
            let offset = crate::f32_bound(0.0, offset as f32, 1.0);

            let (color, opacity) = match stop.attribute(AId::StopColor) {
                Some("currentColor") => stop
                    .find_attribute(AId::Color)
                    .unwrap_or_else(svgtypes::Color::black),
                Some(value) => {
                    if let Ok(c) = svgtypes::Color::from_str(value) {
                        c
                    } else {
                        log::warn!("Failed to parse stop-color value: '{}'.", value);
                        svgtypes::Color::black()
                    }
                }
                _ => svgtypes::Color::black(),
            }
            .split_alpha();

            let stop_opacity = stop
                .attribute::<Opacity>(AId::StopOpacity)
                .unwrap_or(Opacity::ONE);
            stops.push(Stop {
                offset: StopOffset::new_clamped(offset),
                color,
                opacity: opacity * stop_opacity,
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
            let offset1 = stops[i + 0].offset.get();
            let offset2 = stops[i + 1].offset.get();
            let offset3 = stops[i + 2].offset.get();

            if offset1.approx_eq_ulps(&offset2, 4) && offset2.approx_eq_ulps(&offset3, 4) {
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
            let offset1 = stops[i + 0].offset.get();
            let offset2 = stops[i + 1].offset.get();

            if offset1.approx_eq_ulps(&0.0, 4) && offset2.approx_eq_ulps(&0.0, 4) {
                stops[i + 1].offset = StopOffset::new_clamped(offset1 + f32::EPSILON);
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
            let offset1 = stops[i - 1].offset.get();
            let offset2 = stops[i - 0].offset.get();

            // Next offset must be smaller then previous.
            if offset1 > offset2 || offset1.approx_eq_ulps(&offset2, 4) {
                // Make previous offset a bit smaller.
                let new_offset = offset1 - f32::EPSILON;
                stops[i - 1].offset = StopOffset::new_clamped(new_offset);
                stops[i - 0].offset = StopOffset::new_clamped(offset1);
            }

            i += 1;
        }
    }

    stops
}

#[inline(never)]
pub(crate) fn resolve_number(
    node: SvgNode,
    name: AId,
    units: Units,
    state: &converter::State,
    def: Length,
) -> f32 {
    resolve_attr(node, name).convert_length(name, units, state, def)
}

fn resolve_attr<'a, 'input: 'a>(node: SvgNode<'a, 'input>, name: AId) -> SvgNode<'a, 'input> {
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

fn resolve_lg_attr<'a, 'input: 'a>(node: SvgNode<'a, 'input>, name: AId) -> SvgNode<'a, 'input> {
    for link in node.href_iter() {
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

fn resolve_rg_attr<'a, 'input>(node: SvgNode<'a, 'input>, name: AId) -> SvgNode<'a, 'input> {
    for link in node.href_iter() {
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

fn resolve_pattern_attr<'a, 'input: 'a>(
    node: SvgNode<'a, 'input>,
    name: AId,
) -> SvgNode<'a, 'input> {
    for link in node.href_iter() {
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

fn resolve_filter_attr<'a, 'input: 'a>(node: SvgNode<'a, 'input>, aid: AId) -> SvgNode<'a, 'input> {
    for link in node.href_iter() {
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
