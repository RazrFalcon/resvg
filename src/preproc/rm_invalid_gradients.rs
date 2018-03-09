// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom::{
    Color,
    Document,
    ElementType,
    FuzzyEq,
    Node,
    ValueId,
};

// self
use short::{
    AId,
    AValue,
    EId,
};
use traits::{
    GetValue,
};


pub fn remove_invalid_gradients(doc: &mut Document) {
    let mut ids = Vec::new();
    let mut nodes = Vec::new();

    for gradient in doc.descendants().filter(|n| n.is_gradient()) {
        let count = gradient.children().count();

        if count == 0 || count == 1 {
            for mut linked in gradient.linked_nodes().collect::<Vec<Node>>() {
                collect_ids(&linked, &gradient, &mut ids);

                for id in &ids {
                    if count == 0 {
                        linked.set_attribute((*id, ValueId::None));
                    } else {
                        // We know that gradient has first child.
                        let stop = gradient.first_child().unwrap();
                        let color = stop.attributes().get_color(AId::StopColor)
                                        .unwrap_or(Color::new(0, 0, 0));

                        linked.set_attribute((*id, color));
                    }
                }
            }

            nodes.push(gradient);
        } else if gradient.is_tag_name(EId::RadialGradient) {
            if process_negative_r(&gradient, &mut ids) {
                nodes.push(gradient);
            }
        }
    }

    for mut node in nodes {
        node.remove();
    }
}

// 'A value of zero will cause the area to be painted as a single color
// using the color and opacity of the last gradient stop.'
//
// https://www.w3.org/TR/SVG11/pservers.html#RadialGradientElementRAttribute
fn process_negative_r(
    gradient: &Node,
    ids: &mut Vec<AId>,
) -> bool {
    let r = gradient.attributes().get_number(AId::R);
    let r = match r {
        Some(r) => r,
        None => {
            warn!("'r' attribute in 'radialGradient' should be already resolved.");
            return false;
        }
    };

    if !r.is_fuzzy_zero() {
        return false;
    }

    let stop = match gradient.last_child() {
        Some(s) => s,
        None => return false,
    };

    for mut linked in gradient.linked_nodes().collect::<Vec<Node>>() {
        collect_ids(&linked, &gradient, ids);

        for id in ids.iter() {
            let color = stop.attributes().get_color(AId::StopColor)
                            .unwrap_or(Color::new(0, 0, 0));
            let opacity = stop.attributes().get_number(AId::StopOpacity)
                              .unwrap_or(1.0);

            // If `stop` has `stop-opacity` than we should apply it too,
            // but not as `opacity`, but as `fill-opacity` and `stroke-opacity`
            // respectively.
            if opacity.fuzzy_ne(&1.0) {
                match *id {
                    AId::Fill => {
                        update_opacity(&mut linked, AId::FillOpacity, opacity);
                    }
                    AId::Stroke => {
                        update_opacity(&mut linked, AId::StrokeOpacity, opacity);
                    }
                    _ => {
                        // unreachable
                    }
                }
            }

            linked.set_attribute((*id, color));
        }
    }

    true
}

fn collect_ids(linked: &Node, gradient: &Node, ids: &mut Vec<AId>) {
    ids.clear();

    for (aid, attr) in linked.attributes().iter_svg() {
        match attr.value {
              AValue::Link(ref link)
            | AValue::FuncLink(ref link) => {
                if link == gradient {
                    ids.push(aid);
                }
            }
            _ => {}
        }
    }
}

fn update_opacity(node: &mut Node, aid: AId, new_opacity: f64) {
    let old_opacity = node.attributes().get_number(aid).unwrap_or(1.0);
    node.set_attribute((aid, old_opacity * new_opacity));
}
