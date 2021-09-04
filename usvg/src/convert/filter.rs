// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::collections::HashSet;

use crate::svgtree;
use crate::tree;
use super::prelude::*;
use super::paint_server::{resolve_number, convert_units};
use crate::convert::NodeIdGenerator;


pub fn convert(
    node: svgtree::Node,
    state: &State,
    id_generator: &mut NodeIdGenerator,
    tree: &mut tree::Tree,
) -> Result<Vec<String>, ()> {
    let value = match node.attribute::<&str>(AId::Filter) {
        Some(v) => v,
        None => return Ok(Vec::new()),
    };

    let mut has_invalid_urls = false;
    let mut ids = Vec::new();

    let mut create_base_filter_func = |kind, ids: &mut Vec<String>, tree: &mut tree::Tree| {
        let id = id_generator.gen_filter_id();
        ids.push(id.clone());

        // Filter functions, unlike `filter` elements, do not have a filter region.
        // We're currently do not support an unlimited region, so we simply use a fairly large one.
        // This if far from ideal, but good for now.
        // TODO: Should be fixed eventually.
        let rect = match kind {
            tree::FilterKind::FeDropShadow(_) | tree::FilterKind::FeGaussianBlur(_) => {
                Rect::new(-1.0, -1.0, 2.0, 2.0).unwrap()
            }
            _ => Rect::new(-0.1, -0.1, 1.2, 1.2).unwrap(),
        };

        tree.append_to_defs(tree::NodeKind::Filter(tree::Filter {
            id,
            units: tree::Units::ObjectBoundingBox,
            primitive_units: tree::Units::UserSpaceOnUse,
            rect,
            children: vec![
                tree::FilterPrimitive {
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                    // Unlike `filter` elements, filter functions use sRGB colors by default.
                    color_interpolation: tree::ColorInterpolation::SRGB,
                    result: "result".to_string(),
                    kind,
                },
            ],
        }));
    };

    for func in svgtypes::FilterValueListParser::from(value) {
        let func = match func {
            Ok(v) => v,
            Err(e) => {
                // Skip the whole attribute list on error.
                log::warn!("Failed to parse a filter value cause {}. Skipping.", e);
                return Ok(Vec::new())
            }
        };

        match func {
            svgtypes::FilterValue::Blur(std_dev) => {
                create_base_filter_func(convert_blur(node, std_dev, state), &mut ids, tree)
            }
            svgtypes::FilterValue::DropShadow { color, dx, dy, std_dev } => {
                create_base_filter_func(
                    convert_drop_shadow(node, color, dx, dy, std_dev, state),
                    &mut ids,
                    tree,
                )
            }
            svgtypes::FilterValue::Brightness(amount) => {
                create_base_filter_func(convert_brightness(amount), &mut ids, tree)
            }
            svgtypes::FilterValue::Contrast(amount) => {
                create_base_filter_func(convert_contrast(amount), &mut ids, tree)
            }
            svgtypes::FilterValue::Grayscale(amount) => {
                create_base_filter_func(convert_grayscale(amount), &mut ids, tree)
            }
            svgtypes::FilterValue::HueRotate(angle) => {
                create_base_filter_func(convert_hue_rotate(angle), &mut ids, tree)
            }
            svgtypes::FilterValue::Invert(amount) => {
                create_base_filter_func(convert_invert(amount), &mut ids, tree)
            }
            svgtypes::FilterValue::Opacity(amount) => {
                create_base_filter_func(convert_opacity(amount), &mut ids, tree)
            }
            svgtypes::FilterValue::Sepia(amount) => {
                create_base_filter_func(convert_sepia(amount), &mut ids, tree)
            }
            svgtypes::FilterValue::Saturate(amount) => {
                create_base_filter_func(convert_saturate(amount), &mut ids, tree)
            }
            svgtypes::FilterValue::Url(url) => {
                if let Some(link) = node.document().element_by_id(url) {
                    if let Ok(res) = convert_url(link, state, tree) {
                        if let Some(id) = res {
                            ids.push(id);
                        }
                    } else {
                        has_invalid_urls = true;
                    }
                } else {
                    has_invalid_urls = true;
                }
            }
        }
    }

    // If a `filter` attribute had urls pointing to a missing elements
    // and there are no valid filters at all - this is an error.
    //
    // Note that an invalid url is not an error in general.
    if ids.is_empty() && has_invalid_urls {
        return Err(());
    }

    Ok(ids)
}

fn convert_url(
    node: svgtree::Node,
    state: &State,
    tree: &mut tree::Tree,
) -> Result<Option<String>, ()> {
    if tree.defs_by_id(node.element_id()).is_some() {
        return Ok(Some(node.element_id().to_string()));
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
        rect, Err(()),
        "Filter '{}' has an invalid region. Skipped.", node.element_id(),
    );

    let node_with_primitives = match find_filter_with_primitives(node) {
        Some(v) => v,
        None => return Err(()),
    };
    let primitives = collect_children(&node_with_primitives, primitive_units, state);
    if primitives.is_empty() {
        return Err(());
    }

    tree.append_to_defs(
        tree::NodeKind::Filter(tree::Filter {
            id: node.element_id().to_string(),
            units,
            primitive_units,
            rect,
            children: primitives,
        })
    );

    Ok(Some(node.element_id().to_string()))
}

fn find_filter_with_primitives(
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
            return Some(link);
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
        let kind = match try_opt_continue!(child.tag_name()) {
            EId::FeDropShadow => convert_fe_drop_shadow(child, &primitives, &state),
            EId::FeGaussianBlur => convert_fe_gaussian_blur(child, &primitives),
            EId::FeOffset => convert_fe_offset(child, &primitives, state),
            EId::FeBlend => convert_fe_blend(child, &primitives),
            EId::FeFlood => convert_fe_flood(child),
            EId::FeComposite => convert_fe_composite(child, &primitives),
            EId::FeMerge => convert_fe_merge(child, &primitives),
            EId::FeTile => convert_fe_tile(child, &primitives),
            EId::FeImage => convert_fe_image(child, state),
            EId::FeComponentTransfer => convert_fe_component_transfer(child, &primitives),
            EId::FeColorMatrix => convert_fe_color_matrix(child, &primitives),
            EId::FeConvolveMatrix => convert_fe_convolve_matrix(child, &primitives),
            EId::FeMorphology => convert_fe_morphology(child, &primitives),
            EId::FeDisplacementMap => convert_fe_displacement_map(child, &primitives),
            EId::FeTurbulence => convert_fe_turbulence(child),
            EId::FeDiffuseLighting => convert_fe_diffuse_lighting(child, &primitives),
            EId::FeSpecularLighting => convert_fe_specular_lighting(child, &primitives),
            tag_name => {
                warn!("'{}' is not a valid filter primitive. Skipped.", tag_name);
                continue;
            }
        };

        let fe = convert_primitive(child, kind, units, state, &mut results);
        primitives.push(fe);
    }

    // TODO: remove primitives which results are not used

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

fn convert_fe_drop_shadow(
    fe: svgtree::Node,
    primitives: &[tree::FilterPrimitive],
    state: &State,
) -> tree::FilterKind {
    let (std_dev_x, std_dev_y) = convert_std_dev_attr(fe, "2 2");

    tree::FilterKind::FeDropShadow(tree::FeDropShadow {
        input: resolve_input(fe, AId::In, primitives),
        dx: fe.convert_user_length(AId::Dx, state, Length::new_number(2.0)),
        dy: fe.convert_user_length(AId::Dy, state, Length::new_number(2.0)),
        std_dev_x: std_dev_x.into(),
        std_dev_y: std_dev_y.into(),
        color: fe.attribute(AId::FloodColor).unwrap_or_else(tree::Color::black),
        opacity: fe.attribute(AId::FloodOpacity).unwrap_or_default(),
    })
}

fn convert_fe_gaussian_blur(
    fe: svgtree::Node,
    primitives: &[tree::FilterPrimitive],
) -> tree::FilterKind {
    let (std_dev_x, std_dev_y) = convert_std_dev_attr(fe, "0 0");
    tree::FilterKind::FeGaussianBlur(tree::FeGaussianBlur {
        input: resolve_input(fe, AId::In, primitives),
        std_dev_x: std_dev_x.into(),
        std_dev_y: std_dev_y.into(),
    })
}

fn convert_std_dev_attr(fe: svgtree::Node, default: &str) -> (f64, f64) {
    let text = fe.attribute::<&str>(AId::StdDeviation).unwrap_or(default);
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

    (std_dev_x, std_dev_y)
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
        "normal" => tree::FeBlendMode::Normal,
        "multiply" => tree::FeBlendMode::Multiply,
        "screen" => tree::FeBlendMode::Screen,
        "overlay" => tree::FeBlendMode::Overlay,
        "darken" => tree::FeBlendMode::Darken,
        "lighten" => tree::FeBlendMode::Lighten,
        "color-dodge" => tree::FeBlendMode::ColorDodge,
        "color-burn" => tree::FeBlendMode::ColorBurn,
        "hard-light" => tree::FeBlendMode::HardLight,
        "soft-light" => tree::FeBlendMode::SoftLight,
        "difference" => tree::FeBlendMode::Difference,
        "exclusion" => tree::FeBlendMode::Exclusion,
        "hue" => tree::FeBlendMode::Hue,
        "saturation" => tree::FeBlendMode::Saturation,
        "color" => tree::FeBlendMode::Color,
        "luminosity" => tree::FeBlendMode::Luminosity,
        _ => tree::FeBlendMode::Normal,
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
    tree::FilterKind::FeFlood(tree::FeFlood {
        color: fe.attribute(AId::FloodColor).unwrap_or_else(tree::Color::black),
        opacity: fe.attribute(AId::FloodOpacity).unwrap_or_default(),
    })
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
                k1: fe.attribute(AId::K1).unwrap_or(0.0),
                k2: fe.attribute(AId::K2).unwrap_or(0.0),
                k3: fe.attribute(AId::K3).unwrap_or(0.0),
                k4: fe.attribute(AId::K4).unwrap_or(0.0),
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

    if let Some(node) = fe.attribute::<svgtree::Node>(AId::Href) {
        // If `feImage` references an existing SVG element,
        // simply store its ID and do not attempt to convert the element itself.
        // The problem is that `feImage` can reference an element outside `defs`,
        // and we should not create it manually.
        // Instead, after document conversion is finished, we should search for this ID
        // and if it does not exist - create it inside `defs`.
        return tree::FilterKind::FeImage(tree::FeImage {
            aspect,
            rendering_mode,
            data: tree::FeImageKind::Use(node.element_id().to_string()),
        });
    }

    let href = match fe.attribute(AId::Href) {
        Some(s) => s,
        _ => {
            warn!("The 'feImage' element lacks the 'xlink:href' attribute. Skipped.");
            return create_dummy_primitive();
        }
    };

    let href = super::image::get_href_data(fe.element_id(), href, state.opt);
    let img_data = match href {
        Some(data) => data,
        None => return create_dummy_primitive(),
    };

    tree::FilterKind::FeImage(tree::FeImage {
        aspect,
        rendering_mode,
        data: tree::FeImageKind::Image(img_data),
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
            match node.attribute::<&Vec<f64>>(AId::TableValues) {
                Some(values) => Some(tree::TransferFunction::Table(values.clone())),
                None => Some(tree::TransferFunction::Table(Vec::new())),
            }
        }
        "discrete" => {
            match node.attribute::<&Vec<f64>>(AId::TableValues) {
                Some(values) => Some(tree::TransferFunction::Discrete(values.clone())),
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

fn convert_fe_color_matrix(
    fe: svgtree::Node,
    primitives: &[tree::FilterPrimitive],
) -> tree::FilterKind {
    let kind = convert_color_matrix_kind(fe).unwrap_or_default();
    tree::FilterKind::FeColorMatrix(tree::FeColorMatrix {
        input: resolve_input(fe, AId::In, primitives),
        kind,
    })
}

fn convert_color_matrix_kind(
    fe: svgtree::Node
) -> Option<tree::FeColorMatrixKind> {
    match fe.attribute(AId::Type) {
        Some("saturate") => {
            if let Some(list) = fe.attribute::<&Vec<f64>>(AId::Values) {
                if !list.is_empty() {
                    let n = crate::utils::f64_bound(0.0, list[0], 1.0);
                    return Some(tree::FeColorMatrixKind::Saturate(n.into()));
                } else {
                    return Some(tree::FeColorMatrixKind::Saturate(1.0.into()));
                }
            }
        }
        Some("hueRotate") => {
            if let Some(list) = fe.attribute::<&Vec<f64>>(AId::Values) {
                if !list.is_empty() {
                    return Some(tree::FeColorMatrixKind::HueRotate(list[0]));
                } else {
                    return Some(tree::FeColorMatrixKind::HueRotate(0.0));
                }
            }
        }
        Some("luminanceToAlpha") => {
            return Some(tree::FeColorMatrixKind::LuminanceToAlpha);
        }
        _ => {
            // Fallback to `matrix`.
            if let Some(list) = fe.attribute::<&Vec<f64>>(AId::Values) {
                if list.len() == 20 {
                    return Some(tree::FeColorMatrixKind::Matrix(list.clone()));
                }
            }
        }
    }

    None
}

fn convert_fe_convolve_matrix(
    fe: svgtree::Node,
    primitives: &[tree::FilterPrimitive],
) -> tree::FilterKind {
    fn parse_target(target: Option<f64>, order: u32) -> Option<u32> {
        let default_target = (order as f32 / 2.0).floor() as u32;
        let target = target.unwrap_or(default_target as f64) as i32;
        if target < 0 || target >= order as i32 {
            None
        } else {
            Some(target as u32)
        }
    }

    let mut order_x = 3;
    let mut order_y = 3;
    if let Some(value) = fe.attribute::<&str>(AId::Order) {
        let mut s = svgtypes::Stream::from(value);
        let x = s.parse_list_integer().unwrap_or(3);
        let y = s.parse_list_integer().unwrap_or(x);
        if x > 0 && y > 0 {
            order_x = x as u32;
            order_y = y as u32;
        }
    }

    let mut matrix = Vec::new();
    if let Some(list) = fe.attribute::<&Vec<f64>>(AId::KernelMatrix) {
        if list.len() == (order_x * order_y) as usize {
            matrix = list.clone();
        }
    }

    let mut kernel_sum: f64 = matrix.iter().sum();
    // Round up to prevent float precision issues.
    kernel_sum = (kernel_sum * 1_000_000.0).round() / 1_000_000.0;
    if kernel_sum.is_fuzzy_zero() {
        kernel_sum = 1.0;
    }

    let divisor = fe.attribute::<f64>(AId::Divisor).unwrap_or(kernel_sum);
    if divisor.is_fuzzy_zero() {
        return create_dummy_primitive();
    }

    let bias = fe.attribute(AId::Bias).unwrap_or(0.0);

    let target_x = parse_target(fe.attribute(AId::TargetX), order_x);
    let target_y = parse_target(fe.attribute(AId::TargetY), order_y);

    let target_x = try_opt_or!(target_x, create_dummy_primitive());
    let target_y = try_opt_or!(target_y, create_dummy_primitive());

    let kernel_matrix = tree::ConvolveMatrix::new(
        target_x, target_y, order_x, order_y, matrix,
    );
    let kernel_matrix = try_opt_or!(kernel_matrix, create_dummy_primitive());

    let edge_mode = match fe.attribute(AId::EdgeMode).unwrap_or("duplicate") {
        "none" => tree::FeEdgeMode::None,
        "wrap" => tree::FeEdgeMode::Wrap,
        _      => tree::FeEdgeMode::Duplicate,
    };

    let preserve_alpha = fe.attribute(AId::PreserveAlpha).unwrap_or("false") == "true";

    tree::FilterKind::FeConvolveMatrix(tree::FeConvolveMatrix {
        input: resolve_input(fe, AId::In, primitives),
        matrix: kernel_matrix,
        divisor: tree::NonZeroF64::new(divisor).unwrap(),
        bias,
        edge_mode,
        preserve_alpha,
    })
}

fn convert_fe_morphology(
    fe: svgtree::Node,
    primitives: &[tree::FilterPrimitive],
) -> tree::FilterKind {
    let operator = match fe.attribute(AId::Operator).unwrap_or("erode") {
        "dilate" => tree::FeMorphologyOperator::Dilate,
        _        => tree::FeMorphologyOperator::Erode,
    };

    // Both radius are zero by default.
    let mut radius_x = tree::PositiveNumber::new(0.0);
    let mut radius_y = tree::PositiveNumber::new(0.0);
    if let Some(list) = fe.attribute::<&Vec<f64>>(AId::Radius) {
        let mut rx = 0.0;
        let mut ry = 0.0;
        if list.len() == 2 {
            rx = list[0];
            ry = list[1];
        } else if list.len() == 1 {
            rx = list[0];
            ry = list[0]; // The same as `rx`.
        }

        // If only one of the values is zero, reset it to 1.0
        // This is not specified in the spec, but this is how Chrome and Firefox works.
        if rx.is_fuzzy_zero() && !ry.is_fuzzy_zero() {
            rx = 1.0;
        }
        if !rx.is_fuzzy_zero() && ry.is_fuzzy_zero() {
            ry = 1.0;
        }

        // Both values must be positive.
        if rx.is_sign_positive() && ry.is_sign_positive() {
            radius_x = tree::PositiveNumber::new(rx);
            radius_y = tree::PositiveNumber::new(ry);
        }
    }

    tree::FilterKind::FeMorphology(tree::FeMorphology {
        input: resolve_input(fe, AId::In, primitives),
        operator,
        radius_x,
        radius_y,
    })
}

fn convert_fe_displacement_map(
    fe: svgtree::Node,
    primitives: &[tree::FilterPrimitive],
) -> tree::FilterKind {
    let parse_channel = |aid| {
        match fe.attribute(aid).unwrap_or("A") {
            "R" => tree::ColorChannel::R,
            "G" => tree::ColorChannel::G,
            "B" => tree::ColorChannel::B,
            _   => tree::ColorChannel::A,
        }
    };

    tree::FilterKind::FeDisplacementMap(tree::FeDisplacementMap {
        input1: resolve_input(fe, AId::In, primitives),
        input2: resolve_input(fe, AId::In2, primitives),
        scale: fe.attribute(AId::Scale).unwrap_or(0.0),
        x_channel_selector: parse_channel(AId::XChannelSelector),
        y_channel_selector: parse_channel(AId::YChannelSelector),
    })
}

fn convert_fe_turbulence(
    fe: svgtree::Node,
) -> tree::FilterKind {
    let mut base_frequency = Point::new(0.0.into(), 0.0.into());
    if let Some(list) = fe.attribute::<&Vec<f64>>(AId::BaseFrequency) {
        let mut x = 0.0;
        let mut y = 0.0;
        if list.len() == 2 {
            x = list[0];
            y = list[1];
        } else if list.len() == 1 {
            x = list[0];
            y = list[0]; // The same as `x`.
        }

        if x.is_sign_positive() && y.is_sign_positive() {
            base_frequency = Point::new(x.into(), y.into());
        }
    }

    let mut num_octaves = fe.attribute(AId::NumOctaves).unwrap_or(1.0);
    if num_octaves.is_sign_negative() {
        num_octaves = 0.0;
    }

    let kind = match fe.attribute(AId::Type).unwrap_or("turbulence") {
        "fractalNoise" => tree::FeTurbulenceKind::FractalNoise,
        _              => tree::FeTurbulenceKind::Turbulence,
    };

    tree::FilterKind::FeTurbulence(tree::FeTurbulence {
        base_frequency,
        num_octaves: num_octaves.round() as u32,
        seed: fe.attribute(AId::Seed).unwrap_or(0.0).trunc() as i32,
        stitch_tiles: fe.attribute(AId::StitchTiles) == Some("stitch"),
        kind,
    })
}

fn convert_fe_diffuse_lighting(
    fe: svgtree::Node,
    primitives: &[tree::FilterPrimitive],
) -> tree::FilterKind {
    let light_source = try_opt_or!(convert_light_source(fe), create_dummy_primitive());
    tree::FilterKind::FeDiffuseLighting(tree::FeDiffuseLighting {
        input: resolve_input(fe, AId::In, primitives),
        surface_scale: fe.attribute(AId::SurfaceScale).unwrap_or(1.0),
        diffuse_constant: fe.attribute(AId::DiffuseConstant).unwrap_or(1.0),
        lighting_color: convert_lighting_color(fe),
        light_source,
    })
}

fn convert_fe_specular_lighting(
    fe: svgtree::Node,
    primitives: &[tree::FilterPrimitive],
) -> tree::FilterKind {
    let light_source = try_opt_or!(convert_light_source(fe), create_dummy_primitive());

    let specular_exponent = fe.attribute(AId::SpecularExponent).unwrap_or(1.0);
    if !(1.0..=128.0).contains(&specular_exponent) {
        // When exponent is out of range, the whole filter primitive should be ignored.
        return create_dummy_primitive();
    }

    let specular_exponent = crate::utils::f64_bound(1.0, specular_exponent, 128.0);

    tree::FilterKind::FeSpecularLighting(tree::FeSpecularLighting {
        input: resolve_input(fe, AId::In, primitives),
        surface_scale: fe.attribute(AId::SurfaceScale).unwrap_or(1.0),
        specular_constant: fe.attribute(AId::SpecularConstant).unwrap_or(1.0),
        specular_exponent,
        lighting_color: convert_lighting_color(fe),
        light_source,
    })
}

#[inline(never)]
fn convert_lighting_color(
    node: svgtree::Node,
) -> tree::Color {
    match node.attribute::<&svgtree::AttributeValue>(AId::LightingColor) {
        Some(svgtree::AttributeValue::CurrentColor) => {
            node.find_attribute(AId::Color).unwrap_or_else(tree::Color::black)
        }
        Some(svgtree::AttributeValue::Color(c)) => *c,
        _ => tree::Color::white(),
    }
}

#[inline(never)]
fn convert_light_source(
    parent: svgtree::Node,
) -> Option<tree::FeLightSource> {
    let child = parent.children().find(|n|
        matches!(n.tag_name(), Some(EId::FeDistantLight) | Some(EId::FePointLight) | Some(EId::FeSpotLight))
    )?;

    match child.tag_name() {
        Some(EId::FeDistantLight) => {
            Some(tree::FeLightSource::FeDistantLight(tree::FeDistantLight {
                azimuth: child.attribute(AId::Azimuth).unwrap_or(0.0),
                elevation: child.attribute(AId::Elevation).unwrap_or(0.0),
            }))
        }
        Some(EId::FePointLight) => {
            Some(tree::FeLightSource::FePointLight(tree::FePointLight {
                x: child.attribute(AId::X).unwrap_or(0.0),
                y: child.attribute(AId::Y).unwrap_or(0.0),
                z: child.attribute(AId::Z).unwrap_or(0.0),
            }))
        }
        Some(EId::FeSpotLight) => {
            let mut specular_exponent = child.attribute(AId::SpecularExponent).unwrap_or(1.0);
            if specular_exponent.is_sign_negative() {
                specular_exponent = 1.0;
            }

            Some(tree::FeLightSource::FeSpotLight(tree::FeSpotLight {
                x: child.attribute(AId::X).unwrap_or(0.0),
                y: child.attribute(AId::Y).unwrap_or(0.0),
                z: child.attribute(AId::Z).unwrap_or(0.0),
                points_at_x: child.attribute(AId::PointsAtX).unwrap_or(0.0),
                points_at_y: child.attribute(AId::PointsAtY).unwrap_or(0.0),
                points_at_z: child.attribute(AId::PointsAtZ).unwrap_or(0.0),
                specular_exponent: tree::PositiveNumber::new(specular_exponent),
                limiting_cone_angle: child.attribute(AId::LimitingConeAngle),
            }))
        }
        _ => None,
    }
}

// A malformed filter primitive usually should produce a transparent image.
// But since `FilterKind` structs are designed to always be valid,
// we are using `FeFlood` as fallback.
#[inline(never)]
pub fn create_dummy_primitive() -> tree::FilterKind {
    tree::FilterKind::FeFlood(tree::FeFlood {
        color: tree::Color::black(),
        opacity: tree::Opacity::new(0.0),
    })
}

#[inline(never)]
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
                    return if let Some(prev) = primitives.last() {
                        tree::FilterInput::Reference(prev.result.clone())
                    } else {
                        tree::FilterInput::SourceGraphic
                    };
                }
            }

            input
        }
        None => {
            if let Some(prev) = primitives.last() {
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

#[inline(never)]
fn convert_grayscale(mut amount: f64) -> tree::FilterKind {
    amount = amount.min(1.0);
    tree::FilterKind::FeColorMatrix(tree::FeColorMatrix {
        input: tree::FilterInput::SourceGraphic,
        kind: tree::FeColorMatrixKind::Matrix(vec![
            (0.2126 + 0.7874 * (1.0 - amount)),
            (0.7152 - 0.7152 * (1.0 - amount)),
            (0.0722 - 0.0722 * (1.0 - amount)),
            0.0,
            0.0,

            (0.2126 - 0.2126 * (1.0 - amount)),
            (0.7152 + 0.2848 * (1.0 - amount)),
            (0.0722 - 0.0722 * (1.0 - amount)),
            0.0,
            0.0,

            (0.2126 - 0.2126 * (1.0 - amount)),
            (0.7152 - 0.7152 * (1.0 - amount)),
            (0.0722 + 0.9278 * (1.0 - amount)),
            0.0,
            0.0,

            0.0, 0.0, 0.0, 1.0, 0.0,
        ]),
    })
}

#[inline(never)]
fn convert_sepia(mut amount: f64) -> tree::FilterKind {
    amount = amount.min(1.0);
    tree::FilterKind::FeColorMatrix(tree::FeColorMatrix {
        input: tree::FilterInput::SourceGraphic,
        kind: tree::FeColorMatrixKind::Matrix(vec![
            (0.393 + 0.607 * (1.0 - amount)),
            (0.769 - 0.769 * (1.0 - amount)),
            (0.189 - 0.189 * (1.0 - amount)),
            0.0,
            0.0,

            (0.349 - 0.349 * (1.0 - amount)),
            (0.686 + 0.314 * (1.0 - amount)),
            (0.168 - 0.168 * (1.0 - amount)),
            0.0,
            0.0,

            (0.272 - 0.272 * (1.0 - amount)),
            (0.534 - 0.534 * (1.0 - amount)),
            (0.131 + 0.869 * (1.0 - amount)),
            0.0,
            0.0,

            0.0, 0.0, 0.0, 1.0, 0.0,
        ]),
    })
}

#[inline(never)]
fn convert_saturate(amount: f64) -> tree::FilterKind {
    let amount = tree::PositiveNumber::new(amount.max(0.0));
    tree::FilterKind::FeColorMatrix(tree::FeColorMatrix {
        input: tree::FilterInput::SourceGraphic,
        kind: tree::FeColorMatrixKind::Saturate(amount),
    })
}

#[inline(never)]
fn convert_hue_rotate(amount: svgtypes::Angle) -> tree::FilterKind {
    tree::FilterKind::FeColorMatrix(tree::FeColorMatrix {
        input: tree::FilterInput::SourceGraphic,
        kind: tree::FeColorMatrixKind::HueRotate(amount.to_degrees()),
    })
}

#[inline(never)]
fn convert_invert(mut amount: f64) -> tree::FilterKind {
    amount = amount.min(1.0);
    tree::FilterKind::FeComponentTransfer(tree::FeComponentTransfer {
        input: tree::FilterInput::SourceGraphic,
        func_r: tree::TransferFunction::Table(vec![amount, 1.0 - amount]),
        func_g: tree::TransferFunction::Table(vec![amount, 1.0 - amount]),
        func_b: tree::TransferFunction::Table(vec![amount, 1.0 - amount]),
        func_a: tree::TransferFunction::Identity,
    })
}

#[inline(never)]
fn convert_opacity(mut amount: f64) -> tree::FilterKind {
    amount = amount.min(1.0);
    tree::FilterKind::FeComponentTransfer(tree::FeComponentTransfer {
        input: tree::FilterInput::SourceGraphic,
        func_r: tree::TransferFunction::Identity,
        func_g: tree::TransferFunction::Identity,
        func_b: tree::TransferFunction::Identity,
        func_a: tree::TransferFunction::Table(vec![0.0, amount]),
    })
}

#[inline(never)]
fn convert_brightness(amount: f64) -> tree::FilterKind {
    tree::FilterKind::FeComponentTransfer(tree::FeComponentTransfer {
        input: tree::FilterInput::SourceGraphic,
        func_r: tree::TransferFunction::Linear { slope: amount, intercept: 0.0 },
        func_g: tree::TransferFunction::Linear { slope: amount, intercept: 0.0 },
        func_b: tree::TransferFunction::Linear { slope: amount, intercept: 0.0 },
        func_a: tree::TransferFunction::Identity,
    })
}

#[inline(never)]
fn convert_contrast(amount: f64) -> tree::FilterKind {
    tree::FilterKind::FeComponentTransfer(tree::FeComponentTransfer {
        input: tree::FilterInput::SourceGraphic,
        func_r: tree::TransferFunction::Linear { slope: amount, intercept: -(0.5 * amount) + 0.5 },
        func_g: tree::TransferFunction::Linear { slope: amount, intercept: -(0.5 * amount) + 0.5 },
        func_b: tree::TransferFunction::Linear { slope: amount, intercept: -(0.5 * amount) + 0.5 },
        func_a: tree::TransferFunction::Identity,
    })
}

#[inline(never)]
fn convert_blur(
    node: svgtree::Node,
    std_dev: Length,
    state: &State,
) -> tree::FilterKind {
    let std_dev = tree::PositiveNumber::new(
        super::units::convert_length(std_dev, node, AId::Dx, tree::Units::UserSpaceOnUse, state)
    );
    tree::FilterKind::FeGaussianBlur(tree::FeGaussianBlur {
        input: tree::FilterInput::SourceGraphic,
        std_dev_x: std_dev,
        std_dev_y: std_dev,
    })
}

#[inline(never)]
fn convert_drop_shadow(
    node: svgtree::Node,
    color: Option<tree::Color>,
    dx: Length,
    dy: Length,
    std_dev: Length,
    state: &State,
) -> tree::FilterKind {
    let std_dev = tree::PositiveNumber::new(
        super::units::convert_length(std_dev, node, AId::Dx, tree::Units::UserSpaceOnUse, state)
    );

    let color = color.unwrap_or_else(||
        node.find_attribute(AId::Color).unwrap_or_else(tree::Color::black));

    tree::FilterKind::FeDropShadow(tree::FeDropShadow {
        input: tree::FilterInput::SourceGraphic,
        dx: super::units::convert_length(dx, node, AId::Dx, tree::Units::UserSpaceOnUse, state),
        dy: super::units::convert_length(dy, node, AId::Dy, tree::Units::UserSpaceOnUse, state),
        std_dev_x: std_dev,
        std_dev_y: std_dev,
        color,
        opacity: tree::Opacity::default(),
    })
}
