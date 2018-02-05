// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom::{
    self,
    ElementType
};

// self
use tree;
use short::{
    AId,
    AValue,
    EId,
};
use traits::{
    GetValue,
};


pub fn convert(
    doc: &tree::RenderTree,
    attrs: &svgdom::Attributes,
) -> Option<tree::Fill> {
    let paint = if let Some(fill) = attrs.get_type(AId::Fill) {
        match *fill {
            AValue::Color(c) => {
                tree::Paint::Color(c)
            }
            AValue::FuncLink(ref link) => {
                let mut p = None;
                if link.is_gradient() || link.is_tag_name(EId::Pattern) {
                    if let Some(idx) = doc.defs_index(&link.id()) {
                        p = Some(tree::Paint::Link(idx));
                    }
                }

                match p {
                    Some(p) => p,
                    None => {
                        warn!("Filling with {:?} is not supported.",
                              link.tag_id().unwrap());
                        return None;
                    }
                }
            }
            AValue::PredefValue(svgdom::ValueId::None) => {
                return None;
            }
            _ => {
                warn!("An invalid fill value: {}. Skipped.", fill);
                return None;
            }
        }
    } else {
        return None;
    };

    let fill_opacity = attrs.get_number(AId::FillOpacity).unwrap_or(1.0);

    let fill_rule = attrs.get_predef(AId::FillRule)
                         .unwrap_or(svgdom::ValueId::Nonzero);
    let fill_rule = match fill_rule {
        svgdom::ValueId::Evenodd => tree::FillRule::EvenOdd,
        _ => tree::FillRule::NonZero,
    };

    let fill = tree::Fill {
        paint,
        opacity: fill_opacity,
        rule: fill_rule,
    };

    Some(fill)
}
