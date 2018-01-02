// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom::{
    self,
    ElementType
};

use dom;

use short::{
    AId,
    AValue,
    EId,
};

use traits::{
    GetValue,
};


pub fn convert(
    defs: &[dom::RefElement],
    attrs: &svgdom::Attributes,
) -> Option<dom::Fill>
{
    let paint = if let Some(fill) = attrs.get_type(AId::Fill) {
        match *fill {
            AValue::Color(c) => {
                dom::Paint::Color(c)
            }
            AValue::FuncLink(ref link) => {
                let mut p = None;
                if link.is_gradient() || link.is_tag_name(EId::Pattern) {
                    if let Some(idx) = defs.iter().position(|e| e.id == *link.id()) {
                        p = Some(dom::Paint::Link(idx));
                    }
                }

                match p {
                    Some(p) => p,
                    None => {
                        warn!("Filling with {:?} is not supported.", link.tag_id().unwrap());
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

    let fill_rule = match attrs.get_predef(AId::FillRule).unwrap_or(svgdom::ValueId::Nonzero) {
        svgdom::ValueId::Evenodd => dom::FillRule::EvenOdd,
        _ => dom::FillRule::NonZero,
    };

    let fill = dom::Fill {
        paint: paint,
        opacity: fill_opacity,
        rule: fill_rule,
    };

    Some(fill)
}
