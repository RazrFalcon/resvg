// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom;

// self
use tree;
use super::prelude::*;


pub fn convert(
    tree: &tree::Tree,
    attrs: &svgdom::Attributes,
    has_bbox: bool,
) -> Option<tree::Stroke> {
    let dashoffset  = attrs.get_number_or(AId::StrokeDashoffset, 0.0) as f32;
    let miterlimit  = attrs.get_number_or(AId::StrokeMiterlimit, 4.0);
    let opacity     = attrs.get_number_or(AId::StrokeOpacity, 1.0).into();
    let width       = attrs.get_number_or(AId::StrokeWidth, 1.0);

    if !(width > 0.0) {
        return None;
    }

    let width = tree::StrokeWidth::new(width);

    // Must be bigger than 1.
    let miterlimit = if miterlimit < 1.0 { 1.0 } else { miterlimit };
    let miterlimit = tree::StrokeMiterlimit::new(miterlimit);

    let paint = super::fill::resolve_paint(tree, attrs, AId::Stroke, has_bbox)?;

    let linecap = attrs.get_str_or(AId::StrokeLinecap, "butt");
    let linecap = match linecap {
        "butt" => tree::LineCap::Butt,
        "round" => tree::LineCap::Round,
        "square" => tree::LineCap::Square,
        _ => tree::LineCap::Butt,
    };

    let linejoin = attrs.get_str_or(AId::StrokeLinejoin, "miter");
    let linejoin = match linejoin {
        "miter" => tree::LineJoin::Miter,
        "round" => tree::LineJoin::Round,
        "bevel" => tree::LineJoin::Bevel,
        _ => tree::LineJoin::Miter,
    };

    let dasharray = conv_dasharray(attrs.get_value(AId::StrokeDasharray));

    let stroke = tree::Stroke {
        paint,
        dasharray,
        dashoffset,
        miterlimit,
        opacity,
        width,
        linecap,
        linejoin,
    };

    Some(stroke)
}

// Prepare the 'stroke-dasharray' according to:
// https://www.w3.org/TR/SVG11/painting.html#StrokeDasharrayProperty
fn conv_dasharray(av: Option<&AValue>) -> Option<svgdom::NumberList> {
    if let Some(&AValue::NumberList(ref list)) = av {
        // `A negative value is an error`
        if list.iter().any(|n| n.is_sign_negative()) {
            return None;
        }

        // `If the sum of the values is zero, then the stroke is rendered
        // as if a value of none were specified.`
        {
            // no Iter::sum(), because of f64

            let mut sum = 0.0f64;
            for n in list.iter() {
                sum += *n;
            }

            if sum.fuzzy_eq(&0.0) {
                return None;
            }
        }

        // `If an odd number of values is provided, then the list of values
        // is repeated to yield an even number of values.`
        if list.len() % 2 != 0 {
            let mut tmp_list = list.clone();
            tmp_list.extend_from_slice(list);

            return Some(tmp_list.clone());
        }

        return Some(list.clone());
    }

    None
}
