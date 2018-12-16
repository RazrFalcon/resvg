// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom;

// self
use tree;
use super::prelude::*;


pub fn convert_linear(
    node: &svgdom::Node,
    tree: &mut tree::Tree,
) {
    let ref attrs = node.attributes();
    let transform = attrs.get_transform(AId::GradientTransform).unwrap_or_default();
    let stops = try_opt!(convert_stops(node), ());

    tree.append_to_defs(
        tree::NodeKind::LinearGradient(tree::LinearGradient {
            id: node.id().clone(),
            x1: attrs.get_number_or(AId::X1, 0.0),
            y1: attrs.get_number_or(AId::Y1, 0.0),
            x2: attrs.get_number_or(AId::X2, 1.0),
            y2: attrs.get_number_or(AId::Y2, 0.0),
            base: tree::BaseGradient {
                units: super::convert_element_units(attrs, AId::GradientUnits),
                transform,
                spread_method: convert_spread_method(&attrs),
                stops,
            }
        })
    );
}

pub fn convert_radial(
    node: &svgdom::Node,
    tree: &mut tree::Tree,
) {
    let ref attrs = node.attributes();
    let transform = attrs.get_transform(AId::GradientTransform).unwrap_or_default();
    let stops = try_opt!(convert_stops(node), ());

    tree.append_to_defs(
        tree::NodeKind::RadialGradient(tree::RadialGradient {
            id: node.id().clone(),
            cx: attrs.get_number_or(AId::Cx, 0.5),
            cy: attrs.get_number_or(AId::Cy, 0.5),
            r:  attrs.get_number_or(AId::R,  0.5).into(),
            fx: attrs.get_number_or(AId::Fx, 0.5),
            fy: attrs.get_number_or(AId::Fy, 0.5),
            base: tree::BaseGradient {
                units: super::convert_element_units(attrs, AId::GradientUnits),
                transform,
                spread_method: convert_spread_method(&attrs),
                stops,
            }
        })
    );
}

fn convert_spread_method(attrs: &svgdom::Attributes) -> tree::SpreadMethod {
    let av = attrs.get_str_or(AId::SpreadMethod, "pad");

    match av {
        "pad" => tree::SpreadMethod::Pad,
        "reflect" => tree::SpreadMethod::Reflect,
        "repeat" => tree::SpreadMethod::Repeat,
        _ => tree::SpreadMethod::Pad,
    }
}

fn convert_stops(node: &svgdom::Node) -> Option<Vec<tree::Stop>> {
    let mut stops = Vec::new();

    for s in node.children() {
        if !s.is_tag_name(EId::Stop) {
            warn!("Invalid gradient child: '{:?}'.", s.tag_id().unwrap());
            continue;
        }

        let attrs = s.attributes();

        // Do not use `f64_bound` here because `offset` must be already resolved.
        let offset = attrs.get_number_or(AId::Offset, 0.0).into();
        let color = attrs.get_color(AId::StopColor).unwrap_or(svgdom::Color::black());
        let opacity = f64_bound(0.0, attrs.get_number_or(AId::StopOpacity, 1.0), 1.0).into();

        stops.push(tree::Stop {
            offset,
            color,
            opacity,
        });
    }

    debug_assert!(stops.len() >= 2, "gradient must have at least 2 children");

    if stops.len() >= 2 {
        Some(stops)
    } else {
        None
    }
}
