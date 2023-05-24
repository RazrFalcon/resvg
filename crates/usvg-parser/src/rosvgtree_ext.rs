// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::str::FromStr;

use rosvgtree::{AttributeId, Node};
use strict_num::NonZeroPositiveF32;
use usvg_tree::{strict_num, NonZeroRect, Opacity, Transform, Units};

use crate::{converter, units};

// TODO: is there a way yo make it less ugly? Too many lifetimes.
/// A trait for parsing attribute values.
pub trait FromValue<'a, 'input: 'a>: Sized {
    /// Parses an attribute value.
    ///
    /// When `None` is returned, the attribute value will be logged as a parsing failure.
    fn parse(node: Node<'a, 'input>, aid: AttributeId, value: &'a str) -> Option<Self>;
}

// We cannot implement `FromValue` directly to a foreign type,
// therefore we have to use this ugly wrapper.
pub struct OpacityWrapper(pub Opacity);

// TODO: to svgtypes?
impl<'a, 'input: 'a> FromValue<'a, 'input> for OpacityWrapper {
    fn parse(_: Node, _: AttributeId, value: &str) -> Option<Self> {
        let length = svgtypes::Length::from_str(value).ok()?;
        if length.unit == svgtypes::LengthUnit::Percent {
            Some(OpacityWrapper(Opacity::new_clamped(
                length.number as f32 / 100.0,
            )))
        } else if length.unit == svgtypes::LengthUnit::None {
            Some(OpacityWrapper(Opacity::new_clamped(length.number as f32)))
        } else {
            None
        }
    }
}

impl<'a, 'input: 'a> FromValue<'a, 'input> for Transform {
    fn parse(_: Node, _: AttributeId, value: &str) -> Option<Self> {
        let ts = match svgtypes::Transform::from_str(value) {
            Ok(v) => v,
            Err(_) => return None,
        };

        let ts = Transform::from_row(
            ts.a as f32,
            ts.b as f32,
            ts.c as f32,
            ts.d as f32,
            ts.e as f32,
            ts.f as f32,
        );

        if ts.is_valid() {
            Some(ts)
        } else {
            Some(Transform::default())
        }
    }
}

impl<'a, 'input: 'a> FromValue<'a, 'input> for usvg_tree::Units {
    fn parse(_: Node, _: AttributeId, value: &str) -> Option<Self> {
        match value {
            "userSpaceOnUse" => Some(usvg_tree::Units::UserSpaceOnUse),
            "objectBoundingBox" => Some(usvg_tree::Units::ObjectBoundingBox),
            _ => None,
        }
    }
}

impl<'a, 'input: 'a> FromValue<'a, 'input> for f32 {
    fn parse(_: Node, _: AttributeId, value: &str) -> Option<Self> {
        svgtypes::Number::from_str(value).ok().map(|v| v.0 as f32)
    }
}

impl<'a, 'input: 'a> FromValue<'a, 'input> for svgtypes::Length {
    fn parse(_: Node, _: AttributeId, value: &str) -> Option<Self> {
        svgtypes::Length::from_str(value).ok()
    }
}

impl<'a, 'input: 'a> FromValue<'a, 'input> for svgtypes::AspectRatio {
    fn parse(_: Node, _: AttributeId, value: &str) -> Option<Self> {
        Self::from_str(value).ok()
    }
}

impl<'a, 'input: 'a> FromValue<'a, 'input> for svgtypes::PaintOrder {
    fn parse(_: Node, _: AttributeId, value: &str) -> Option<Self> {
        Self::from_str(value).ok()
    }
}

impl<'a, 'input: 'a> FromValue<'a, 'input> for svgtypes::Color {
    fn parse(_: Node, _: AttributeId, value: &str) -> Option<Self> {
        Self::from_str(value).ok()
    }
}

impl<'a, 'input: 'a> FromValue<'a, 'input> for svgtypes::Angle {
    fn parse(_: Node, _: AttributeId, value: &str) -> Option<Self> {
        Self::from_str(value).ok()
    }
}

impl<'a, 'input: 'a> FromValue<'a, 'input> for svgtypes::ViewBox {
    fn parse(_: Node, _: AttributeId, value: &str) -> Option<Self> {
        Self::from_str(value).ok()
    }
}

impl<'a, 'input: 'a> FromValue<'a, 'input> for svgtypes::EnableBackground {
    fn parse(_: Node, _: AttributeId, value: &str) -> Option<Self> {
        Self::from_str(value).ok()
    }
}

impl<'a, 'input: 'a> FromValue<'a, 'input> for svgtypes::Paint<'a> {
    fn parse(_: Node, _: AttributeId, value: &'a str) -> Option<Self> {
        Self::from_str(value).ok()
    }
}

impl<'a, 'input: 'a> FromValue<'a, 'input> for Vec<f32> {
    fn parse(_: Node, _: AttributeId, value: &str) -> Option<Self> {
        let mut list = Vec::new();
        for n in svgtypes::NumberListParser::from(value) {
            list.push(n.ok()? as f32);
        }

        Some(list)
    }
}

impl<'a, 'input: 'a> FromValue<'a, 'input> for Vec<svgtypes::Length> {
    fn parse(_: Node, _: AttributeId, value: &str) -> Option<Self> {
        let mut list = Vec::new();
        for n in svgtypes::LengthListParser::from(value) {
            list.push(n.ok()?);
        }

        Some(list)
    }
}

impl<'a, 'input: 'a> FromValue<'a, 'input> for usvg_tree::Visibility {
    fn parse(_: Node, _: AttributeId, value: &str) -> Option<Self> {
        match value {
            "visible" => Some(usvg_tree::Visibility::Visible),
            "hidden" => Some(usvg_tree::Visibility::Hidden),
            "collapse" => Some(usvg_tree::Visibility::Collapse),
            _ => None,
        }
    }
}

impl<'a, 'input: 'a> FromValue<'a, 'input> for usvg_tree::SpreadMethod {
    fn parse(_: Node, _: AttributeId, value: &str) -> Option<Self> {
        match value {
            "pad" => Some(usvg_tree::SpreadMethod::Pad),
            "reflect" => Some(usvg_tree::SpreadMethod::Reflect),
            "repeat" => Some(usvg_tree::SpreadMethod::Repeat),
            _ => None,
        }
    }
}

impl<'a, 'input: 'a> FromValue<'a, 'input> for usvg_tree::ShapeRendering {
    fn parse(_: Node, _: AttributeId, value: &str) -> Option<Self> {
        match value {
            "optimizeSpeed" => Some(usvg_tree::ShapeRendering::OptimizeSpeed),
            "crispEdges" => Some(usvg_tree::ShapeRendering::CrispEdges),
            "geometricPrecision" => Some(usvg_tree::ShapeRendering::GeometricPrecision),
            _ => None,
        }
    }
}

impl<'a, 'input: 'a> FromValue<'a, 'input> for usvg_tree::TextRendering {
    fn parse(_: Node, _: AttributeId, value: &str) -> Option<Self> {
        match value {
            "optimizeSpeed" => Some(usvg_tree::TextRendering::OptimizeSpeed),
            "optimizeLegibility" => Some(usvg_tree::TextRendering::OptimizeLegibility),
            "geometricPrecision" => Some(usvg_tree::TextRendering::GeometricPrecision),
            _ => None,
        }
    }
}

impl<'a, 'input: 'a> FromValue<'a, 'input> for usvg_tree::ImageRendering {
    fn parse(_: Node, _: AttributeId, value: &str) -> Option<Self> {
        match value {
            "optimizeQuality" => Some(usvg_tree::ImageRendering::OptimizeQuality),
            "optimizeSpeed" => Some(usvg_tree::ImageRendering::OptimizeSpeed),
            _ => None,
        }
    }
}

impl<'a, 'input: 'a> FromValue<'a, 'input> for usvg_tree::BlendMode {
    fn parse(_: Node, _: AttributeId, value: &str) -> Option<Self> {
        match value {
            "normal" => Some(usvg_tree::BlendMode::Normal),
            "multiply" => Some(usvg_tree::BlendMode::Multiply),
            "screen" => Some(usvg_tree::BlendMode::Screen),
            "overlay" => Some(usvg_tree::BlendMode::Overlay),
            "darken" => Some(usvg_tree::BlendMode::Darken),
            "lighten" => Some(usvg_tree::BlendMode::Lighten),
            "color-dodge" => Some(usvg_tree::BlendMode::ColorDodge),
            "color-burn" => Some(usvg_tree::BlendMode::ColorBurn),
            "hard-light" => Some(usvg_tree::BlendMode::HardLight),
            "soft-light" => Some(usvg_tree::BlendMode::SoftLight),
            "difference" => Some(usvg_tree::BlendMode::Difference),
            "exclusion" => Some(usvg_tree::BlendMode::Exclusion),
            "hue" => Some(usvg_tree::BlendMode::Hue),
            "saturation" => Some(usvg_tree::BlendMode::Saturation),
            "color" => Some(usvg_tree::BlendMode::Color),
            "luminosity" => Some(usvg_tree::BlendMode::Luminosity),
            _ => None,
        }
    }
}

impl<'a, 'input: 'a> FromValue<'a, 'input> for Node<'a, 'input> {
    fn parse(node: Node<'a, 'input>, aid: AttributeId, value: &str) -> Option<Self> {
        let id = if aid == AttributeId::Href {
            svgtypes::IRI::from_str(value).ok().map(|v| v.0)
        } else {
            svgtypes::FuncIRI::from_str(value).ok().map(|v| v.0)
        }?;

        node.document().element_by_id(id)
    }
}

pub trait SvgNodeExt {
    fn has_valid_transform(&self, aid: AttributeId) -> bool;
    fn parse_viewbox(&self) -> Option<NonZeroRect>;
    fn resolve_length(&self, aid: AttributeId, state: &converter::State, def: f32) -> f32;
    fn resolve_valid_length(
        &self,
        aid: AttributeId,
        state: &converter::State,
        def: f32,
    ) -> Option<NonZeroPositiveF32>;
    fn convert_length(
        &self,
        aid: AttributeId,
        object_units: Units,
        state: &converter::State,
        def: svgtypes::Length,
    ) -> f32;
    fn try_convert_length(
        &self,
        aid: AttributeId,
        object_units: Units,
        state: &converter::State,
    ) -> Option<f32>;
    fn convert_user_length(
        &self,
        aid: AttributeId,
        state: &converter::State,
        def: svgtypes::Length,
    ) -> f32;
    fn is_visible_element(&self, opt: &crate::Options) -> bool;
}

impl SvgNodeExt for Node<'_, '_> {
    fn has_valid_transform(&self, aid: AttributeId) -> bool {
        // Do not use Node::attribute::<Transform>, because it will always
        // return a valid transform.

        let attr = match self.attribute(aid) {
            Some(attr) => attr,
            None => return true,
        };

        let ts = match svgtypes::Transform::from_str(attr) {
            Ok(v) => v,
            Err(_) => return true,
        };

        let ts = Transform::from_row(
            ts.a as f32,
            ts.b as f32,
            ts.c as f32,
            ts.d as f32,
            ts.e as f32,
            ts.f as f32,
        );
        ts.is_valid()
    }

    fn parse_viewbox(&self) -> Option<NonZeroRect> {
        let vb: svgtypes::ViewBox = self.parse_attribute(AttributeId::ViewBox)?;
        NonZeroRect::from_xywh(vb.x as f32, vb.y as f32, vb.w as f32, vb.h as f32)
    }

    fn resolve_length(&self, aid: AttributeId, state: &converter::State, def: f32) -> f32 {
        debug_assert!(
            !matches!(aid, AttributeId::BaselineShift | AttributeId::FontSize),
            "{} cannot be resolved via this function",
            aid
        );

        if let Some(n) = self.ancestors().find(|n| n.has_attribute(aid)) {
            if let Some(length) = n.parse_attribute(aid) {
                return units::convert_length(length, n, aid, Units::UserSpaceOnUse, state);
            }
        }

        def
    }

    fn resolve_valid_length(
        &self,
        aid: AttributeId,
        state: &converter::State,
        def: f32,
    ) -> Option<NonZeroPositiveF32> {
        let n = self.resolve_length(aid, state, def);
        NonZeroPositiveF32::new(n)
    }

    fn convert_length(
        &self,
        aid: AttributeId,
        object_units: Units,
        state: &converter::State,
        def: svgtypes::Length,
    ) -> f32 {
        units::convert_length(
            self.parse_attribute(aid).unwrap_or(def),
            *self,
            aid,
            object_units,
            state,
        )
    }

    fn try_convert_length(
        &self,
        aid: AttributeId,
        object_units: Units,
        state: &converter::State,
    ) -> Option<f32> {
        Some(units::convert_length(
            self.parse_attribute(aid)?,
            *self,
            aid,
            object_units,
            state,
        ))
    }

    fn convert_user_length(
        &self,
        aid: AttributeId,
        state: &converter::State,
        def: svgtypes::Length,
    ) -> f32 {
        self.convert_length(aid, Units::UserSpaceOnUse, state, def)
    }

    fn is_visible_element(&self, opt: &crate::Options) -> bool {
        self.attribute(AttributeId::Display) != Some("none")
            && self.has_valid_transform(AttributeId::Transform)
            && crate::switch::is_condition_passed(*self, opt)
    }
}

pub trait SvgNodeExt2<'a, 'input: 'a> {
    fn parse_attribute<T: FromValue<'a, 'input>>(&self, aid: AttributeId) -> Option<T>;
    fn find_and_parse_attribute<T: FromValue<'a, 'input>>(&self, aid: AttributeId) -> Option<T>;
}

impl<'a, 'input: 'a> SvgNodeExt2<'a, 'input> for Node<'a, 'input> {
    fn parse_attribute<T: FromValue<'a, 'input>>(&self, aid: AttributeId) -> Option<T> {
        let value = self.attribute(aid)?;
        match T::parse(*self, aid, value) {
            Some(v) => Some(v),
            None => {
                // TODO: show position in XML
                log::warn!("Failed to parse {} value: '{}'.", aid, value);
                None
            }
        }
    }

    fn find_and_parse_attribute<T: FromValue<'a, 'input>>(&self, aid: AttributeId) -> Option<T> {
        let node = self.find_attribute(aid)?;
        node.parse_attribute(aid)
    }
}

pub(crate) trait SvgColorExt {
    fn split_alpha(self) -> (usvg_tree::Color, Opacity);
}

impl SvgColorExt for svgtypes::Color {
    fn split_alpha(self) -> (usvg_tree::Color, Opacity) {
        (
            usvg_tree::Color::new_rgb(self.red, self.green, self.blue),
            Opacity::new_u8(self.alpha),
        )
    }
}
