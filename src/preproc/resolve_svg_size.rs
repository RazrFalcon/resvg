// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom::{
    Node,
    Length,
};

// self
use short::{
    AId,
    Unit,
};
use traits::{
    GetValue,
    GetViewBox,
};


/// Tested by:
/// - struct-svg-*.svg
pub fn resolve_svg_size(svg: &mut Node) -> bool {
    // We doesn't converted units yet, so operate on Length.

    let width = get_length(&svg, AId::Width);
    let height = get_length(&svg, AId::Height);

    let view_box = svg.get_viewbox().ok();

    if (width.unit == Unit::Percent || height.unit == Unit::Percent) && view_box.is_none() {
        // TODO: it this case we should detect the bounding box of all elements,
        //       which is currently impossible
        return false;
    }

    if let Some(vbox) = view_box {
        if width.unit == Unit::Percent {
            let num = vbox.w * (width.num / 100.0);
            svg.set_attribute((AId::Width, Length::new_number(num)));
        }

        if height.unit == Unit::Percent {
            let num = vbox.h * (height.num / 100.0);
            svg.set_attribute((AId::Height, Length::new_number(num)));
        }
    }

    true
}

fn get_length(node: &Node, aid: AId) -> Length {
    node.attributes().get_length(aid).unwrap_or(Length::new(100.0, Unit::Percent))
}
