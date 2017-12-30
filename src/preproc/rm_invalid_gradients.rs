// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom::{
    Color,
    Document,
    Node,
    ValueId,
    ElementType,
};

use short::{
    AId,
    AValue,
};

use traits::{
    GetValue,
};


// Tested by:
// - pservers-grad-16-b.svg
// - pservers-grad-stops-01-f.svg
pub fn remove_invalid_gradients(doc: &mut Document) {
    let mut ids = Vec::new();
    let mut nodes = Vec::new();

    for gradient in doc.descendants().filter(|n| n.is_gradient()) {
        let count = gradient.children().count();

        if count == 0 || count == 1 {
            for mut linked in gradient.linked_nodes().collect::<Vec<Node>>() {
                ids.clear();

                for (aid, attr) in linked.attributes().iter_svg() {
                    match attr.value {
                          AValue::Link(ref link)
                        | AValue::FuncLink(ref link) => {
                            if link == &gradient {
                                ids.push(aid);
                            }
                        }
                        _ => {}
                    }
                }

                for id in &ids {
                    if count == 0 {
                        linked.set_attribute((*id, ValueId::None));
                    } else {
                        let stop = gradient.first_child().unwrap();
                        let color = stop.attributes().get_color(AId::StopColor)
                                        .unwrap_or(Color::new(0, 0, 0));

                        linked.set_attribute((*id, color));
                    }
                }
            }

            nodes.push(gradient);
        }
    }

    for mut node in nodes {
        node.remove();
    }
}
