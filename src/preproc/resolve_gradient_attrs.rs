// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom::{
    Document,
    Node,
    ValueId,
};

// self
use short::{
    AId,
    AValue,
    EId,
};
use geom::{
    Line,
};
use traits::{
    GetValue,
};

// TODO: rename mod

/// Resolve attributes of `linearGradient` elements.
///
/// According to the SVG spec, `linearGradient` attributes can be
/// inherited via `xlink:href` attribute.
/// So we have to search linked gradients first and if they do not have such attributes
/// we have to fallback to the default one.
///
/// This method will process all `linearGradient` elements in the `Document`.
///
/// Resolvable attributes: `x1`, `y1`, `x2`, `y2`, `gradientUnits`,
/// `gradientTransform`, `spreadMethod`.
///
/// Details: https://www.w3.org/TR/SVG/pservers.html#LinearGradients
pub fn resolve_linear_gradient_attributes(doc: &Document) {
    for node in &mut gen_order(doc, EId::LinearGradient) {
        check_attr(node, AId::GradientUnits,
            Some(AValue::from(ValueId::ObjectBoundingBox)));
        check_attr(node, AId::SpreadMethod, Some(AValue::from(ValueId::Pad)));
        check_attr(node, AId::X1, Some(AValue::from(0.0)));
        check_attr(node, AId::Y1, Some(AValue::from(0.0)));
        check_attr(node, AId::X2, Some(AValue::from(1.0)));
        check_attr(node, AId::Y2, Some(AValue::from(0.0)));
        check_attr(node, AId::GradientTransform, None);
    }
}

/// Resolve attributes of `radialGradient` elements.
///
/// According to the SVG spec, `radialGradient` attributes can be
/// inherited via `xlink:href` attribute.
/// So we have to search linked gradients first and if they do not have such attributes
/// we have to fallback to the default one.
///
/// This method will process all `radialGradient` elements in the `Document`.
///
/// Resolvable attributes: `cx`, `cy`, `fx`, `fy`, `r`, `gradientUnits`,
/// `gradientTransform`, `spreadMethod`.
///
/// Details: https://www.w3.org/TR/SVG/pservers.html#RadialGradients
pub fn resolve_radial_gradient_attributes(doc: &Document) {
    // We trying to find 'fx', 'fy' in referenced nodes first,
    // and if they not found - we get it from current 'cx', 'cy'.
    // But if we resolve referenced node first, it will have own
    // 'fx', 'fy' which we will inherit, instead of nodes own.
    // Which will lead to rendering error.
    // So we need to resolve nodes in referencing order.
    // From not referenced to referenced.

    for node in &mut gen_order(doc, EId::RadialGradient) {
        check_attr(node, AId::GradientUnits,
            Some(AValue::from(ValueId::ObjectBoundingBox)));
        check_attr(node, AId::SpreadMethod, Some(AValue::from(ValueId::Pad)));
        check_attr(node, AId::Cx, Some(AValue::from(0.5)));
        check_attr(node, AId::Cy, Some(AValue::from(0.5)));
        check_attr(node, AId::R,  Some(AValue::from(0.5)));

        // Replace negative `r` with zero
        // otherwise `fx` and `fy` may became NaN.
        //
        // Note: negative `r` is an error and UB, so we can resolve it
        // in whatever way we want.
        // By replacing it with zero we will get the same behavior as on Chrome.
        let r = node.attributes().get_number(AId::R).unwrap();
        if r < 0.0 {
            node.set_attribute((AId::R, 0.0));
        }

        let cx = node.attributes().get_value(AId::Cx).cloned();
        let cy = node.attributes().get_value(AId::Cy).cloned();
        check_attr(node, AId::Fx, cx);
        check_attr(node, AId::Fy, cy);
        prepare_focal(node);

        check_attr(node, AId::GradientTransform, None);
    }
}

/// Resolve attributes of `pattern` elements.
pub fn resolve_pattern_attributes(doc: &Document) {
    for node in &mut gen_order(doc, EId::Pattern) {
        check_attr(node, AId::PatternUnits,
                   Some(AValue::from(ValueId::ObjectBoundingBox)));
        check_attr(node, AId::PatternContentUnits,
                   Some(AValue::from(ValueId::UserSpaceOnUse)));
        check_attr(node, AId::PatternTransform, None);
        check_attr(node, AId::X, Some(AValue::from(0.0)));
        check_attr(node, AId::Y, Some(AValue::from(0.0)));
        check_attr(node, AId::Width, Some(AValue::from(0.0)));
        check_attr(node, AId::Height, Some(AValue::from(0.0)));
        check_attr(node, AId::PreserveAspectRatio, Some(AValue::from("xMidYMid meet")));
        check_attr(node, AId::ViewBox, None);
    }
}

/// Generates a list of elements from less used to most used.
fn gen_order(doc: &Document, eid: EId) -> Vec<Node> {
    let nodes = doc.descendants().filter(|n| n.is_tag_name(eid))
                   .collect::<Vec<Node>>();

    let mut order = Vec::with_capacity(nodes.len());

    while order.len() != nodes.len() {
        for node in &nodes {
            if order.iter().any(|n| n == node) {
                continue;
            }

            let c = node.linked_nodes().filter(|n| {
                n.is_tag_name(eid) && !order.iter().any(|on| on == n)
            }).count();

            if c == 0 {
                order.push(node.clone());
            }
        }
    }

    order
}

fn check_attr(node: &mut Node, id: AId, def_value: Option<AValue>) {
    if !node.has_attribute(id) {
        let eid = node.tag_id().unwrap();

        let v = match eid {
            EId::LinearGradient => resolve_lg_attr(node, id, def_value),
            EId::RadialGradient => resolve_rg_attr(node, id, def_value),
            EId::Pattern => resolve_patt_attr(node, id, def_value),
            _ => None,
        };

        if let Some(v) = v {
            node.set_attribute((id, v));
        }
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
    let link = match node.attributes().get_value(("xlink", AId::Href)) {
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
    let link = match node.attributes().get_value(("xlink", AId::Href)) {
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
    let link = match node.attributes().get_value(("xlink", AId::Href)) {
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
        _ => return def_value,
    }
}

// According to the SVG spec:
// If the point defined by 'fx' and 'fy' lies outside the circle defined by
// 'cx', 'cy' and 'r', then the user agent shall set the focal point to the
// intersection of the line from ('cx', 'cy') to ('fx', 'fy') with the circle
// defined by 'cx', 'cy' and 'r'.
fn prepare_focal(node: &mut Node) {
    let mut attrs = node.attributes_mut();

    // Unwrap is safe, because we just resolved all this attributes.
    let cx = attrs.get_number(AId::Cx).unwrap();
    let cy = attrs.get_number(AId::Cy).unwrap();
    let r = attrs.get_number(AId::R).unwrap();
    let fx = attrs.get_number(AId::Fx).unwrap();
    let fy = attrs.get_number(AId::Fy).unwrap();

    let (new_fx, new_fy) = _prepare_focal(cx, cy, r, fx, fy);
    attrs.insert_from(AId::Fx, new_fx);
    attrs.insert_from(AId::Fy, new_fy);
}

fn _prepare_focal(cx: f64, cy: f64, r: f64, fx: f64, fy: f64) -> (f64, f64) {
    let max_r = r - r * 0.001;

    let mut line = Line::new(cx, cy, fx, fy);

    if line.length() > max_r {
        line.set_length(max_r);
    }

    (line.x2, line.y2)
}
