// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom::{
    AspectRatio,
};

use super::prelude::*;


/// Resolves `linearGradient` attributes.
pub fn resolve_linear_gradient_attributes(doc: &Document) {
    for node in &mut gen_order(doc, EId::LinearGradient) {
        resolve_attr(node, AId::GradientUnits, Some(AValue::from("objectBoundingBox")));
        resolve_attr(node, AId::SpreadMethod, Some(AValue::from("pad")));
        resolve_attr(node, AId::X1, Some(AValue::from(0.0)));
        resolve_attr(node, AId::Y1, Some(AValue::from(0.0)));
        resolve_attr(node, AId::X2, Some(AValue::from(1.0)));
        resolve_attr(node, AId::Y2, Some(AValue::from(0.0)));
        resolve_attr(node, AId::GradientTransform, None);
    }
}

/// Resolves `radialGradient` attributes.
pub fn resolve_radial_gradient_attributes(doc: &Document) {
    for node in &mut gen_order(doc, EId::RadialGradient) {
        resolve_attr(node, AId::GradientUnits, Some(AValue::from("objectBoundingBox")));
        resolve_attr(node, AId::SpreadMethod, Some(AValue::from("pad")));
        resolve_attr(node, AId::Cx, Some(AValue::from(0.5)));
        resolve_attr(node, AId::Cy, Some(AValue::from(0.5)));
        resolve_attr(node, AId::R, Some(AValue::from(0.5)));
        resolve_attr(node, AId::GradientTransform, None);

        // Replace negative `r` with zero
        // otherwise `fx` and `fy` may became NaN.
        //
        // Note: negative `r` is an error and UB, so we can resolve it
        // in whatever way we want.
        // By replacing it with zero we will get the same behavior as Chrome.
        let r = node.attributes().get_number(AId::R).unwrap();
        if r < 0.0 {
            node.set_attribute((AId::R, 0.0));
        }


        // If `fx` is not found then set it to `cx`.
        let cx = node.attributes().get_value(AId::Cx).cloned();
        resolve_attr(node, AId::Fx, cx);

        // If `fy` is not found then set it to `cy`.
        let cy = node.attributes().get_value(AId::Cy).cloned();
        resolve_attr(node, AId::Fy, cy);

        prepare_focal(node);
    }
}

/// Resolves `pattern` attributes.
pub fn resolve_pattern_attributes(doc: &Document) {
    for node in &mut gen_order(doc, EId::Pattern) {
        resolve_attr(node, AId::PatternUnits, Some(AValue::from("objectBoundingBox")));
        resolve_attr(node, AId::PatternContentUnits, Some(AValue::from("userSpaceOnUse")));
        resolve_attr(node, AId::PatternTransform, None);
        resolve_attr(node, AId::X, Some(AValue::from(0.0)));
        resolve_attr(node, AId::Y, Some(AValue::from(0.0)));
        resolve_attr(node, AId::Width, Some(AValue::from(0.0)));
        resolve_attr(node, AId::Height, Some(AValue::from(0.0)));
        resolve_attr(node, AId::PreserveAspectRatio, Some(AValue::from(AspectRatio::default())));
        resolve_attr(node, AId::ViewBox, None);
    }
}

/// Resolves `filter` attributes.
pub fn resolve_filter_attributes(doc: &Document) {
    for node in &mut gen_order(doc, EId::Filter) {
        resolve_attr(node, AId::FilterUnits, Some(AValue::from("objectBoundingBox")));
        resolve_attr(node, AId::PrimitiveUnits, Some(AValue::from("userSpaceOnUse")));
        resolve_attr(node, AId::X, Some(AValue::from(-0.1)));
        resolve_attr(node, AId::Y, Some(AValue::from(-0.1)));
        resolve_attr(node, AId::Width, Some(AValue::from(1.2)));
        resolve_attr(node, AId::Height, Some(AValue::from(1.2)));

        // TODO: do this for other elements
        let is_valid_filter_units = {
            let attrs = node.attributes();
            let filter_units = attrs.get_str_or(AId::FilterUnits, "");
            filter_units == "objectBoundingBox" || filter_units == "userSpaceOnUse"
        };

        if !is_valid_filter_units {
            node.set_attribute((AId::FilterUnits, "objectBoundingBox"));
        }
    }
}

/// Generates a list of elements from less used to most used.
///
/// If an element has an `xlink:href` attribute then it can inherit specific
/// attributes from the linked element.
/// But if it also has an `xlink:href` attribute than we have to follow it too.
/// And on and on. Until the element with a required attribute.
///
/// Let's say we have an SVG like this:
/// ```text
/// <linearGradient id="lg1" x1="5" y2="20"/>
/// <linearGradient id="lg2" xlink:href="#lg1" x2="10"/>
/// <linearGradient id="lg3" xlink:href="#lg2" y1="15"/>
/// ```
///
/// It should be resolved to:
/// ```text
/// <linearGradient id="lg1" x1="5" y2="20"/>
/// <linearGradient id="lg2" x1="5" x2="10" y2="20"/>
/// <linearGradient id="lg3" x1="5" x2="10" y1="15" y2="20"/>
/// ```
///
/// But in the `radialGradient` case, the `fx` and `fy` attributes
/// should fallback to `cx` and `cy` attributes and only after
/// the `fx` and `fy` attributes are resolved.
///
/// So an SVG like this:
/// ```text
/// <radialGradient id="rg2" xlink:href="#rg1" cx="10"/>
/// <radialGradient id="rg3" xlink:href="#rg2" fy="15"/>
/// <radialGradient id="rg1" fx="5"/>
/// ```
///
/// Should be resolved in order: `rg1` -> `rg2` -> `rg3`.
/// And will produce:
/// ```text
/// <radialGradient id="rg2" fx="5" cx="10"/>
/// <radialGradient id="rg3" cx="10" fx="5" fy="15"/>
/// <radialGradient id="rg1" fx="5"/>
/// ```
///
/// And not in `rg2` -> `rg3` -> `rg1` order.
/// Because it will produce:
/// ```text
/// <radialGradient id="rg2" fx="10" fy="10" cx="10"/>
/// <radialGradient id="rg3" fx="10" fy="15" cx="10"/> <!-- fx is 10, not 5 -->
/// <radialGradient id="rg1" fx="5"/>
/// ```
fn gen_order(doc: &Document, eid: EId) -> Vec<Node> {
    let nodes = doc.root().descendants().filter(|n| n.is_tag_name(eid))
                   .collect::<Vec<Node>>();

    let mut order = Vec::with_capacity(nodes.len());

    while order.len() != nodes.len() {
        for node in &nodes {
            if order.iter().any(|n| n == node) {
                continue;
            }

            let c = node.linked_nodes().iter().filter(|n| {
                n.is_tag_name(eid) && !order.iter().any(|on| on == *n)
            }).count();

            if c == 0 {
                order.push(node.clone());
            }
        }
    }

    order
}

fn resolve_attr(node: &mut Node, id: AId, def_value: Option<AValue>) {
    if node.has_attribute(id) {
        return;
    }

    let v = match node.tag_id().unwrap() {
        EId::LinearGradient => resolve_lg_attr(node, id, def_value),
        EId::RadialGradient => resolve_rg_attr(node, id, def_value),
        EId::Pattern => resolve_patt_attr(node, id, def_value),
        EId::Filter => resolve_filter_attr(node, id, def_value),
        _ => None,
    };

    if let Some(v) = v {
        node.set_attribute((id, v));
    }
}

fn resolve_lg_attr(
    node: &Node,
    aid: AId,
    def_value: Option<AValue>,
) -> Option<AValue> {
    if node.has_attribute(aid) {
        return node.attributes().get_value(aid).cloned();
    }

    // Check for referenced element first.
    let link = match node.attributes().get_value(AId::Href) {
        Some(&AValue::Link(ref link)) => link.clone(),
        _ => {
            // Use current element.
            return match node.attributes().get_value(aid) {
                Some(v) => Some(v.clone()),
                None => def_value,
            };
        }
    };

    // If `link` is not an SVG element - return `def_value`.
    let eid = match link.tag_id() {
        Some(eid) => eid,
        None => return def_value,
    };

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
            resolve_lg_attr(&link, aid, def_value)
        }
        _ => def_value
    }
}

fn resolve_rg_attr(
    node: &Node,
    aid: AId,
    def_value: Option<AValue>,
) -> Option<AValue> {
    if node.has_attribute(aid) {
        return node.attributes().get_value(aid).cloned();
    }

    // Check for referenced element first.
    let link = match node.attributes().get_value(AId::Href) {
        Some(&AValue::Link(ref link)) => link.clone(),
        _ => {
            // Use current element.
            return match node.attributes().get_value(aid) {
                Some(v) => Some(v.clone()),
                None => def_value,
            };
        }
    };

    // If `link` is not an SVG element - return `def_value`.
    let eid = match link.tag_id() {
        Some(eid) => eid,
        None => return def_value,
    };

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
            resolve_rg_attr(&link, aid, def_value)
        }
        _ => def_value
    }
}

fn resolve_patt_attr(
    node: &Node,
    aid: AId,
    def_value: Option<AValue>,
) -> Option<AValue> {
    if node.has_attribute(aid) {
        return node.attributes().get_value(aid).cloned();
    }

    // Check for referenced element first.
    let link = match node.attributes().get_value(AId::Href) {
        Some(&AValue::Link(ref link)) => link.clone(),
        _ => {
            // Use current element.
            return match node.attributes().get_value(aid) {
                Some(v) => Some(v.clone()),
                None => def_value,
            };
        }
    };

    // If `link` is not an SVG element - return `def_value`.
    match link.tag_id() {
        Some(EId::Pattern) => resolve_patt_attr(&link, aid, def_value),
        _ => def_value,
    }
}

fn resolve_filter_attr(
    node: &Node,
    aid: AId,
    def_value: Option<AValue>,
) -> Option<AValue> {
    if node.has_attribute(aid) {
        return node.attributes().get_value(aid).cloned();
    }

    // Check for referenced element first.
    let link = match node.attributes().get_value(AId::Href) {
        Some(&AValue::Link(ref link)) => link.clone(),
        _ => {
            // Use current element.
            return match node.attributes().get_value(aid) {
                Some(v) => Some(v.clone()),
                None => def_value,
            };
        }
    };

    // If `link` is not an SVG element - return `def_value`.
    match link.tag_id() {
        Some(EId::Filter) => resolve_filter_attr(&link, aid, def_value),
        _ => def_value,
    }
}

/// Prepares the radial gradient focal radius.
///
/// According to the SVG spec:
///
/// If the point defined by `fx` and `fy` lies outside the circle defined by
/// `cx`, `cy` and `r`, then the user agent shall set the focal point to the
/// intersection of the line from (`cx`, `cy`) to (`fx`, `fy`) with the circle
/// defined by `cx`, `cy` and `r`.
fn prepare_focal(node: &mut Node) {
    let (new_fx, new_fy) = {
        let attrs = node.attributes();

        // Unwrap is safe, because we just resolved all this attributes.
        let cx = attrs.get_number(AId::Cx).unwrap();
        let cy = attrs.get_number(AId::Cy).unwrap();
        let r  = attrs.get_number(AId::R).unwrap();
        let fx = attrs.get_number(AId::Fx).unwrap();
        let fy = attrs.get_number(AId::Fy).unwrap();

        let max_r = r - r * 0.001;

        let mut line = Line::new(cx, cy, fx, fy);

        if line.length() > max_r {
            line.set_length(max_r);
        }

        (line.x2, line.y2)
    };

    node.set_attribute((AId::Fx, new_fx));
    node.set_attribute((AId::Fy, new_fy));
}
