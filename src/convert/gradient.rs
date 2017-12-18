// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::f64;

use svgdom;

use dom;
use math;

use short::{
    AId,
    EId,
};

use traits::{
    GetValue,
};


pub fn convert_linear(node: &svgdom::Node) -> Option<dom::RefElement> {
    let attrs = node.attributes();

    if let Some(stops) = convert_stops(node) {
        let elem = dom::RefElement {
            data: dom::RefType::LinearGradient(dom::LinearGradient {
                x1: attrs.get_number(AId::X1).unwrap_or(0.0),
                y1: attrs.get_number(AId::Y1).unwrap_or(0.0),
                x2: attrs.get_number(AId::X2).unwrap_or(1.0),
                y2: attrs.get_number(AId::Y2).unwrap_or(0.0),
                d: dom::BaseGradient {
                    units: convert_grad_units(&attrs),
                    transform: attrs.get_transform(AId::GradientTransform).unwrap_or_default(),
                    spread_method: convert_spread_method(&attrs),
                    stops,
                }
            }),
            id: node.id().clone(),
        };

        Some(elem)
    } else {
        None
    }
}

pub fn convert_radial(node: &svgdom::Node) -> Option<dom::RefElement> {
    let attrs = node.attributes();

    if let Some(stops) = convert_stops(node) {
        let elem = dom::RefElement {
            data: dom::RefType::RadialGradient(dom::RadialGradient {
                cx: attrs.get_number(AId::Cx).unwrap_or(0.5),
                cy: attrs.get_number(AId::Cy).unwrap_or(0.5),
                r:  attrs.get_number(AId::R).unwrap_or(0.5),
                fx: attrs.get_number(AId::Fx).unwrap_or(0.5),
                fy: attrs.get_number(AId::Fy).unwrap_or(0.5),
                d: dom::BaseGradient {
                    units: convert_grad_units(&attrs),
                    transform: attrs.get_transform(AId::GradientTransform).unwrap_or_default(),
                    spread_method: convert_spread_method(&attrs),
                    stops,
                }
            }),
            id: node.id().clone(),
        };

        Some(elem)
    } else {
        None
    }
}

fn convert_grad_units(attrs: &svgdom::Attributes) -> dom::GradientUnits {
    let av = attrs.get_predef(AId::GradientUnits).unwrap_or(svgdom::ValueId::UserSpaceOnUse);

    match av {
        svgdom::ValueId::UserSpaceOnUse => dom::GradientUnits::UserSpaceOnUse,
        svgdom::ValueId::ObjectBoundingBox => dom::GradientUnits::ObjectBoundingBox,
        _ => dom::GradientUnits::UserSpaceOnUse,
    }
}

fn convert_spread_method(attrs: &svgdom::Attributes) -> dom::SpreadMethod {
    let av = attrs.get_predef(AId::SpreadMethod).unwrap_or(svgdom::ValueId::Pad);

    match av {
        svgdom::ValueId::Pad => dom::SpreadMethod::Pad,
        svgdom::ValueId::Reflect => dom::SpreadMethod::Reflect,
        svgdom::ValueId::Repeat => dom::SpreadMethod::Repeat,
        _ => dom::SpreadMethod::Pad,
    }
}

fn convert_stops(node: &svgdom::Node) -> Option<Vec<dom::Stop>> {
    let mut stops: Vec<dom::Stop> = Vec::new();
    let mut prev_offset = 0.0;

    for s in node.children() {
        if !s.is_tag_name(EId::Stop) {
            debug!("Invalid gradient child: '{:?}'.", s.tag_id().unwrap());
            continue;
        }

        let attrs = s.attributes();

        let mut offset = attrs.get_number(AId::Offset).unwrap_or(0.0);
        offset = math::f64_bound(0.0, offset, 1.0);
        // Next offset must be smaller then previous.
        if offset < prev_offset {
            if let Some(ref mut prev) = stops.last_mut() {
                // Make previous offset a bit smaller.
                prev.offset = prev_offset - f64::EPSILON;
            }

            offset = prev_offset;
        }
        prev_offset = offset;

        // Tested by:
        // - pservers-grad-18-b.svg
        let color = attrs.get_color(AId::StopColor).unwrap_or(svgdom::types::Color::new(0, 0, 0));
        let opacity = attrs.get_number(AId::StopOpacity).unwrap_or(1.0);

        stops.push(dom::Stop {
            offset,
            color,
            opacity,
        });
    }

    if stops.len() < 2 {
        warn!("Gradient '{}' contains less than 2 stop children. Skipped.", node.id());
        return None;
    }

    Some(stops)
}
