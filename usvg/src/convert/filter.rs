// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::collections::HashSet;

use crate::svgtree;
use crate::tree;
use super::prelude::*;
use super::paint_server::{
    resolve_number,
    convert_units,
};


pub fn convert(
    node: svgtree::Node,
    state: &State,
    tree: &mut tree::Tree,
) -> Option<String> {
    if tree.defs_by_id(node.element_id()).is_some() {
        return Some(node.element_id().to_string());
    }

    let units = convert_units(node, AId::FilterUnits, tree::Units::ObjectBoundingBox);
    let primitive_units = convert_units(node, AId::PrimitiveUnits, tree::Units::UserSpaceOnUse);

    let rect = Rect::new(
        resolve_number(node, AId::X, units, state, Length::new(-10.0, Unit::Percent)),
        resolve_number(node, AId::Y, units, state, Length::new(-10.0, Unit::Percent)),
        resolve_number(node, AId::Width, units, state, Length::new(120.0, Unit::Percent)),
        resolve_number(node, AId::Height, units, state, Length::new(120.0, Unit::Percent)),
    );
    let rect = try_opt_warn_or!(
        rect, None,
        "Filter '{}' has an invalid region. Skipped.", node.element_id(),
    );

    let node_with_children = find_filter_with_children(node)?;
    let children = collect_children(&node_with_children, primitive_units, state);
    if children.is_empty() {
        return None;
    }

    tree.append_to_defs(
        tree::NodeKind::Filter(tree::Filter {
            id: node.element_id().to_string(),
            units,
            primitive_units,
            rect,
            children,
        })
    );

    Some(node.element_id().to_string())
}

fn find_filter_with_children(
    node: svgtree::Node,
) -> Option<svgtree::Node> {
    for link_id in node.href_iter() {
        let link = node.document().get(link_id);
        if !link.has_tag_name(EId::Filter) {
            warn!(
                "Filter '{}' cannot reference '{}' via 'xlink:href'.",
                node.element_id(), link.tag_name().unwrap()
            );
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
    filter: &svgtree::Node,
    units: tree::Units,
    state: &State,
) -> Vec<tree::FilterPrimitive> {
    let mut primitives = Vec::new();

    let mut results = FilterResults {
        names: HashSet::new(),
        idx: 1,
    };

    for child in filter.children() {
        let kind = match child.tag_name() {
            Some(EId::FeGaussianBlur) => {
                convert_fe_gaussian_blur(child, &primitives)
            }
            Some(EId::FeOffset) => {
                convert_fe_offset(child, &primitives, state)
            }
            Some(EId::FeBlend) => {
                convert_fe_blend(child, &primitives)
            }
            Some(EId::FeFlood) => {
                convert_fe_flood(child)
            }
            Some(EId::FeComposite) => {
                convert_fe_composite(child, &primitives)
            }
            Some(EId::FeMerge) => {
                convert_fe_merge(child, &primitives)
            }
            Some(EId::FeTile) => {
                convert_fe_tile(child, &primitives)
            }
            Some(EId::FeImage) => {
                convert_fe_image(child, state)
            }
            Some(EId::FeComponentTransfer) => {
                convert_fe_component_transfer(child, &primitives)
            }
            Some(name) => {
                warn!("Filter with '{}' child is not supported.", name);
                continue;
            }
            None => continue,
        };

        let fe = convert_primitive(child, kind, units, state, &mut results);
        primitives.push(fe);
    }

    primitives
}

fn convert_primitive(
    fe: svgtree::Node,
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
        color_interpolation: fe.find_attribute(AId::ColorInterpolationFilters).unwrap_or_default(),
        result: gen_result(fe, results),
        kind,
    }
}

fn convert_fe_gaussian_blur(
    fe: svgtree::Node,
    primitives: &[tree::FilterPrimitive],
) -> tree::FilterKind {
    let text = fe.attribute::<&str>(AId::StdDeviation).unwrap_or("0 0");
    let mut parser = svgtypes::NumberListParser::from(text);

    let n1 = parser.next().and_then(|n| n.ok());
    let n2 = parser.next().and_then(|n| n.ok());
    // `stdDeviation` must have no more than two values.
    // Otherwise we should fallback to `0 0`.
    let n3 = parser.next().and_then(|n| n.ok());

    let (mut std_dev_x, mut std_dev_y) = match (n1, n2, n3) {
        (Some(n1), Some(n2), None) => (n1, n2),
        (Some(n1), None, None) => (n1, n1),
        _ => (0.0, 0.0),
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
    fe: svgtree::Node,
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
    fe: svgtree::Node,
    primitives: &[tree::FilterPrimitive],
) -> tree::FilterKind {
    let mode = match fe.attribute(AId::Mode).unwrap_or("normal") {
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

fn convert_fe_flood(
    fe: svgtree::Node,
) -> tree::FilterKind {
    let color = fe.attribute(AId::FloodColor).unwrap_or_else(tree::Color::black);
    let opacity = fe.attribute(AId::FloodOpacity).unwrap_or_default();
    tree::FilterKind::FeFlood(tree::FeFlood {
        color,
        opacity,
    })
}

fn get_coeff(
    node: svgtree::Node,
    aid: AId,
) -> tree::CompositingCoefficient {
    let k: f64 = node.attribute(aid).unwrap_or(0.0);
    f64_bound(0.0, k, 1.0).into()
}

fn convert_fe_composite(
    fe: svgtree::Node,
    primitives: &[tree::FilterPrimitive],
) -> tree::FilterKind {
    let operator = match fe.attribute(AId::Operator).unwrap_or("over") {
        "in"            => tree::FeCompositeOperator::In,
        "out"           => tree::FeCompositeOperator::Out,
        "atop"          => tree::FeCompositeOperator::Atop,
        "xor"           => tree::FeCompositeOperator::Xor,
        "arithmetic"    => {
            tree::FeCompositeOperator::Arithmetic {
                k1: get_coeff(fe, AId::K1),
                k2: get_coeff(fe, AId::K2),
                k3: get_coeff(fe, AId::K3),
                k4: get_coeff(fe, AId::K4),
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
    fe: svgtree::Node,
    primitives: &[tree::FilterPrimitive],
) -> tree::FilterKind {
    let mut inputs = Vec::new();
    for child in fe.children() {
        inputs.push(resolve_input(child, AId::In, primitives));
    }

    tree::FilterKind::FeMerge(tree::FeMerge {
        inputs,
    })
}

fn convert_fe_image(
    fe: svgtree::Node,
    state: &State,
) -> tree::FilterKind {
    let aspect = fe.attribute(AId::PreserveAspectRatio).unwrap_or_default();
    let rendering_mode = fe
        .find_attribute(AId::ImageRendering)
        .unwrap_or(state.opt.image_rendering);

    let href = match fe.attribute(AId::Href) {
        Some(s) => s,
        _ => {
            warn!("The 'feImage' element lacks the 'xlink:href' attribute. Skipped.");
            return tree::FilterKind::FeImage(tree::FeImage {
                aspect,
                rendering_mode,
                data: tree::FeImageKind::None,
            });
        }
    };

    let href = super::image::get_href_data(fe.element_id(), href, state.opt.path.as_ref());
    let (img_data, format) = match href {
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
    fe: svgtree::Node,
    primitives: &[tree::FilterPrimitive],
) -> tree::FilterKind {
    tree::FilterKind::FeTile(tree::FeTile {
        input: resolve_input(fe, AId::In, primitives),
    })
}

fn convert_fe_component_transfer(
    fe: svgtree::Node,
    primitives: &[tree::FilterPrimitive],
) -> tree::FilterKind {
    let mut kind = tree::FeComponentTransfer {
        input: resolve_input(fe, AId::In, primitives),
        func_r: tree::TransferFunction::Identity,
        func_g: tree::TransferFunction::Identity,
        func_b: tree::TransferFunction::Identity,
        func_a: tree::TransferFunction::Identity,
    };

    for child in fe.children().filter(|n| n.is_element()) {
        if let Some(func) = convert_transfer_function(child) {
            match child.tag_name().unwrap() {
                EId::FeFuncR => kind.func_r = func,
                EId::FeFuncG => kind.func_g = func,
                EId::FeFuncB => kind.func_b = func,
                EId::FeFuncA => kind.func_a = func,
                _ => {}
            }
        }
    }

    tree::FilterKind::FeComponentTransfer(kind)
}

fn convert_transfer_function(
    node: svgtree::Node,
) -> Option<tree::TransferFunction> {
    match node.attribute(AId::Type)? {
        "identity" => {
            Some(tree::TransferFunction::Identity)
        }
        "table" => {
            match node.attribute::<&svgtypes::NumberList>(AId::TableValues) {
                Some(values) => Some(tree::TransferFunction::Table(values.0.clone())),
                None => Some(tree::TransferFunction::Table(Vec::new())),
            }
        }
        "discrete" => {
            match node.attribute::<&svgtypes::NumberList>(AId::TableValues) {
                Some(values) => Some(tree::TransferFunction::Discrete(values.0.clone())),
                None => Some(tree::TransferFunction::Discrete(Vec::new())),
            }
        }
        "linear" => {
            Some(tree::TransferFunction::Linear {
                slope: node.attribute(AId::Slope).unwrap_or(1.0),
                intercept: node.attribute(AId::Intercept).unwrap_or(0.0),
            })
        }
        "gamma" => {
            Some(tree::TransferFunction::Gamma {
                amplitude: node.attribute(AId::Amplitude).unwrap_or(1.0),
                exponent: node.attribute(AId::Exponent).unwrap_or(1.0),
                offset: node.attribute(AId::Offset).unwrap_or(0.0),
            })
        }
        _ => None,
    }
}

fn resolve_input(
    node: svgtree::Node,
    aid: AId,
    primitives: &[tree::FilterPrimitive],
) -> tree::FilterInput {
    match node.attribute(aid) {
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

fn parse_in(
    s: &str,
) -> tree::FilterInput {
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

fn gen_result(
    node: svgtree::Node,
    results: &mut FilterResults,
) -> String {
    match node.attribute::<&str>(AId::Result) {
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
