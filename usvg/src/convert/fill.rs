// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom::{
    self,
    PaintFallback,
};

// self
use tree;
use tree::prelude::*;
use super::prelude::*;


pub fn convert(
    tree: &tree::Tree,
    attrs: &svgdom::Attributes,
    has_bbox: bool,
) -> Option<tree::Fill> {
    let paint = resolve_paint(tree, attrs, AId::Fill, has_bbox)?;

    let fill_opacity = attrs.get_number_or(AId::FillOpacity, 1.0);

    let fill_rule = attrs.get_str_or(AId::FillRule, "nonzero");
    let fill_rule = match fill_rule {
        "evenodd" => tree::FillRule::EvenOdd,
        _ => tree::FillRule::NonZero,
    };

    let fill = tree::Fill {
        paint,
        opacity: fill_opacity.into(),
        rule: fill_rule,
    };

    Some(fill)
}

pub fn resolve_paint(
    tree: &tree::Tree,
    attrs: &svgdom::Attributes,
    aid: AId,
    has_bbox: bool,
) -> Option<tree::Paint> {
    match attrs.get_type(aid) {
        Some(&AValue::Color(c)) => {
            Some(tree::Paint::Color(c))
        }
        Some(&AValue::Paint(ref link, fallback)) => {
            if link.is_paint_server() {
                if let Some(node) = tree.defs_by_id(&link.id()) {
                    let server_units = match *node.borrow() {
                        tree::NodeKind::LinearGradient(ref lg) => lg.units,
                        tree::NodeKind::RadialGradient(ref rg) => rg.units,
                        tree::NodeKind::Pattern(ref patt) => patt.units,
                        // safe, because we already checked for is_paint_server()
                        _ => unreachable!(),
                    };

                    // We can use a paint server node with ObjectBoundingBox units
                    // for painting only when the shape itself has a bbox.
                    //
                    // See SVG spec 7.11 for details.
                    if !has_bbox && server_units == tree::Units::ObjectBoundingBox {
                        if let Some(PaintFallback::Color(c)) = fallback {
                            Some(tree::Paint::Color(c))
                        } else {
                            None
                        }
                    } else {
                        Some(tree::Paint::Link(node.id().to_string()))
                    }
                } else if let Some(PaintFallback::Color(c)) = fallback {
                    Some(tree::Paint::Color(c))
                } else {
                    None
                }
            } else {
                warn!("'{}' cannot be used to {} the shape.", link.tag_name(), aid);
                None
            }
        }
        Some(&AValue::None) => {
            None
        }
        Some(av) => {
            warn!("An invalid {} value: {}. Skipped.", aid, av);
            None
        }
        None => None,
    }
}
