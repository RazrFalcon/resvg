// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom::{
    Length,
};

use super::prelude::*;


pub fn resolve_svg_size(svg: &mut Node) -> bool {
    // We doesn't converted units yet, so operate on Length.

    let def = Length::new(100.0, Unit::Percent);
    let width = svg.attributes().get_length(AId::Width).unwrap_or(def);
    let height = svg.attributes().get_length(AId::Height).unwrap_or(def);

    let view_box = svg.get_viewbox();

    if (width.unit == Unit::Percent || height.unit == Unit::Percent) && view_box.is_none() {
        // TODO: it this case we should detect the bounding box of all elements,
        //       which is currently impossible
        return false;
    }

    if let Some(vbox) = view_box {
        if width.unit == Unit::Percent {
            let num = vbox.width * (width.num / 100.0);
            svg.set_attribute((AId::Width, Length::new_number(num)));
        }

        if height.unit == Unit::Percent {
            let num = vbox.height * (height.num / 100.0);
            svg.set_attribute((AId::Height, Length::new_number(num)));
        }
    }

    true
}
