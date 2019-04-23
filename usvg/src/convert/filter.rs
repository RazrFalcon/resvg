// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::collections::HashSet;

// external
use svgdom;

// self
use crate::tree;
use super::prelude::*;
use super::paint_server::{
    resolve_number,
    convert_units,
};


pub fn convert(
    node: &svgdom::Node,
    state: &State,
    tree: &mut tree::Tree,
) -> Option<String> {
    if tree.defs_by_id(node.id().as_str()).is_some() {
        return Some(node.id().clone());
    }

    let units = convert_units(node, AId::FilterUnits, tree::Units::ObjectBoundingBox);
    let primitive_units = convert_units(node, AId::PrimitiveUnits, tree::Units::UserSpaceOnUse);

    let rect = Rect::new(
        resolve_number(node, AId::X, units, state, Length::new(-10.0, Unit::Percent)),
        resolve_number(node, AId::Y, units, state, Length::new(-10.0, Unit::Percent)),
        resolve_number(node, AId::Width, units, state, Length::new(120.0, Unit::Percent)),
        resolve_number(node, AId::Height, units, state, Length::new(120.0, Unit::Percent)),
    );
    let rect = try_opt_warn!(rect, None, "Filter '{}' has an invalid region. Skipped.", node.id());

    let node_with_children = find_filter_with_children(node)?;
    let children = collect_children(&node_with_children, primitive_units, state);
    if children.is_empty() {
        return None;
    }

    tree.append_to_defs(
        tree::NodeKind::Filter(tree::Filter {
            id: node.id().clone(),
            units,
            primitive_units,
            rect,
            children,
        })
    );

    Some(node.id().clone())
}

fn find_filter_with_children(node: &svgdom::Node) -> Option<svgdom::Node> {
    for link in node.href_iter() {
        if !link.is_tag_name(EId::Filter) {
            warn!("Filter '{}' cannot reference '{}' via 'xlink:href'.",
                  node.id(), link.tag_id().unwrap());
            return None;
        }

        if link.has_children() {
            return Some(link.clone());
        }
    }

    None
}

struct FilterResults {
    names: HashSet<String>,
    idx: usize,
}

fn collect_children(
    filter: &svgdom::Node,
    units: tree::Units,
    state: &State,
) -> Vec<tree::FilterPrimitive> {
    let mut primitives = Vec::new();

    let mut results = FilterResults {
        names: HashSet::new(),
        idx: 1,
    };

    for child in filter.children() {
        let kind = match child.tag_id() {
            Some(EId::FeGaussianBlur) => {
                convert_fe_gaussian_blur(&child, &primitives)
            }
            Some(EId::FeOffset) => {
                convert_fe_offset(&child, &primitives, state)
            }
            Some(EId::FeBlend) => {
                convert_fe_blend(&child, &primitives)
            }
            Some(EId::FeFlood) => {
                convert_fe_flood(&child)
            }
            Some(EId::FeComposite) => {
                convert_fe_composite(&child, &primitives)
            }
            Some(EId::FeMerge) => {
                convert_fe_merge(&child, &primitives)
            }
            Some(EId::FeTile) => {
                convert_fe_tile(&child, &primitives)
            }
            Some(EId::FeImage) => {
                convert_fe_image(&child, state)
            }
            Some(_) => {
                warn!("Filter with '{}' child is not supported.", child.tag_name());
                continue;
            }
            None => continue,
        };

        let fe = convert_primitive(&child, kind, units, state, &mut results);
        primitives.push(fe);
    }

    primitives
}

fn convert_primitive(
    fe: &svgdom::Node,
    kind: tree::FilterKind,
    units: tree::Units,
    state: &State,
    results: &mut FilterResults,
) -> tree::FilterPrimitive {
    tree::FilterPrimitive {
        x: fe.try_convert_length(AId::X, units, state),
        y: fe.try_convert_length(AId::Y, units, state),
        // TODO: validate and test
        width: fe.try_convert_length(AId::Width, units, state),
        height: fe.try_convert_length(AId::Height, units, state),
        color_interpolation: fe.find_enum(AId::ColorInterpolationFilters),
        result: gen_result(fe, results),
        kind,
    }
}

fn convert_fe_gaussian_blur(
    fe: &svgdom::Node,
    primitives: &[tree::FilterPrimitive],
) -> tree::FilterKind {
    let attrs = fe.attributes();

    let std_dev_list = attrs.get_number_list(AId::StdDeviation).cloned();

    let (mut std_dev_x, mut std_dev_y) = match std_dev_list {
        Some(list) => {
            if list.len() == 1 {
                (list[0], list[0])
            } else if list.len() == 2 {
                (list[0], list[1])
            } else {
                (0.0, 0.0)
            }
        }
        None => {
            (0.0, 0.0)
        }
    };

    if std_dev_x.is_sign_negative() { std_dev_x = 0.0; }
    if std_dev_y.is_sign_negative() { std_dev_y = 0.0; }

    tree::FilterKind::FeGaussianBlur(tree::FeGaussianBlur {
        input: resolve_input(fe, AId::In, primitives),
        std_dev_x: std_dev_x.into(),
        std_dev_y: std_dev_y.into(),
    })
}

fn convert_fe_offset(
    fe: &svgdom::Node,
    primitives: &[tree::FilterPrimitive],
    state: &State,
) -> tree::FilterKind {
    tree::FilterKind::FeOffset(tree::FeOffset {
        input: resolve_input(fe, AId::In, primitives),
        dx: fe.convert_user_length(AId::Dx, state, Length::zero()),
        dy: fe.convert_user_length(AId::Dy, state, Length::zero()),
    })
}

fn convert_fe_blend(
    fe: &svgdom::Node,
    primitives: &[tree::FilterPrimitive],
) -> tree::FilterKind {
    let attrs = fe.attributes();

    let mode = match attrs.get_str_or(AId::Mode, "normal") {
        "multiply"  => tree::FeBlendMode::Multiply,
        "screen"    => tree::FeBlendMode::Screen,
        "darken"    => tree::FeBlendMode::Darken,
        "lighten"   => tree::FeBlendMode::Lighten,
        _           => tree::FeBlendMode::Normal,
    };

    let input1 = resolve_input(fe, AId::In, primitives);
    let input2 = resolve_input(fe, AId::In2, primitives);

    tree::FilterKind::FeBlend(tree::FeBlend {
        mode,
        input1,
        input2,
    })
}

fn convert_fe_flood(fe: &svgdom::Node) -> tree::FilterKind {
    let attrs = fe.attributes();

    let color = attrs.get_color(AId::FloodColor).unwrap_or_else(tree::Color::black);
    let opacity = fe.convert_opacity(AId::FloodOpacity);

    tree::FilterKind::FeFlood(tree::FeFlood {
        color,
        opacity,
    })
}

fn get_coeff(attrs: &svgdom::Attributes, aid: AId) -> tree::CompositingCoefficient {
    let k = match attrs.get_value(aid) {
        Some(AValue::Number(n)) => *n,
        _ => 0.0,
    };

    f64_bound(0.0, k, 1.0).into()
}

fn convert_fe_composite(
    fe: &svgdom::Node,
    primitives: &[tree::FilterPrimitive],
) -> tree::FilterKind {
    let ref attrs = fe.attributes();

    let operator = match attrs.get_str_or(AId::Operator, "over") {
        "in"            => tree::FeCompositeOperator::In,
        "out"           => tree::FeCompositeOperator::Out,
        "atop"          => tree::FeCompositeOperator::Atop,
        "xor"           => tree::FeCompositeOperator::Xor,
        "arithmetic"    => {
            tree::FeCompositeOperator::Arithmetic {
                k1: get_coeff(attrs, AId::K1),
                k2: get_coeff(attrs, AId::K2),
                k3: get_coeff(attrs, AId::K3),
                k4: get_coeff(attrs, AId::K4),
            }
        }
        _ => tree::FeCompositeOperator::Over,
    };

    let input1 = resolve_input(fe, AId::In, primitives);
    let input2 = resolve_input(fe, AId::In2, primitives);

    tree::FilterKind::FeComposite(tree::FeComposite {
        operator,
        input1,
        input2,
    })
}

fn convert_fe_merge(
    fe: &svgdom::Node,
    primitives: &[tree::FilterPrimitive],
) -> tree::FilterKind {
    let mut inputs = Vec::new();
    for child in fe.children() {
        inputs.push(resolve_input(&child, AId::In, primitives));
    }

    tree::FilterKind::FeMerge(tree::FeMerge {
        inputs,
    })
}

fn convert_fe_image(
    fe: &svgdom::Node,
    state: &State,
) -> tree::FilterKind {
    let ref attrs = fe.attributes();

    let aspect = super::convert_aspect(attrs);
    let rendering_mode = fe.try_find_enum(AId::ImageRendering)
                           .unwrap_or(state.opt.image_rendering);

    let href = match attrs.get_value(AId::Href) {
        Some(&AValue::String(ref s)) => s,
        _ => {
            warn!("The 'feImage' element lacks the 'xlink:href' attribute. Skipped.");
            return tree::FilterKind::FeImage(tree::FeImage {
                aspect,
                rendering_mode,
                data: tree::FeImageKind::None,
            });
        }
    };

    let (img_data, format) = match super::image::get_href_data(&*fe.id(), href, state.opt.path.as_ref()) {
        Some((data, format)) => (data, format),
        None => {
            return tree::FilterKind::FeImage(tree::FeImage {
                aspect,
                rendering_mode,
                data: tree::FeImageKind::None,
            });
        }
    };


    tree::FilterKind::FeImage(tree::FeImage {
        aspect,
        rendering_mode,
        data: tree::FeImageKind::Image(img_data, format),
    })
}

fn convert_fe_tile(
    fe: &svgdom::Node,
    primitives: &[tree::FilterPrimitive],
) -> tree::FilterKind {
    tree::FilterKind::FeTile(tree::FeTile {
        input: resolve_input(fe, AId::In, primitives),
    })
}

fn resolve_input(
    node: &svgdom::Node,
    aid: AId,
    primitives: &[tree::FilterPrimitive],
) -> tree::FilterInput {
    let attrs = node.attributes();

    match attrs.get_str(aid) {
        Some(s) => {
            let input = parse_in(s);

            // If `in` references an unknown `result` than fallback
            // to previous result or `SourceGraphic`.
            if let tree::FilterInput::Reference(ref name) = input {
                if !primitives.iter().any(|p| p.result == *name) {
                    return if let Some(ref prev) = primitives.last() {
                        tree::FilterInput::Reference(prev.result.clone())
                    } else {
                        tree::FilterInput::SourceGraphic
                    };
                }
            }

            input
        }
        None => {
            if let Some(ref prev) = primitives.last() {
                // If `in` is not set and this is not the first primitive
                // than the input is a result of the previous primitive.
                tree::FilterInput::Reference(prev.result.clone())
            } else {
                // If `in` is not set and this is the first primitive
                // than the input is `SourceGraphic`.
                tree::FilterInput::SourceGraphic
            }
        }
    }
}

fn parse_in(s: &str) -> tree::FilterInput {
    match s {
        "SourceGraphic"     => tree::FilterInput::SourceGraphic,
        "SourceAlpha"       => tree::FilterInput::SourceAlpha,
        "BackgroundImage"   => tree::FilterInput::BackgroundImage,
        "BackgroundAlpha"   => tree::FilterInput::BackgroundAlpha,
        "FillPaint"         => tree::FilterInput::FillPaint,
        "StrokePaint"       => tree::FilterInput::StrokePaint,
        _                   => tree::FilterInput::Reference(s.to_string())
    }
}

fn gen_result(node: &svgdom::Node, results: &mut FilterResults) -> String {
    match node.attributes().get_str(AId::Result) {
        Some(s) => {
            // Remember predefined result.
            results.names.insert(s.to_string());
            results.idx += 1;

            s.to_string()
        }
        None => {
            // Generate an unique name for `result`.
            loop {
                let name = format!("result{}", results.idx);
                results.idx += 1;

                if !results.names.contains(&name) {
                    return name;
                }
            }
        }
    }
}
