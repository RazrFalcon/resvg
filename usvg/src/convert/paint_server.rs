// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::f64;

// external
use svgdom;

// self
use crate::tree;
use super::prelude::*;


pub enum ServerOrColor {
    Server {
        id: String,
        units: tree::Units,
    },
    Color {
        color: tree::Color,
        opacity: tree::Opacity,
    },
}

pub fn convert(
    node: &svgdom::Node,
    state: &State,
    tree: &mut tree::Tree,
) -> Option<ServerOrColor> {
    // Check for existing.
    if let Some(exist_node) = tree.defs_by_id(node.id().as_str()) {
        let units = match *exist_node.borrow() {
            tree::NodeKind::LinearGradient(ref lg) => lg.units,
            tree::NodeKind::RadialGradient(ref rg) => rg.units,
            tree::NodeKind::Pattern(ref patt) => patt.units,
            _ => return None, // Unreachable.
        };

        return Some(ServerOrColor::Server {
            id: node.id().to_string(),
            units,
        });
    }

    // Unwrap is safe, because we already checked for is_paint_server().
    match node.tag_id().unwrap() {
        EId::LinearGradient => convert_linear(node, state, tree),
        EId::RadialGradient => convert_radial(node, state, tree),
        EId::Pattern => convert_pattern(node, state, tree),
        _ => unreachable!(),
    }
}

fn convert_linear(
    node: &svgdom::Node,
    state: &State,
    tree: &mut tree::Tree,
) -> Option<ServerOrColor> {
    let stops = convert_stops(&find_gradient_with_stops(node)?);
    if stops.len() < 2 {
        return stops_to_color(&stops);
    }

    let units = convert_units(node, AId::GradientUnits, tree::Units::ObjectBoundingBox);
    let spread_method = convert_spread_method(node);
    let x1 = resolve_number(node, AId::X1, units, state, Length::zero());
    let y1 = resolve_number(node, AId::Y1, units, state, Length::zero());
    let x2 = resolve_number(node, AId::X2, units, state, Length::new(100.0, Unit::Percent));
    let y2 = resolve_number(node, AId::Y2, units, state, Length::zero());
    let transform = {
        let n = resolve_attr(node, AId::GradientTransform);
        let attrs = n.attributes();
        attrs.get_transform(AId::GradientTransform)
    };

    tree.append_to_defs(
        tree::NodeKind::LinearGradient(tree::LinearGradient {
            id: node.id().clone(),
            x1,
            y1,
            x2,
            y2,
            base: tree::BaseGradient {
                units,
                transform,
                spread_method,
                stops,
            }
        })
    );

    Some(ServerOrColor::Server {
        id: node.id().clone(),
        units,
    })
}

fn convert_radial(
    node: &svgdom::Node,
    state: &State,
    tree: &mut tree::Tree,
) -> Option<ServerOrColor> {
    let stops = convert_stops(&find_gradient_with_stops(node)?);
    if stops.len() < 2 {
        return stops_to_color(&stops);
    }

    let units = convert_units(node, AId::GradientUnits, tree::Units::ObjectBoundingBox);
    let r = resolve_number(node, AId::R, units, state, Length::new(50.0, Unit::Percent));

    // 'A value of zero will cause the area to be painted as a single color
    // using the color and opacity of the last gradient stop.'
    //
    // https://www.w3.org/TR/SVG11/pservers.html#RadialGradientElementRAttribute
    if !(r > 0.0) {
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
    let transform = {
        let n = resolve_attr(node, AId::GradientTransform);
        let attrs = n.attributes();
        attrs.get_transform(AId::GradientTransform)
    };

    tree.append_to_defs(
        tree::NodeKind::RadialGradient(tree::RadialGradient {
            id: node.id().clone(),
            cx,
            cy,
            r: r.into(),
            fx,
            fy,
            base: tree::BaseGradient {
                units,
                transform,
                spread_method,
                stops,
            }
        })
    );

    Some(ServerOrColor::Server {
        id: node.id().clone(),
        units,
    })
}

fn convert_pattern(
    node: &svgdom::Node,
    state: &State,
    tree: &mut tree::Tree,
) -> Option<ServerOrColor> {
    let node_with_children = find_pattern_with_children(node)?;

    let ref attrs = node.attributes();

    let view_box = node.get_viewbox().map(|vb|
        tree::ViewBox {
            rect: vb,
            aspect: super::convert_aspect(attrs), // TODO: via href?
        }
    );

    let units = convert_units(node, AId::PatternUnits, tree::Units::ObjectBoundingBox);
    let content_units = convert_units(node, AId::PatternContentUnits, tree::Units::UserSpaceOnUse);

    let transform = {
        let n = resolve_attr(node, AId::PatternTransform);
        let attrs = n.attributes();
        attrs.get_transform(AId::PatternTransform)
    };

    let rect = Rect::new(
        resolve_number(node, AId::X, units, state, Length::zero()),
        resolve_number(node, AId::Y, units, state, Length::zero()),
        resolve_number(node, AId::Width, units, state, Length::zero()),
        resolve_number(node, AId::Height, units, state, Length::zero()),
    );
    let rect = try_opt_warn_or!(
        rect, None,
        "Pattern '{}' has an invalid size. Skipped.", node.id()
    );

    let mut patt = tree.append_to_defs(tree::NodeKind::Pattern(tree::Pattern {
        id: node.id().clone(),
        units,
        content_units,
        transform,
        rect,
        view_box,
    }));

    super::convert_children(&node_with_children, state, &mut patt, tree);

    if !patt.has_children() {
        return None;
    }

    Some(ServerOrColor::Server {
        id: node.id().clone(),
        units,
    })
}

fn convert_spread_method(node: &svgdom::Node) -> tree::SpreadMethod {
    let node = resolve_attr(node, AId::SpreadMethod);
    let attrs = node.attributes();

    match attrs.get_str_or(AId::SpreadMethod, "pad") {
        "pad" => tree::SpreadMethod::Pad,
        "reflect" => tree::SpreadMethod::Reflect,
        "repeat" => tree::SpreadMethod::Repeat,
        _ => tree::SpreadMethod::Pad,
    }
}

pub fn convert_units(
    node: &svgdom::Node,
    aid: AId,
    def: tree::Units,
) -> tree::Units {
    let node = resolve_attr(node, aid);
    let attrs = node.attributes();
    match attrs.get_str(aid) {
        Some("userSpaceOnUse") => tree::Units::UserSpaceOnUse,
        Some("objectBoundingBox") => tree::Units::ObjectBoundingBox,
        _ => def,
    }
}

fn find_gradient_with_stops(node: &svgdom::Node) -> Option<svgdom::Node> {
    for link in node.href_iter() {
        if !link.is_gradient() {
            warn!(
                "Gradient '{}' cannot reference '{}' via 'xlink:href'.",
                node.id(), link.tag_id().unwrap()
            );
            return None;
        }

        if link.children().any(|n| n.is_tag_name(EId::Stop)) {
            return Some(link.clone());
        }
    }

    None
}

fn find_pattern_with_children(node: &svgdom::Node) -> Option<svgdom::Node> {
    for link in node.href_iter() {
        if !link.is_tag_name(EId::Pattern) {
            warn!(
                "Pattern '{}' cannot reference '{}' via 'xlink:href'.",
                node.id(), link.tag_id().unwrap()
            );
            return None;
        }

        if link.has_children() {
            return Some(link.clone());
        }
    }

    None
}

fn convert_stops(grad: &svgdom::Node) -> Vec<tree::Stop> {
    let mut stops = Vec::new();

    {
        let mut prev_offset = Length::zero();
        for stop in grad.children() {
            if !stop.is_tag_name(EId::Stop) {
                warn!("Invalid gradient child: '{:?}'.", stop.tag_id().unwrap());
                continue;
            }

            // `number` can be either a number or a percentage.
            let offset = stop
                .attributes()
                .get_length(AId::Offset)
                .unwrap_or(prev_offset);
            let offset = match offset.unit {
                Unit::None => offset.num,
                Unit::Percent => offset.num / 100.0,
                _ => prev_offset.num,
            };
            let offset = f64_bound(0.0, offset, 1.0);
            prev_offset = Length::new_number(offset);

            let color = stop
                .attributes()
                .get_color(AId::StopColor)
                .unwrap_or_else(svgdom::Color::black);

            stops.push(tree::Stop {
                offset: offset.into(),
                color,
                opacity: stop.convert_opacity(AId::StopOpacity),
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
                stops[i - 1].offset = f64_bound(0.0, new_offset, 1.0).into();
                stops[i - 0].offset = offset1.into();
            }

            i += 1;
        }
    }

    stops
}

pub fn resolve_number(
    node: &svgdom::Node, aid: AId, units: tree::Units, state: &State, def: Length
) -> f64 {
    resolve_attr(node, aid).convert_length(aid, units, state, def)
}

fn resolve_attr(
    node: &svgdom::Node,
    aid: AId,
) -> svgdom::Node {
    if node.has_attribute(aid) {
        return node.clone();
    }

    match node.tag_id().unwrap() {
        EId::LinearGradient => resolve_lg_attr(node.clone(), aid),
        EId::RadialGradient => resolve_rg_attr(node.clone(), aid),
        EId::Pattern => resolve_pattern_attr(node.clone(), aid),
        EId::Filter => resolve_filter_attr(node.clone(), aid),
        _ => node.clone(),
    }
}

fn resolve_lg_attr(
    node: svgdom::Node,
    aid: AId,
) -> svgdom::Node {
    for link in node.href_iter() {
        let eid = try_opt_or!(link.tag_id(), node.clone());
        match (aid, eid) {
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
                if link.has_attribute(aid) {
                    return link;
                }
            }
            _ => break,
        }
    }

    node
}

fn resolve_rg_attr(
    node: svgdom::Node,
    aid: AId,
) -> svgdom::Node {
    for link in node.href_iter() {
        let eid = try_opt_or!(link.tag_id(), node.clone());
        match (aid, eid) {
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
                if link.has_attribute(aid) {
                    return link;
                }
            }
            _ => break,
        }
    }

    node
}

fn resolve_pattern_attr(
    node: svgdom::Node,
    aid: AId,
) -> svgdom::Node {
    for link in node.href_iter() {
        let eid = try_opt_or!(link.tag_id(), node.clone());

        if eid != EId::Pattern {
            break;
        }

        if link.has_attribute(aid) {
            return link;
        }
    }

    node
}

fn resolve_filter_attr(
    node: svgdom::Node,
    aid: AId,
) -> svgdom::Node {
    for link in node.href_iter() {
        let eid = try_opt_or!(link.tag_id(), node.clone());

        if eid != EId::Filter {
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

fn stops_to_color(
    stops: &[tree::Stop],
) -> Option<ServerOrColor> {
    if stops.is_empty() {
        None
    } else {
        Some(ServerOrColor::Color {
            color: stops[0].color,
            opacity: stops[0].opacity,
        })
    }
}
