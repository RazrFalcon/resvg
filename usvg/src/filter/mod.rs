// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::collections::HashSet;

use svgtypes::{Length, LengthUnit as Unit};

use crate::{converter, Color, NodeKind, Opacity, Tree, Units, Rect};
use crate::paint_server::{resolve_number, convert_units};
use crate::svgtree::{self, EId, AId};

mod funcs;
mod blend;
mod color_matrix;
mod component_transfer;
mod composite;
mod convolve_matrix;
mod displacement_map;
mod drop_shadow;
mod flood;
mod gaussian_blur;
mod image;
mod lighting;
mod merge;
mod morphology;
mod offset;
mod tile;
mod turbulence;

pub use self::blend::*;
pub use self::color_matrix::*;
pub use self::component_transfer::*;
pub use self::composite::*;
pub use self::convolve_matrix::*;
pub use self::displacement_map::*;
pub use self::drop_shadow::*;
pub use self::flood::*;
pub use self::gaussian_blur::*;
pub use self::image::*;
pub use self::lighting::*;
pub use self::merge::*;
pub use self::morphology::*;
pub use self::offset::*;
pub use self::tile::*;
pub use self::turbulence::*;

/// A filter element.
///
/// `filter` element in the SVG.
#[derive(Clone, Debug)]
pub struct Filter {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Can't be empty.
    pub id: String,

    /// Region coordinate system units.
    ///
    /// `filterUnits` in the SVG.
    pub units: Units,

    /// Content coordinate system units.
    ///
    /// `primitiveUnits` in the SVG.
    pub primitive_units: Units,

    /// Filter region.
    ///
    /// `x`, `y`, `width` and `height` in the SVG.
    pub rect: Rect,

    // TODO: rename to `primitives`
    /// A list of filter primitives.
    pub children: Vec<FilterPrimitive>,
}


/// A filter primitive element.
#[derive(Clone, Debug)]
pub struct FilterPrimitive {
    /// `x` coordinate of the filter subregion.
    pub x: Option<f64>,

    /// `y` coordinate of the filter subregion.
    pub y: Option<f64>,

    /// The filter subregion width.
    pub width: Option<f64>,

    /// The filter subregion height.
    pub height: Option<f64>,

    /// Color interpolation mode.
    ///
    /// `color-interpolation-filters` in the SVG.
    pub color_interpolation: ColorInterpolation,

    /// Assigned name for this filter primitive.
    ///
    /// `result` in the SVG.
    pub result: String,

    /// Filter primitive kind.
    pub kind: FilterKind,
}


/// A filter kind.
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub enum FilterKind {
    FeBlend(FeBlend),
    FeColorMatrix(FeColorMatrix),
    FeComponentTransfer(FeComponentTransfer),
    FeComposite(FeComposite),
    FeConvolveMatrix(FeConvolveMatrix),
    FeDiffuseLighting(FeDiffuseLighting),
    FeDisplacementMap(FeDisplacementMap),
    FeDropShadow(FeDropShadow),
    FeFlood(FeFlood),
    FeGaussianBlur(FeGaussianBlur),
    FeImage(FeImage),
    FeMerge(FeMerge),
    FeMorphology(FeMorphology),
    FeOffset(FeOffset),
    FeSpecularLighting(FeSpecularLighting),
    FeTile(FeTile),
    FeTurbulence(FeTurbulence),
}

impl FilterKind {
    /// Checks that `FilterKind` has a specific input.
    pub fn has_input(&self, input: &FilterInput) -> bool {
        match self {
            FilterKind::FeBlend(ref fe) => fe.input1 == *input || fe.input2 == *input,
            FilterKind::FeColorMatrix(ref fe) => fe.input == *input,
            FilterKind::FeComponentTransfer(ref fe) => fe.input == *input,
            FilterKind::FeComposite(ref fe) => fe.input1 == *input || fe.input2 == *input,
            FilterKind::FeConvolveMatrix(ref fe) => fe.input == *input,
            FilterKind::FeDiffuseLighting(ref fe) => fe.input == *input,
            FilterKind::FeDisplacementMap(ref fe) => fe.input1 == *input || fe.input2 == *input,
            FilterKind::FeDropShadow(ref fe) => fe.input == *input,
            FilterKind::FeFlood(_) => false,
            FilterKind::FeGaussianBlur(ref fe) => fe.input == *input,
            FilterKind::FeImage(_) => false,
            FilterKind::FeMerge(ref fe) => fe.inputs.iter().any(|i| i == input),
            FilterKind::FeMorphology(ref fe) => fe.input == *input,
            FilterKind::FeOffset(ref fe) => fe.input == *input,
            FilterKind::FeSpecularLighting(ref fe) => fe.input == *input,
            FilterKind::FeTile(ref fe) => fe.input == *input,
            FilterKind::FeTurbulence(_) => false,
        }
    }
}


/// Identifies input for a filter primitive.
#[allow(missing_docs)]
#[derive(Clone, PartialEq, Debug)]
pub enum FilterInput {
    SourceGraphic,
    SourceAlpha,
    BackgroundImage,
    BackgroundAlpha,
    FillPaint,
    StrokePaint,
    Reference(String),
}


/// A color interpolation mode.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ColorInterpolation {
    SRGB,
    LinearRGB,
}

impl_enum_default!(ColorInterpolation, LinearRGB);

impl_enum_from_str!(ColorInterpolation,
    "sRGB"      => ColorInterpolation::SRGB,
    "linearRGB" => ColorInterpolation::LinearRGB
);


pub(crate) fn convert(
    node: svgtree::Node,
    state: &converter::State,
    id_generator: &mut converter::NodeIdGenerator,
    tree: &mut Tree,
) -> Result<Vec<String>, ()> {
    let value = match node.attribute::<&str>(AId::Filter) {
        Some(v) => v,
        None => return Ok(Vec::new()),
    };

    let mut has_invalid_urls = false;
    let mut ids = Vec::new();

    let mut create_base_filter_func = |kind, ids: &mut Vec<String>, tree: &mut Tree| {
        let id = id_generator.gen_filter_id();
        ids.push(id.clone());

        // Filter functions, unlike `filter` elements, do not have a filter region.
        // We're currently do not support an unlimited region, so we simply use a fairly large one.
        // This if far from ideal, but good for now.
        // TODO: Should be fixed eventually.
        let rect = match kind {
            FilterKind::FeDropShadow(_) | FilterKind::FeGaussianBlur(_) => {
                Rect::new(-1.0, -1.0, 2.0, 2.0).unwrap()
            }
            _ => Rect::new(-0.1, -0.1, 1.2, 1.2).unwrap(),
        };

        tree.append_to_defs(NodeKind::Filter(Filter {
            id,
            units: Units::ObjectBoundingBox,
            primitive_units: Units::UserSpaceOnUse,
            rect,
            children: vec![
                FilterPrimitive {
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                    // Unlike `filter` elements, filter functions use sRGB colors by default.
                    color_interpolation: ColorInterpolation::SRGB,
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
                create_base_filter_func(funcs::convert_blur(node, std_dev, state), &mut ids, tree)
            }
            svgtypes::FilterValue::DropShadow { color, dx, dy, std_dev } => {
                create_base_filter_func(
                    funcs::convert_drop_shadow(node, color, dx, dy, std_dev, state),
                    &mut ids,
                    tree,
                )
            }
            svgtypes::FilterValue::Brightness(amount) => {
                create_base_filter_func(funcs::convert_brightness(amount), &mut ids, tree)
            }
            svgtypes::FilterValue::Contrast(amount) => {
                create_base_filter_func(funcs::convert_contrast(amount), &mut ids, tree)
            }
            svgtypes::FilterValue::Grayscale(amount) => {
                create_base_filter_func(funcs::convert_grayscale(amount), &mut ids, tree)
            }
            svgtypes::FilterValue::HueRotate(angle) => {
                create_base_filter_func(funcs::convert_hue_rotate(angle), &mut ids, tree)
            }
            svgtypes::FilterValue::Invert(amount) => {
                create_base_filter_func(funcs::convert_invert(amount), &mut ids, tree)
            }
            svgtypes::FilterValue::Opacity(amount) => {
                create_base_filter_func(funcs::convert_opacity(amount), &mut ids, tree)
            }
            svgtypes::FilterValue::Sepia(amount) => {
                create_base_filter_func(funcs::convert_sepia(amount), &mut ids, tree)
            }
            svgtypes::FilterValue::Saturate(amount) => {
                create_base_filter_func(funcs::convert_saturate(amount), &mut ids, tree)
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
    state: &converter::State,
    tree: &mut Tree,
) -> Result<Option<String>, ()> {
    if tree.defs_by_id(node.element_id()).is_some() {
        return Ok(Some(node.element_id().to_string()));
    }

    let units = convert_units(node, AId::FilterUnits, Units::ObjectBoundingBox);
    let primitive_units = convert_units(node, AId::PrimitiveUnits, Units::UserSpaceOnUse);

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
        NodeKind::Filter(Filter {
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
            log::warn!(
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
    units: Units,
    state: &converter::State,
) -> Vec<FilterPrimitive> {
    let mut primitives = Vec::new();

    let mut results = FilterResults {
        names: HashSet::new(),
        idx: 1,
    };

    for child in filter.children() {
        let kind = match try_opt_continue!(child.tag_name()) {
            EId::FeDropShadow => drop_shadow::convert(child, &primitives, &state),
            EId::FeGaussianBlur => gaussian_blur::convert(child, &primitives),
            EId::FeOffset => offset::convert(child, &primitives, state),
            EId::FeBlend => blend::convert(child, &primitives),
            EId::FeFlood => flood::convert(child),
            EId::FeComposite => composite::convert(child, &primitives),
            EId::FeMerge => merge::convert(child, &primitives),
            EId::FeTile => tile::convert(child, &primitives),
            EId::FeImage => image::convert(child, state),
            EId::FeComponentTransfer => component_transfer::convert(child, &primitives),
            EId::FeColorMatrix => color_matrix::convert(child, &primitives),
            EId::FeConvolveMatrix => convolve_matrix::convert(child, &primitives),
            EId::FeMorphology => morphology::convert(child, &primitives),
            EId::FeDisplacementMap => displacement_map::convert(child, &primitives),
            EId::FeTurbulence => turbulence::convert(child),
            EId::FeDiffuseLighting => lighting::convert_diffuse(child, &primitives),
            EId::FeSpecularLighting => lighting::convert_specular(child, &primitives),
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
    fe: svgtree::Node,
    kind: FilterKind,
    units: Units,
    state: &converter::State,
    results: &mut FilterResults,
) -> FilterPrimitive {
    FilterPrimitive {
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

// A malformed filter primitive usually should produce a transparent image.
// But since `FilterKind` structs are designed to always be valid,
// we are using `FeFlood` as fallback.
#[inline(never)]
pub(crate) fn create_dummy_primitive() -> FilterKind {
    FilterKind::FeFlood(FeFlood {
        color: Color::black(),
        opacity: Opacity::new(0.0),
    })
}

#[inline(never)]
fn resolve_input(
    node: svgtree::Node,
    aid: AId,
    primitives: &[FilterPrimitive],
) -> FilterInput {
    match node.attribute(aid) {
        Some(s) => {
            let input = parse_in(s);

            // If `in` references an unknown `result` than fallback
            // to previous result or `SourceGraphic`.
            if let FilterInput::Reference(ref name) = input {
                if !primitives.iter().any(|p| p.result == *name) {
                    return if let Some(prev) = primitives.last() {
                        FilterInput::Reference(prev.result.clone())
                    } else {
                        FilterInput::SourceGraphic
                    };
                }
            }

            input
        }
        None => {
            if let Some(prev) = primitives.last() {
                // If `in` is not set and this is not the first primitive
                // than the input is a result of the previous primitive.
                FilterInput::Reference(prev.result.clone())
            } else {
                // If `in` is not set and this is the first primitive
                // than the input is `SourceGraphic`.
                FilterInput::SourceGraphic
            }
        }
    }
}

fn parse_in(
    s: &str,
) -> FilterInput {
    match s {
        "SourceGraphic"     => FilterInput::SourceGraphic,
        "SourceAlpha"       => FilterInput::SourceAlpha,
        "BackgroundImage"   => FilterInput::BackgroundImage,
        "BackgroundAlpha"   => FilterInput::BackgroundAlpha,
        "FillPaint"         => FilterInput::FillPaint,
        "StrokePaint"       => FilterInput::StrokePaint,
        _                   => FilterInput::Reference(s.to_string())
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
