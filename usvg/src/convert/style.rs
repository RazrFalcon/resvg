// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom;

// self
use crate::tree;
use super::prelude::*;
use super::{
    paint_server,
    switch,
};


pub fn resolve_fill(
    node: &svgdom::Node,
    has_bbox: bool,
    state: &State,
    tree: &mut tree::Tree,
) -> Option<tree::Fill> {
    if state.is_in_clip_path() {
        // A `clipPath` child can be filled only with a black color.
        return Some(tree::Fill {
            paint: tree::Paint::Color(tree::Color::black()),
            opacity: tree::Opacity::default(),
            rule: node.find_enum(AId::ClipRule),
        });
    }


    let mut sub_opacity = tree::Opacity::default();
    let paint = if let Some(n) = node.find_node_with_attribute(AId::Fill) {
        convert_paint(&n, AId::Fill, has_bbox, state, &mut sub_opacity, tree)?
    } else {
        tree::Paint::Color(tree::Color::black())
    };

    let fill_opacity = node.resolve_length(AId::FillOpacity, state, 1.0) * sub_opacity.value();

    // The `fill-rule` should be ignored.
    // https://www.w3.org/TR/SVG2/text.html#TextRenderingOrder
    //
    // 'Since the fill-rule property does not apply to SVG text elements,
    // the specific order of the subpaths within the equivalent path does not matter.'
    let fill_rule = if state.current_root.is_tag_name(EId::Text) {
        tree::FillRule::NonZero
    } else {
        node.find_enum(AId::FillRule)
    };

    Some(tree::Fill {
        paint,
        opacity: fill_opacity.into(),
        rule: fill_rule,
    })
}

pub fn resolve_stroke(
    node: &svgdom::Node,
    has_bbox: bool,
    state: &State,
    tree: &mut tree::Tree,
) -> Option<tree::Stroke> {
    if state.is_in_clip_path() {
        // A `clipPath` child cannot be stroked.
        return None;
    }


    let mut sub_opacity = tree::Opacity::default();
    let paint = if let Some(n) = node.find_node_with_attribute(AId::Stroke) {
        convert_paint(&n, AId::Stroke, has_bbox, state, &mut sub_opacity, tree)?
    } else {
        return None;
    };

    let dashoffset  = node.resolve_length(AId::StrokeDashoffset, state, 0.0) as f32;
    let miterlimit  = node.resolve_length(AId::StrokeMiterlimit, state, 4.0);
    let opacity     = node.resolve_length(AId::StrokeOpacity, state, 1.0) * sub_opacity.value();
    let width       = node.resolve_length(AId::StrokeWidth, state, 1.0);

    if !(width > 0.0) {
        return None;
    }

    let width = tree::StrokeWidth::new(width);

    // Must be bigger than 1.
    let miterlimit = if miterlimit < 1.0 { 1.0 } else { miterlimit };
    let miterlimit = tree::StrokeMiterlimit::new(miterlimit);

    let linecap = node.find_enum(AId::StrokeLinecap);
    let linejoin = node.find_enum(AId::StrokeLinejoin);
    let dasharray = conv_dasharray(node, state);

    let stroke = tree::Stroke {
        paint,
        dasharray,
        dashoffset,
        miterlimit,
        opacity: opacity.into(),
        width,
        linecap,
        linejoin,
    };

    Some(stroke)
}

fn convert_paint(
    node: &svgdom::Node,
    aid: AId,
    has_bbox: bool,
    state: &State,
    opacity: &mut tree::Opacity,
    tree: &mut tree::Tree,
) -> Option<tree::Paint> {
    let av = node.attributes().get_value(aid).cloned()?;
    match av {
        AValue::Color(c) => {
            Some(tree::Paint::Color(c))
        }
        AValue::Paint(ref link, fallback) => {
            if link.is_paint_server() {
                match paint_server::convert(link, state, tree) {
                    Some(paint_server::ServerOrColor::Server { id, units }) => {
                        // We can use a paint server node with ObjectBoundingBox units
                        // for painting only when the shape itself has a bbox.
                        //
                        // See SVG spec 7.11 for details.
                        if !has_bbox && units == tree::Units::ObjectBoundingBox {
                            from_fallback(fallback)
                        } else {
                            Some(tree::Paint::Link(id))
                        }
                    }
                    Some(paint_server::ServerOrColor::Color { color, opacity: so }) => {
                        *opacity = so;
                        Some(tree::Paint::Color(color))
                    }
                    None => {
                        from_fallback(fallback)
                    }
                }
            } else {
                warn!("'{}' cannot be used to {} a shape.", link.tag_name(), aid);
                None
            }
        }
        AValue::None => {
            None
        }
        _ => {
            warn!("An invalid {} value: {}. Skipped.", aid, av);
            None
        }
    }
}

fn from_fallback(fallback: Option<svgdom::PaintFallback>) -> Option<tree::Paint> {
    match fallback? {
        svgdom::PaintFallback::None => {
            None
        }
        svgdom::PaintFallback::CurrentColor => {
            warn!("'currentColor' must be already resolved");
            None
        }
        svgdom::PaintFallback::Color(c) => {
            Some(tree::Paint::Color(c))
        }
    }
}

// Prepare the 'stroke-dasharray' according to:
// https://www.w3.org/TR/SVG11/painting.html#StrokeDasharrayProperty
fn conv_dasharray(node: &svgdom::Node, state: &State) -> Option<Vec<f64>> {
    let node = node.find_node_with_attribute(AId::StrokeDasharray)?;
    let list = super::units::convert_list(&node, AId::StrokeDasharray, state)?;

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
        tmp_list.extend_from_slice(&list);
        return Some(tmp_list);
    }

    Some(list)
}

pub fn is_visible_element(node: &svgdom::Node, opt: &Options) -> bool {
    let display = node.attributes().get_value(AId::Display) != Some(&AValue::None);

       display
    && node.is_valid_transform(AId::Transform)
    && switch::is_condition_passed(&node, opt)
}
