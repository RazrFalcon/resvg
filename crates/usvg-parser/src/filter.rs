// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! A collection of SVG filters.

use std::collections::HashSet;
use std::rc::Rc;
use std::str::FromStr;

use rosvgtree::{self, AttributeId as AId, ElementId as EId};
use strict_num::PositiveF64;
use svgtypes::{Length, LengthUnit as Unit};
use usvg_tree::filter::*;
use usvg_tree::{Color, FuzzyZero, Group, Node, NodeKind, NonZeroF64, Opacity, Point, Rect, Units};

use crate::paint_server::{convert_units, resolve_number};
use crate::rosvgtree_ext::{OpacityWrapper, SvgColorExt, SvgNodeExt, SvgNodeExt2};
use crate::{converter, FromValue, OptionLog};

impl<'a, 'input: 'a> FromValue<'a, 'input> for usvg_tree::filter::ColorInterpolation {
    fn parse(_: rosvgtree::Node, _: rosvgtree::AttributeId, value: &str) -> Option<Self> {
        match value {
            "sRGB" => Some(usvg_tree::filter::ColorInterpolation::SRGB),
            "linearRGB" => Some(usvg_tree::filter::ColorInterpolation::LinearRGB),
            _ => None,
        }
    }
}

pub(crate) fn convert(
    node: rosvgtree::Node,
    state: &converter::State,
    cache: &mut converter::Cache,
) -> Result<Vec<Rc<Filter>>, ()> {
    let value = match node.attribute(AId::Filter) {
        Some(v) => v,
        None => return Ok(Vec::new()),
    };

    let mut has_invalid_urls = false;
    let mut filters = Vec::new();

    let create_base_filter_func = |kind,
                                   filters: &mut Vec<Rc<Filter>>,
                                   cache: &mut converter::Cache| {
        // Filter functions, unlike `filter` elements, do not have a filter region.
        // We're currently do not support an unlimited region, so we simply use a fairly large one.
        // This if far from ideal, but good for now.
        // TODO: Should be fixed eventually.
        let rect = match kind {
            Kind::DropShadow(_) | Kind::GaussianBlur(_) => Rect::new(-0.5, -0.5, 2.0, 2.0).unwrap(),
            _ => Rect::new(-0.1, -0.1, 1.2, 1.2).unwrap(),
        };

        filters.push(Rc::new(Filter {
            id: cache.gen_filter_id(),
            units: Units::ObjectBoundingBox,
            primitive_units: Units::UserSpaceOnUse,
            rect,
            primitives: vec![Primitive {
                x: None,
                y: None,
                width: None,
                height: None,
                // Unlike `filter` elements, filter functions use sRGB colors by default.
                color_interpolation: ColorInterpolation::SRGB,
                result: "result".to_string(),
                kind,
            }],
        }));
    };

    for func in svgtypes::FilterValueListParser::from(value) {
        let func = match func {
            Ok(v) => v,
            Err(e) => {
                // Skip the whole attribute list on error.
                log::warn!("Failed to parse a filter value cause {}. Skipping.", e);
                return Ok(Vec::new());
            }
        };

        match func {
            svgtypes::FilterValue::Blur(std_dev) => create_base_filter_func(
                convert_blur_function(node, std_dev, state),
                &mut filters,
                cache,
            ),
            svgtypes::FilterValue::DropShadow {
                color,
                dx,
                dy,
                std_dev,
            } => create_base_filter_func(
                convert_drop_shadow_function(node, color, dx, dy, std_dev, state),
                &mut filters,
                cache,
            ),
            svgtypes::FilterValue::Brightness(amount) => {
                create_base_filter_func(convert_brightness_function(amount), &mut filters, cache)
            }
            svgtypes::FilterValue::Contrast(amount) => {
                create_base_filter_func(convert_contrast_function(amount), &mut filters, cache)
            }
            svgtypes::FilterValue::Grayscale(amount) => {
                create_base_filter_func(convert_grayscale_function(amount), &mut filters, cache)
            }
            svgtypes::FilterValue::HueRotate(angle) => {
                create_base_filter_func(convert_hue_rotate_function(angle), &mut filters, cache)
            }
            svgtypes::FilterValue::Invert(amount) => {
                create_base_filter_func(convert_invert_function(amount), &mut filters, cache)
            }
            svgtypes::FilterValue::Opacity(amount) => {
                create_base_filter_func(convert_opacity_function(amount), &mut filters, cache)
            }
            svgtypes::FilterValue::Sepia(amount) => {
                create_base_filter_func(convert_sepia_function(amount), &mut filters, cache)
            }
            svgtypes::FilterValue::Saturate(amount) => {
                create_base_filter_func(convert_saturate_function(amount), &mut filters, cache)
            }
            svgtypes::FilterValue::Url(url) => {
                if let Some(link) = node.document().element_by_id(url) {
                    if let Ok(res) = convert_url(link, state, cache) {
                        if let Some(f) = res {
                            filters.push(f);
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
    if filters.is_empty() && has_invalid_urls {
        return Err(());
    }

    Ok(filters)
}

fn convert_url(
    node: rosvgtree::Node,
    state: &converter::State,
    cache: &mut converter::Cache,
) -> Result<Option<Rc<Filter>>, ()> {
    if let Some(filter) = cache.filters.get(node.element_id()) {
        return Ok(Some(filter.clone()));
    }

    let units = convert_units(node, AId::FilterUnits, Units::ObjectBoundingBox);
    let primitive_units = convert_units(node, AId::PrimitiveUnits, Units::UserSpaceOnUse);

    let rect = Rect::new(
        resolve_number(
            node,
            AId::X,
            units,
            state,
            Length::new(-10.0, Unit::Percent),
        ),
        resolve_number(
            node,
            AId::Y,
            units,
            state,
            Length::new(-10.0, Unit::Percent),
        ),
        resolve_number(
            node,
            AId::Width,
            units,
            state,
            Length::new(120.0, Unit::Percent),
        ),
        resolve_number(
            node,
            AId::Height,
            units,
            state,
            Length::new(120.0, Unit::Percent),
        ),
    );
    let rect = rect
        .log_none(|| {
            log::warn!(
                "Filter '{}' has an invalid region. Skipped.",
                node.element_id()
            )
        })
        .ok_or(())?;

    let node_with_primitives = match find_filter_with_primitives(node) {
        Some(v) => v,
        None => return Err(()),
    };
    let primitives = collect_children(&node_with_primitives, primitive_units, state, cache);
    if primitives.is_empty() {
        return Err(());
    }

    let filter = Rc::new(Filter {
        id: node.element_id().to_string(),
        units,
        primitive_units,
        rect,
        primitives,
    });

    cache
        .filters
        .insert(node.element_id().to_string(), filter.clone());

    Ok(Some(filter))
}

fn find_filter_with_primitives<'a>(
    node: rosvgtree::Node<'a, 'a>,
) -> Option<rosvgtree::Node<'a, 'a>> {
    for link in node.href_iter() {
        if link.tag_name() != Some(EId::Filter) {
            log::warn!(
                "Filter '{}' cannot reference '{}' via 'xlink:href'.",
                node.element_id(),
                link.tag_name().unwrap()
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
    filter: &rosvgtree::Node,
    units: Units,
    state: &converter::State,
    cache: &mut converter::Cache,
) -> Vec<Primitive> {
    let mut primitives = Vec::new();

    let mut results = FilterResults {
        names: HashSet::new(),
        idx: 1,
    };

    for child in filter.children() {
        let tag_name = match child.tag_name() {
            Some(v) => v,
            None => continue,
        };

        let kind =
            match tag_name {
                EId::FeDropShadow => convert_drop_shadow(child, &primitives),
                EId::FeGaussianBlur => convert_gaussian_blur(child, &primitives),
                EId::FeOffset => convert_offset(child, &primitives),
                EId::FeBlend => convert_blend(child, &primitives),
                EId::FeFlood => convert_flood(child),
                EId::FeComposite => convert_composite(child, &primitives),
                EId::FeMerge => convert_merge(child, &primitives),
                EId::FeTile => convert_tile(child, &primitives),
                EId::FeImage => convert_image(child, state, cache),
                EId::FeComponentTransfer => convert_component_transfer(child, &primitives),
                EId::FeColorMatrix => convert_color_matrix(child, &primitives),
                EId::FeConvolveMatrix => convert_convolve_matrix(child, &primitives)
                    .unwrap_or_else(create_dummy_primitive),
                EId::FeMorphology => convert_morphology(child, &primitives),
                EId::FeDisplacementMap => convert_displacement_map(child, &primitives),
                EId::FeTurbulence => convert_turbulence(child),
                EId::FeDiffuseLighting => convert_diffuse_lighting(child, &primitives)
                    .unwrap_or_else(create_dummy_primitive),
                EId::FeSpecularLighting => convert_specular_lighting(child, &primitives)
                    .unwrap_or_else(create_dummy_primitive),
                tag_name => {
                    log::warn!("'{}' is not a valid filter primitive. Skipped.", tag_name);
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
    fe: rosvgtree::Node,
    kind: Kind,
    units: Units,
    state: &converter::State,
    results: &mut FilterResults,
) -> Primitive {
    Primitive {
        x: fe.try_convert_length(AId::X, units, state),
        y: fe.try_convert_length(AId::Y, units, state),
        // TODO: validate and test
        width: fe.try_convert_length(AId::Width, units, state),
        height: fe.try_convert_length(AId::Height, units, state),
        color_interpolation: fe
            .find_and_parse_attribute(AId::ColorInterpolationFilters)
            .unwrap_or_default(),
        result: gen_result(fe, results),
        kind,
    }
}

// A malformed filter primitive usually should produce a transparent image.
// But since `FilterKind` structs are designed to always be valid,
// we are using `FeFlood` as fallback.
#[inline(never)]
pub(crate) fn create_dummy_primitive() -> Kind {
    Kind::Flood(Flood {
        color: Color::black(),
        opacity: Opacity::ZERO,
    })
}

#[inline(never)]
fn resolve_input(node: rosvgtree::Node, aid: AId, primitives: &[Primitive]) -> Input {
    match node.attribute(aid) {
        Some(s) => {
            let input = parse_in(s);

            // If `in` references an unknown `result` than fallback
            // to previous result or `SourceGraphic`.
            if let Input::Reference(ref name) = input {
                if !primitives.iter().any(|p| p.result == *name) {
                    return if let Some(prev) = primitives.last() {
                        Input::Reference(prev.result.clone())
                    } else {
                        Input::SourceGraphic
                    };
                }
            }

            input
        }
        None => {
            if let Some(prev) = primitives.last() {
                // If `in` is not set and this is not the first primitive
                // than the input is a result of the previous primitive.
                Input::Reference(prev.result.clone())
            } else {
                // If `in` is not set and this is the first primitive
                // than the input is `SourceGraphic`.
                Input::SourceGraphic
            }
        }
    }
}

fn parse_in(s: &str) -> Input {
    match s {
        "SourceGraphic" => Input::SourceGraphic,
        "SourceAlpha" => Input::SourceAlpha,
        "BackgroundImage" => Input::BackgroundImage,
        "BackgroundAlpha" => Input::BackgroundAlpha,
        "FillPaint" => Input::FillPaint,
        "StrokePaint" => Input::StrokePaint,
        _ => Input::Reference(s.to_string()),
    }
}

fn gen_result(node: rosvgtree::Node, results: &mut FilterResults) -> String {
    match node.attribute(AId::Result) {
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

fn convert_blend(fe: rosvgtree::Node, primitives: &[Primitive]) -> Kind {
    let mode = fe.parse_attribute(AId::Mode).unwrap_or_default();
    let input1 = resolve_input(fe, AId::In, primitives);
    let input2 = resolve_input(fe, AId::In2, primitives);
    Kind::Blend(Blend {
        mode,
        input1,
        input2,
    })
}

fn convert_color_matrix(fe: rosvgtree::Node, primitives: &[Primitive]) -> Kind {
    let kind = convert_color_matrix_kind(fe).unwrap_or_default();
    Kind::ColorMatrix(ColorMatrix {
        input: resolve_input(fe, AId::In, primitives),
        kind,
    })
}

fn convert_color_matrix_kind(fe: rosvgtree::Node) -> Option<ColorMatrixKind> {
    match fe.attribute(AId::Type) {
        Some("saturate") => {
            if let Some(list) = fe.parse_attribute::<Vec<f64>>(AId::Values) {
                if !list.is_empty() {
                    let n = crate::f64_bound(0.0, list[0], 1.0);
                    return Some(ColorMatrixKind::Saturate(PositiveF64::new(n).unwrap()));
                } else {
                    return Some(ColorMatrixKind::Saturate(PositiveF64::new(1.0).unwrap()));
                }
            }
        }
        Some("hueRotate") => {
            if let Some(list) = fe.parse_attribute::<Vec<f64>>(AId::Values) {
                if !list.is_empty() {
                    return Some(ColorMatrixKind::HueRotate(list[0]));
                } else {
                    return Some(ColorMatrixKind::HueRotate(0.0));
                }
            }
        }
        Some("luminanceToAlpha") => {
            return Some(ColorMatrixKind::LuminanceToAlpha);
        }
        _ => {
            // Fallback to `matrix`.
            if let Some(list) = fe.parse_attribute::<Vec<f64>>(AId::Values) {
                if list.len() == 20 {
                    return Some(ColorMatrixKind::Matrix(list));
                }
            }
        }
    }

    None
}

fn convert_component_transfer(fe: rosvgtree::Node, primitives: &[Primitive]) -> Kind {
    let mut kind = ComponentTransfer {
        input: resolve_input(fe, AId::In, primitives),
        func_r: TransferFunction::Identity,
        func_g: TransferFunction::Identity,
        func_b: TransferFunction::Identity,
        func_a: TransferFunction::Identity,
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

    Kind::ComponentTransfer(kind)
}

fn convert_transfer_function(node: rosvgtree::Node) -> Option<TransferFunction> {
    match node.attribute(AId::Type)? {
        "identity" => Some(TransferFunction::Identity),
        "table" => match node.parse_attribute::<Vec<f64>>(AId::TableValues) {
            Some(values) => Some(TransferFunction::Table(values)),
            None => Some(TransferFunction::Table(Vec::new())),
        },
        "discrete" => match node.parse_attribute::<Vec<f64>>(AId::TableValues) {
            Some(values) => Some(TransferFunction::Discrete(values)),
            None => Some(TransferFunction::Discrete(Vec::new())),
        },
        "linear" => Some(TransferFunction::Linear {
            slope: node.parse_attribute(AId::Slope).unwrap_or(1.0),
            intercept: node.parse_attribute(AId::Intercept).unwrap_or(0.0),
        }),
        "gamma" => Some(TransferFunction::Gamma {
            amplitude: node.parse_attribute(AId::Amplitude).unwrap_or(1.0),
            exponent: node.parse_attribute(AId::Exponent).unwrap_or(1.0),
            offset: node.parse_attribute(AId::Offset).unwrap_or(0.0),
        }),
        _ => None,
    }
}

fn convert_composite(fe: rosvgtree::Node, primitives: &[Primitive]) -> Kind {
    let operator = match fe.attribute(AId::Operator).unwrap_or("over") {
        "in" => CompositeOperator::In,
        "out" => CompositeOperator::Out,
        "atop" => CompositeOperator::Atop,
        "xor" => CompositeOperator::Xor,
        "arithmetic" => CompositeOperator::Arithmetic {
            k1: fe.parse_attribute::<f64>(AId::K1).unwrap_or(0.0),
            k2: fe.parse_attribute::<f64>(AId::K2).unwrap_or(0.0),
            k3: fe.parse_attribute::<f64>(AId::K3).unwrap_or(0.0),
            k4: fe.parse_attribute::<f64>(AId::K4).unwrap_or(0.0),
        },
        _ => CompositeOperator::Over,
    };

    let input1 = resolve_input(fe, AId::In, primitives);
    let input2 = resolve_input(fe, AId::In2, primitives);

    Kind::Composite(Composite {
        operator,
        input1,
        input2,
    })
}

fn convert_convolve_matrix(fe: rosvgtree::Node, primitives: &[Primitive]) -> Option<Kind> {
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
    if let Some(value) = fe.attribute(AId::Order) {
        let mut s = svgtypes::NumberListParser::from(value);
        let x = s.next().and_then(|a| a.ok()).map(|n| n as i32).unwrap_or(3);
        let y = s.next().and_then(|a| a.ok()).map(|n| n as i32).unwrap_or(x);
        if x > 0 && y > 0 {
            order_x = x as u32;
            order_y = y as u32;
        }
    }

    let mut matrix = Vec::new();
    if let Some(list) = fe.parse_attribute::<Vec<f64>>(AId::KernelMatrix) {
        if list.len() == (order_x * order_y) as usize {
            matrix = list;
        }
    }

    let mut kernel_sum: f64 = matrix.iter().sum();
    // Round up to prevent float precision issues.
    kernel_sum = (kernel_sum * 1_000_000.0).round() / 1_000_000.0;
    if kernel_sum.is_fuzzy_zero() {
        kernel_sum = 1.0;
    }

    let divisor = fe.parse_attribute(AId::Divisor).unwrap_or(kernel_sum);
    if divisor.is_fuzzy_zero() {
        return None;
    }

    let bias = fe.parse_attribute(AId::Bias).unwrap_or(0.0);

    let target_x = parse_target(fe.parse_attribute(AId::TargetX), order_x)?;
    let target_y = parse_target(fe.parse_attribute(AId::TargetY), order_y)?;

    let kernel_matrix = ConvolveMatrixData::new(target_x, target_y, order_x, order_y, matrix)?;

    let edge_mode = match fe.attribute(AId::EdgeMode).unwrap_or("duplicate") {
        "none" => EdgeMode::None,
        "wrap" => EdgeMode::Wrap,
        _ => EdgeMode::Duplicate,
    };

    let preserve_alpha = fe.attribute(AId::PreserveAlpha).unwrap_or("false") == "true";

    Some(Kind::ConvolveMatrix(ConvolveMatrix {
        input: resolve_input(fe, AId::In, primitives),
        matrix: kernel_matrix,
        divisor: NonZeroF64::new(divisor).unwrap(),
        bias,
        edge_mode,
        preserve_alpha,
    }))
}

fn convert_displacement_map(fe: rosvgtree::Node, primitives: &[Primitive]) -> Kind {
    let parse_channel = |aid| match fe.attribute(aid).unwrap_or("A") {
        "R" => ColorChannel::R,
        "G" => ColorChannel::G,
        "B" => ColorChannel::B,
        _ => ColorChannel::A,
    };

    Kind::DisplacementMap(DisplacementMap {
        input1: resolve_input(fe, AId::In, primitives),
        input2: resolve_input(fe, AId::In2, primitives),
        scale: fe.parse_attribute(AId::Scale).unwrap_or(0.0),
        x_channel_selector: parse_channel(AId::XChannelSelector),
        y_channel_selector: parse_channel(AId::YChannelSelector),
    })
}

fn convert_drop_shadow(fe: rosvgtree::Node, primitives: &[Primitive]) -> Kind {
    let (std_dev_x, std_dev_y) = convert_std_dev_attr(fe, "2 2");

    let (color, opacity) = fe
        .parse_attribute(AId::FloodColor)
        .unwrap_or_else(svgtypes::Color::black)
        .split_alpha();

    let flood_opacity = fe
        .parse_attribute::<OpacityWrapper>(AId::FloodOpacity)
        .map(|v| v.0)
        .unwrap_or(Opacity::ONE);

    Kind::DropShadow(DropShadow {
        input: resolve_input(fe, AId::In, primitives),
        dx: fe.parse_attribute(AId::Dx).unwrap_or(2.0),
        dy: fe.parse_attribute(AId::Dy).unwrap_or(2.0),
        std_dev_x,
        std_dev_y,
        color,
        opacity: opacity * flood_opacity,
    })
}

fn convert_flood(fe: rosvgtree::Node) -> Kind {
    let (color, opacity) = fe
        .parse_attribute(AId::FloodColor)
        .unwrap_or_else(svgtypes::Color::black)
        .split_alpha();

    let flood_opacity = fe
        .parse_attribute::<OpacityWrapper>(AId::FloodOpacity)
        .map(|v| v.0)
        .unwrap_or(Opacity::ONE);

    Kind::Flood(Flood {
        color,
        opacity: opacity * flood_opacity,
    })
}

fn convert_gaussian_blur(fe: rosvgtree::Node, primitives: &[Primitive]) -> Kind {
    let (std_dev_x, std_dev_y) = convert_std_dev_attr(fe, "0 0");
    Kind::GaussianBlur(GaussianBlur {
        input: resolve_input(fe, AId::In, primitives),
        std_dev_x,
        std_dev_y,
    })
}

fn convert_std_dev_attr(fe: rosvgtree::Node, default: &str) -> (PositiveF64, PositiveF64) {
    let text = fe.attribute(AId::StdDeviation).unwrap_or(default);
    let mut parser = svgtypes::NumberListParser::from(text);

    let n1 = parser.next().and_then(|n| n.ok());
    let n2 = parser.next().and_then(|n| n.ok());
    // `stdDeviation` must have no more than two values.
    // Otherwise we should fallback to `0 0`.
    let n3 = parser.next().and_then(|n| n.ok());

    let (std_dev_x, std_dev_y) = match (n1, n2, n3) {
        (Some(n1), Some(n2), None) => (n1, n2),
        (Some(n1), None, None) => (n1, n1),
        _ => (0.0, 0.0),
    };

    let std_dev_x = PositiveF64::new(std_dev_x).unwrap_or(PositiveF64::ZERO);
    let std_dev_y = PositiveF64::new(std_dev_y).unwrap_or(PositiveF64::ZERO);

    (std_dev_x, std_dev_y)
}

fn convert_image(
    fe: rosvgtree::Node,
    state: &converter::State,
    cache: &mut converter::Cache,
) -> Kind {
    let aspect = fe
        .parse_attribute(AId::PreserveAspectRatio)
        .unwrap_or_default();
    let rendering_mode = fe
        .find_and_parse_attribute(AId::ImageRendering)
        .unwrap_or(state.opt.image_rendering);

    if let Some(node) = fe.parse_attribute::<rosvgtree::Node>(AId::Href) {
        let mut state = state.clone();
        state.fe_image_link = true;
        let mut root = Node::new(NodeKind::Group(Group::default()));
        crate::converter::convert_element(node, &state, cache, &mut root);
        return if let Some(node) = root.first_child() {
            node.detach(); // drops `root` node
            Kind::Image(Image {
                aspect,
                rendering_mode,
                data: ImageKind::Use(node),
            })
        } else {
            create_dummy_primitive()
        };
    }

    let href = match fe.attribute(AId::Href) {
        Some(s) => s,
        _ => {
            log::warn!("The 'feImage' element lacks the 'xlink:href' attribute. Skipped.");
            return create_dummy_primitive();
        }
    };

    let href = crate::image::get_href_data(href, state.opt);
    let img_data = match href {
        Some(data) => data,
        None => return create_dummy_primitive(),
    };

    Kind::Image(Image {
        aspect,
        rendering_mode,
        data: ImageKind::Image(img_data),
    })
}

fn convert_diffuse_lighting(fe: rosvgtree::Node, primitives: &[Primitive]) -> Option<Kind> {
    let light_source = convert_light_source(fe)?;
    Some(Kind::DiffuseLighting(DiffuseLighting {
        input: resolve_input(fe, AId::In, primitives),
        surface_scale: fe.parse_attribute(AId::SurfaceScale).unwrap_or(1.0),
        diffuse_constant: fe.parse_attribute(AId::DiffuseConstant).unwrap_or(1.0),
        lighting_color: convert_lighting_color(fe),
        light_source,
    }))
}

fn convert_specular_lighting(fe: rosvgtree::Node, primitives: &[Primitive]) -> Option<Kind> {
    let light_source = convert_light_source(fe)?;

    let specular_exponent = fe.parse_attribute(AId::SpecularExponent).unwrap_or(1.0);
    if !(1.0..=128.0).contains(&specular_exponent) {
        // When exponent is out of range, the whole filter primitive should be ignored.
        return None;
    }

    let specular_exponent = crate::f64_bound(1.0, specular_exponent, 128.0);

    Some(Kind::SpecularLighting(SpecularLighting {
        input: resolve_input(fe, AId::In, primitives),
        surface_scale: fe.parse_attribute(AId::SurfaceScale).unwrap_or(1.0),
        specular_constant: fe.parse_attribute(AId::SpecularConstant).unwrap_or(1.0),
        specular_exponent,
        lighting_color: convert_lighting_color(fe),
        light_source,
    }))
}

#[inline(never)]
fn convert_lighting_color(node: rosvgtree::Node) -> Color {
    // Color's alpha doesn't affect lighting-color. Simply skip it.
    match node.attribute(AId::LightingColor) {
        Some("currentColor") => {
            node.find_and_parse_attribute(AId::Color)
                // Yes, a missing `currentColor` resolves to black and not white.
                .unwrap_or(svgtypes::Color::black())
                .split_alpha()
                .0
        }
        Some(value) => {
            if let Ok(c) = svgtypes::Color::from_str(value) {
                c.split_alpha().0
            } else {
                log::warn!("Failed to parse lighting-color value: '{}'.", value);
                Color::white()
            }
        }
        _ => Color::white(),
    }
}

#[inline(never)]
fn convert_light_source(parent: rosvgtree::Node) -> Option<LightSource> {
    let child = parent.children().find(|n| {
        matches!(
            n.tag_name(),
            Some(EId::FeDistantLight) | Some(EId::FePointLight) | Some(EId::FeSpotLight)
        )
    })?;

    match child.tag_name() {
        Some(EId::FeDistantLight) => Some(LightSource::DistantLight(DistantLight {
            azimuth: child.parse_attribute(AId::Azimuth).unwrap_or(0.0),
            elevation: child.parse_attribute(AId::Elevation).unwrap_or(0.0),
        })),
        Some(EId::FePointLight) => Some(LightSource::PointLight(PointLight {
            x: child.parse_attribute(AId::X).unwrap_or(0.0),
            y: child.parse_attribute(AId::Y).unwrap_or(0.0),
            z: child.parse_attribute(AId::Z).unwrap_or(0.0),
        })),
        Some(EId::FeSpotLight) => {
            let specular_exponent = child.parse_attribute(AId::SpecularExponent).unwrap_or(1.0);
            let specular_exponent = PositiveF64::new(specular_exponent)
                .unwrap_or_else(|| PositiveF64::new(1.0).unwrap());

            Some(LightSource::SpotLight(SpotLight {
                x: child.parse_attribute(AId::X).unwrap_or(0.0),
                y: child.parse_attribute(AId::Y).unwrap_or(0.0),
                z: child.parse_attribute(AId::Z).unwrap_or(0.0),
                points_at_x: child.parse_attribute(AId::PointsAtX).unwrap_or(0.0),
                points_at_y: child.parse_attribute(AId::PointsAtY).unwrap_or(0.0),
                points_at_z: child.parse_attribute(AId::PointsAtZ).unwrap_or(0.0),
                specular_exponent,
                limiting_cone_angle: child.parse_attribute(AId::LimitingConeAngle),
            }))
        }
        _ => None,
    }
}

fn convert_merge(fe: rosvgtree::Node, primitives: &[Primitive]) -> Kind {
    let mut inputs = Vec::new();
    for child in fe.children() {
        inputs.push(resolve_input(child, AId::In, primitives));
    }

    Kind::Merge(Merge { inputs })
}

fn convert_morphology(fe: rosvgtree::Node, primitives: &[Primitive]) -> Kind {
    let operator = match fe.attribute(AId::Operator).unwrap_or("erode") {
        "dilate" => MorphologyOperator::Dilate,
        _ => MorphologyOperator::Erode,
    };

    let mut radius_x = PositiveF64::new(1.0).unwrap();
    let mut radius_y = PositiveF64::new(1.0).unwrap();
    if let Some(list) = fe.parse_attribute::<Vec<f64>>(AId::Radius) {
        let mut rx = 0.0;
        let mut ry = 0.0;
        if list.len() == 2 {
            rx = list[0];
            ry = list[1];
        } else if list.len() == 1 {
            rx = list[0];
            ry = list[0]; // The same as `rx`.
        }

        if rx.is_fuzzy_zero() && ry.is_fuzzy_zero() {
            rx = 1.0;
            ry = 1.0;
        }

        // If only one of the values is zero, reset it to 1.0
        // This is not specified in the spec, but this is how Chrome and Safari work.
        if rx.is_fuzzy_zero() && !ry.is_fuzzy_zero() {
            rx = 1.0;
        }
        if !rx.is_fuzzy_zero() && ry.is_fuzzy_zero() {
            ry = 1.0;
        }

        // Both values must be positive.
        if rx.is_sign_positive() && ry.is_sign_positive() {
            radius_x = PositiveF64::new(rx).unwrap();
            radius_y = PositiveF64::new(ry).unwrap();
        }
    }

    Kind::Morphology(Morphology {
        input: resolve_input(fe, AId::In, primitives),
        operator,
        radius_x,
        radius_y,
    })
}

fn convert_offset(fe: rosvgtree::Node, primitives: &[Primitive]) -> Kind {
    Kind::Offset(Offset {
        input: resolve_input(fe, AId::In, primitives),
        dx: fe.parse_attribute::<f64>(AId::Dx).unwrap_or(0.0),
        dy: fe.parse_attribute::<f64>(AId::Dy).unwrap_or(0.0),
    })
}

fn convert_tile(fe: rosvgtree::Node, primitives: &[Primitive]) -> Kind {
    Kind::Tile(Tile {
        input: resolve_input(fe, AId::In, primitives),
    })
}

fn convert_turbulence(fe: rosvgtree::Node) -> Kind {
    let mut base_frequency = Point::new(PositiveF64::ZERO, PositiveF64::ZERO);
    if let Some(list) = fe.parse_attribute::<Vec<f64>>(AId::BaseFrequency) {
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
            base_frequency = Point::new(PositiveF64::new(x).unwrap(), PositiveF64::new(y).unwrap());
        }
    }

    let mut num_octaves = fe.parse_attribute(AId::NumOctaves).unwrap_or(1.0);
    if num_octaves.is_sign_negative() {
        num_octaves = 0.0;
    }

    let kind = match fe.attribute(AId::Type).unwrap_or("turbulence") {
        "fractalNoise" => TurbulenceKind::FractalNoise,
        _ => TurbulenceKind::Turbulence,
    };

    Kind::Turbulence(Turbulence {
        base_frequency,
        num_octaves: num_octaves.round() as u32,
        seed: fe.parse_attribute(AId::Seed).unwrap_or(0.0).trunc() as i32,
        stitch_tiles: fe.attribute(AId::StitchTiles) == Some("stitch"),
        kind,
    })
}

#[inline(never)]
fn convert_grayscale_function(mut amount: f64) -> Kind {
    amount = amount.min(1.0);
    Kind::ColorMatrix(ColorMatrix {
        input: Input::SourceGraphic,
        kind: ColorMatrixKind::Matrix(vec![
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
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
        ]),
    })
}

#[inline(never)]
fn convert_sepia_function(mut amount: f64) -> Kind {
    amount = amount.min(1.0);
    Kind::ColorMatrix(ColorMatrix {
        input: Input::SourceGraphic,
        kind: ColorMatrixKind::Matrix(vec![
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
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
        ]),
    })
}

#[inline(never)]
fn convert_saturate_function(amount: f64) -> Kind {
    let amount = PositiveF64::new(amount).unwrap_or(PositiveF64::ZERO);
    Kind::ColorMatrix(ColorMatrix {
        input: Input::SourceGraphic,
        kind: ColorMatrixKind::Saturate(amount),
    })
}

#[inline(never)]
fn convert_hue_rotate_function(amount: svgtypes::Angle) -> Kind {
    Kind::ColorMatrix(ColorMatrix {
        input: Input::SourceGraphic,
        kind: ColorMatrixKind::HueRotate(amount.to_degrees()),
    })
}

#[inline(never)]
fn convert_invert_function(mut amount: f64) -> Kind {
    amount = amount.min(1.0);
    Kind::ComponentTransfer(ComponentTransfer {
        input: Input::SourceGraphic,
        func_r: TransferFunction::Table(vec![amount, 1.0 - amount]),
        func_g: TransferFunction::Table(vec![amount, 1.0 - amount]),
        func_b: TransferFunction::Table(vec![amount, 1.0 - amount]),
        func_a: TransferFunction::Identity,
    })
}

#[inline(never)]
fn convert_opacity_function(mut amount: f64) -> Kind {
    amount = amount.min(1.0);
    Kind::ComponentTransfer(ComponentTransfer {
        input: Input::SourceGraphic,
        func_r: TransferFunction::Identity,
        func_g: TransferFunction::Identity,
        func_b: TransferFunction::Identity,
        func_a: TransferFunction::Table(vec![0.0, amount]),
    })
}

#[inline(never)]
fn convert_brightness_function(amount: f64) -> Kind {
    Kind::ComponentTransfer(ComponentTransfer {
        input: Input::SourceGraphic,
        func_r: TransferFunction::Linear {
            slope: amount,
            intercept: 0.0,
        },
        func_g: TransferFunction::Linear {
            slope: amount,
            intercept: 0.0,
        },
        func_b: TransferFunction::Linear {
            slope: amount,
            intercept: 0.0,
        },
        func_a: TransferFunction::Identity,
    })
}

#[inline(never)]
fn convert_contrast_function(amount: f64) -> Kind {
    Kind::ComponentTransfer(ComponentTransfer {
        input: Input::SourceGraphic,
        func_r: TransferFunction::Linear {
            slope: amount,
            intercept: -(0.5 * amount) + 0.5,
        },
        func_g: TransferFunction::Linear {
            slope: amount,
            intercept: -(0.5 * amount) + 0.5,
        },
        func_b: TransferFunction::Linear {
            slope: amount,
            intercept: -(0.5 * amount) + 0.5,
        },
        func_a: TransferFunction::Identity,
    })
}

#[inline(never)]
fn convert_blur_function(node: rosvgtree::Node, std_dev: Length, state: &converter::State) -> Kind {
    let std_dev = PositiveF64::new(crate::units::convert_length(
        std_dev,
        node,
        AId::Dx,
        Units::UserSpaceOnUse,
        state,
    ))
    .unwrap_or(PositiveF64::ZERO);
    Kind::GaussianBlur(GaussianBlur {
        input: Input::SourceGraphic,
        std_dev_x: std_dev,
        std_dev_y: std_dev,
    })
}

#[inline(never)]
fn convert_drop_shadow_function(
    node: rosvgtree::Node,
    color: Option<svgtypes::Color>,
    dx: Length,
    dy: Length,
    std_dev: Length,
    state: &converter::State,
) -> Kind {
    let std_dev = PositiveF64::new(crate::units::convert_length(
        std_dev,
        node,
        AId::Dx,
        Units::UserSpaceOnUse,
        state,
    ))
    .unwrap_or(PositiveF64::ZERO);

    let (color, opacity) = color
        .unwrap_or_else(|| {
            node.find_and_parse_attribute(AId::Color)
                .unwrap_or_else(svgtypes::Color::black)
        })
        .split_alpha();

    Kind::DropShadow(DropShadow {
        input: Input::SourceGraphic,
        dx: crate::units::convert_length(dx, node, AId::Dx, Units::UserSpaceOnUse, state),
        dy: crate::units::convert_length(dy, node, AId::Dy, Units::UserSpaceOnUse, state),
        std_dev_x: std_dev,
        std_dev_y: std_dev,
        color,
        opacity,
    })
}
