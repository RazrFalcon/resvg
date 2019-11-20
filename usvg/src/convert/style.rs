// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{svgtree, tree};
use super::{prelude::*, paint_server};


pub fn resolve_fill(
    node: svgtree::Node,
    has_bbox: bool,
    state: &State,
    tree: &mut tree::Tree,
) -> Option<tree::Fill> {
    if state.parent_clip_path.is_some() {
        // A `clipPath` child can be filled only with a black color.
        return Some(tree::Fill {
            paint: tree::Paint::Color(tree::Color::black()),
            opacity: tree::Opacity::default(),
            rule: node.find_attribute(AId::ClipRule).unwrap_or_default(),
        });
    }

    let mut sub_opacity = tree::Opacity::default();
    let paint = if let Some(n) = node.find_node_with_attribute(AId::Fill) {
        convert_paint(n, AId::Fill, has_bbox, state, &mut sub_opacity, tree)?
    } else {
        tree::Paint::Color(tree::Color::black())
    };

    Some(tree::Fill {
        paint,
        opacity: sub_opacity * node.find_attribute(AId::FillOpacity).unwrap_or_default(),
        rule: node.find_attribute(AId::FillRule).unwrap_or_default(),
    })
}

pub fn resolve_stroke(
    node: svgtree::Node,
    has_bbox: bool,
    state: &State,
    tree: &mut tree::Tree,
) -> Option<tree::Stroke> {
    if state.parent_clip_path.is_some() {
        // A `clipPath` child cannot be stroked.
        return None;
    }

    let mut sub_opacity = tree::Opacity::default();
    let paint = if let Some(n) = node.find_node_with_attribute(AId::Stroke) {
        convert_paint(n, AId::Stroke, has_bbox, state, &mut sub_opacity, tree)?
    } else {
        return None;
    };

    let width = node.resolve_valid_length(AId::StrokeWidth, state, 1.0)?;

    // Must be bigger than 1.
    let miterlimit = node.find_attribute(AId::StrokeMiterlimit).unwrap_or(4.0);
    let miterlimit = if miterlimit < 1.0 { 1.0 } else { miterlimit };
    let miterlimit = tree::StrokeMiterlimit::new(miterlimit);

    let stroke = tree::Stroke {
        paint,
        dasharray: conv_dasharray(node, state),
        dashoffset: node.resolve_length(AId::StrokeDashoffset, state, 0.0) as f32,
        miterlimit,
        opacity: sub_opacity * node.find_attribute(AId::StrokeOpacity).unwrap_or_default(),
        width: tree::StrokeWidth::new(width),
        linecap: node.find_attribute(AId::StrokeLinecap).unwrap_or_default(),
        linejoin: node.find_attribute(AId::StrokeLinejoin).unwrap_or_default(),
    };

    Some(stroke)
}

fn convert_paint(
    node: svgtree::Node,
    aid: AId,
    has_bbox: bool,
    state: &State,
    opacity: &mut tree::Opacity,
    tree: &mut tree::Tree,
) -> Option<tree::Paint> {
    match node.attribute::<&svgtree::AttributeValue>(aid)? {
        svgtree::AttributeValue::CurrentColor => {
            let c = node.find_attribute(AId::Color).unwrap_or_else(tree::Color::black);
            Some(tree::Paint::Color(c))
        }
        svgtree::AttributeValue::Color(c) => {
            Some(tree::Paint::Color(*c))
        }
        svgtree::AttributeValue::Paint(func_iri, fallback) => {
            if let Some(link) = node.document().element_by_id(func_iri) {
                let tag_name = link.tag_name().unwrap();
                if tag_name.is_paint_server() {
                    match paint_server::convert(link, state, tree) {
                        Some(paint_server::ServerOrColor::Server { id, units }) => {
                            // We can use a paint server node with ObjectBoundingBox units
                            // for painting only when the shape itself has a bbox.
                            //
                            // See SVG spec 7.11 for details.
                            if !has_bbox && units == tree::Units::ObjectBoundingBox {
                                from_fallback(node, *fallback)
                            } else {
                                Some(tree::Paint::Link(id))
                            }
                        }
                        Some(paint_server::ServerOrColor::Color { color, opacity: so }) => {
                            *opacity = so;
                            Some(tree::Paint::Color(color))
                        }
                        None => {
                            from_fallback(node, *fallback)
                        }
                    }
                } else {
                    warn!("'{}' cannot be used to {} a shape.", tag_name, aid);
                    None
                }
            } else {
                from_fallback(node, *fallback)
            }
        }
        _ => {
            None
        }
    }
}

fn from_fallback(
    node: svgtree::Node,
    fallback: Option<svgtypes::PaintFallback>,
) -> Option<tree::Paint> {
    match fallback? {
        svgtypes::PaintFallback::None => {
            None
        }
        svgtypes::PaintFallback::CurrentColor => {
            let c = node.find_attribute(AId::Color).unwrap_or_else(tree::Color::black);
            Some(tree::Paint::Color(c))
        }
        svgtypes::PaintFallback::Color(c) => {
            Some(tree::Paint::Color(c))
        }
    }
}

// Prepare the 'stroke-dasharray' according to:
// https://www.w3.org/TR/SVG11/painting.html#StrokeDasharrayProperty
fn conv_dasharray(
    node: svgtree::Node,
    state: &State,
) -> Option<Vec<f64>> {
    let node = node.find_node_with_attribute(AId::StrokeDasharray)?;
    let list = super::units::convert_list(node, AId::StrokeDasharray, state)?;

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
