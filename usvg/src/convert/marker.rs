// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::f64;

// external
use svgdom;

// self
use tree;
use super::prelude::*;


pub fn convert(
    node: &svgdom::Node,
    tree: &mut tree::Tree,
) -> Option<tree::Node> {
    let ref attrs = node.attributes();

    let rect = convert_rect(attrs);
    if !rect.is_valid() {
        warn!("Marker '{}' has an invalid size. Skipped.", node.id());
        return None;
    }

    let view_box = node.get_viewbox().map(|vb|
        tree::ViewBox {
            rect: vb,
            aspect: super::convert_aspect(attrs),
        }
    );

    Some(tree.append_to_defs(tree::NodeKind::Marker(tree::Marker {
        id: node.id().clone(),
        units: convert_units(attrs),
        rect,
        view_box,
        orientation: convert_orientation(attrs),
        overflow: convert_overflow(attrs),
    })))
}

fn convert_rect(attrs: &svgdom::Attributes) -> Rect {
    (
        attrs.get_number_or(AId::RefX, 0.0),
        attrs.get_number_or(AId::RefY, 0.0),
        attrs.get_number_or(AId::MarkerWidth, 3.0),
        attrs.get_number_or(AId::MarkerHeight, 3.0),
    ).into()
}

fn convert_units(attrs: &svgdom::Attributes) -> tree::MarkerUnits {
    match attrs.get_str(AId::MarkerUnits) {
        Some("userSpaceOnUse") => tree::MarkerUnits::UserSpaceOnUse,
        _ => tree::MarkerUnits::StrokeWidth,
    }
}

fn convert_overflow(attrs: &svgdom::Attributes) -> tree::Overflow {
    match attrs.get_str(AId::Overflow) {
        Some("visible") => tree::Overflow::Visible,
        Some("hidden") => tree::Overflow::Hidden,
        Some("scroll") => tree::Overflow::Scroll,
        Some("auto") => tree::Overflow::Auto,
        _ => tree::Overflow::Hidden,
    }
}

fn convert_orientation(attrs: &svgdom::Attributes) -> tree::MarkerOrientation {
    match attrs.get_value(AId::Orient) {
        Some(AValue::Angle(angle)) => {
            let a = match angle.unit {
                svgdom::AngleUnit::Degrees => angle.num,
                svgdom::AngleUnit::Gradians => angle.num * 180.0 / 200.0,
                svgdom::AngleUnit::Radians => angle.num * 180.0 / f64::consts::PI,
            };

            tree::MarkerOrientation::Angle(a)
        }
        Some(AValue::String(s)) if s == "auto" => {
            tree::MarkerOrientation::Auto
        }
        _ => tree::MarkerOrientation::Angle(0.0),
    }
}
