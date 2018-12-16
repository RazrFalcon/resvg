// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom::{
    Color,
    PaintFallback,
};

use super::prelude::*;


pub fn remove_invalid_gradients(doc: &mut Document) {
    let mut ids = Vec::new();
    let mut nodes = Vec::new();

    for gradient in doc.root().descendants().filter(|n| n.is_gradient()) {
        let count = gradient.children().count();

        if count == 0 || count == 1 {
            let linked_nodes = gradient.linked_nodes().clone();
            for mut linked in linked_nodes {
                collect_ids(&linked, &gradient, &mut ids);

                for aid in &ids {
                    if count == 0 {
                        let av = linked.attributes().get_value(*aid).cloned();
                        let av = if let Some(AValue::Paint(_, fallback)) = av {
                            match fallback {
                                Some(PaintFallback::None) => {
                                    AValue::None
                                }
                                Some(PaintFallback::CurrentColor) => {
                                    debug_panic!("'currentColor' must be already resolved.");
                                    AValue::None
                                }
                                Some(PaintFallback::Color(c)) => {
                                    AValue::Color(c)
                                }
                                None => {
                                    AValue::None
                                }
                            }
                        } else {
                            AValue::None
                        };

                        linked.set_attribute((*aid, av));
                    } else {
                        // We know that gradient has first child.
                        let stop = gradient.first_child().unwrap();
                        let color = stop.attributes().get_color(AId::StopColor)
                                        .unwrap_or(Color::black());
                        let opacity = stop.attributes().get_number_or(AId::StopOpacity, 1.0);

                        prepare_link_opacity(&mut linked, *aid, opacity);
                        linked.set_attribute((*aid, color));
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

    for node in nodes {
        doc.remove_node(node);
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
    let r = try_opt_warn!(gradient.attributes().get_number(AId::R), false,
                "'r' attribute in 'radialGradient' should be already resolved.");

    if !r.is_fuzzy_zero() {
        return false;
    }

    let stop = match gradient.last_child() {
        Some(s) => s,
        None => return false,
    };

    let linked_nodes = gradient.linked_nodes().clone();
    for mut linked in linked_nodes {
        collect_ids(&linked, gradient, ids);

        for id in ids.iter() {
            let color = stop.attributes().get_color(AId::StopColor)
                            .unwrap_or(Color::black());
            let opacity = stop.attributes().get_number_or(AId::StopOpacity, 1.0);

            prepare_link_opacity(&mut linked, *id, opacity);
            linked.set_attribute((*id, color));
        }
    }

    true
}

fn collect_ids(linked: &Node, gradient: &Node, ids: &mut Vec<AId>) {
    ids.clear();

    for (aid, attr) in linked.attributes().iter().svg() {
        match attr.value {
            AValue::Paint(ref link, _) => {
                if link == gradient {
                    ids.push(aid);
                }
            }
            _ => {}
        }
    }
}

fn prepare_link_opacity(linked: &mut Node, aid: AId, opacity: f64) {
    // If `stop` has `stop-opacity` than we should apply it too,
    // but not as `opacity`, but as `fill-opacity` and `stroke-opacity`
    // respectively.
    if opacity.fuzzy_ne(&1.0) {
        match aid {
            AId::Fill => {
                update_opacity(linked, AId::FillOpacity, opacity);
            }
            AId::Stroke => {
                update_opacity(linked, AId::StrokeOpacity, opacity);
            }
            _ => {
                // unreachable
            }
        }
    }
}

fn update_opacity(node: &mut Node, aid: AId, new_opacity: f64) {
    let old_opacity = node.attributes().get_number_or(aid, 1.0);
    node.set_attribute((aid, old_opacity * new_opacity));
}
