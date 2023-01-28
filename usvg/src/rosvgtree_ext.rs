use std::str::FromStr;

use rosvgtree::{svgtypes, AttributeId, FromValue, Node};
use strict_num::NonZeroPositiveF64;

use crate::{converter, units, EnableBackground, FuzzyEq, Opacity, Rect, Transform, Units};

// We cannot implement `FromValue` directly to a foreign type,
// therefore we have to use this ugly wrapper.
pub struct OpacityWrapper(pub Opacity);

// TODO: to svgtypes?
impl<'a, 'input: 'a> FromValue<'a, 'input> for OpacityWrapper {
    fn parse(_: Node, _: AttributeId, value: &str) -> Option<Self> {
        let length = svgtypes::Length::from_str(value).ok()?;
        if length.unit == svgtypes::LengthUnit::Percent {
            Some(OpacityWrapper(Opacity::new_clamped(length.number / 100.0)))
        } else if length.unit == svgtypes::LengthUnit::None {
            Some(OpacityWrapper(Opacity::new_clamped(length.number)))
        } else {
            None
        }
    }
}

impl<'a, 'input: 'a> FromValue<'a, 'input> for Transform {
    fn parse(_: Node, aid: AttributeId, value: &str) -> Option<Self> {
        let ts = match svgtypes::Transform::from_str(value) {
            Ok(v) => v,
            Err(_) => {
                log::warn!("Failed to parse {} value: '{}'.", aid, value);
                return None;
            }
        };

        let ts = crate::Transform::from(ts);

        let (sx, sy) = ts.get_scale();
        if sx.fuzzy_eq(&0.0) || sy.fuzzy_eq(&0.0) {
            Some(crate::Transform::default())
        } else {
            Some(ts)
        }
    }
}

impl<'a, 'input: 'a> FromValue<'a, 'input> for EnableBackground {
    fn parse(_: Node, _: AttributeId, value: &str) -> Option<Self> {
        let eb = svgtypes::EnableBackground::from_str(value).ok()?;
        match eb {
            svgtypes::EnableBackground::Accumulate => None,
            svgtypes::EnableBackground::New => Some(EnableBackground(None)),
            svgtypes::EnableBackground::NewWithRegion {
                x,
                y,
                width,
                height,
            } => {
                let r = Rect::new(x, y, width, height)?;
                Some(EnableBackground(Some(r)))
            }
        }
    }
}

pub trait SvgNodeExt {
    fn has_valid_transform(&self, aid: AttributeId) -> bool;
    fn parse_viewbox(&self) -> Option<Rect>;
    fn resolve_length(&self, aid: AttributeId, state: &converter::State, def: f64) -> f64;
    fn resolve_valid_length(
        &self,
        aid: AttributeId,
        state: &converter::State,
        def: f64,
    ) -> Option<NonZeroPositiveF64>;
    fn convert_length(
        &self,
        aid: AttributeId,
        object_units: Units,
        state: &converter::State,
        def: svgtypes::Length,
    ) -> f64;
    fn try_convert_length(
        &self,
        aid: AttributeId,
        object_units: Units,
        state: &converter::State,
    ) -> Option<f64>;
    fn convert_user_length(
        &self,
        aid: AttributeId,
        state: &converter::State,
        def: svgtypes::Length,
    ) -> f64;
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
            Err(_) => return false,
        };

        let ts = crate::Transform::from(ts);
        let (sx, sy) = ts.get_scale();
        if sx.fuzzy_eq(&0.0) || sy.fuzzy_eq(&0.0) {
            return false;
        }

        true
    }

    fn parse_viewbox(&self) -> Option<Rect> {
        let vb: svgtypes::ViewBox = self.attribute(AttributeId::ViewBox)?;
        Rect::new(vb.x, vb.y, vb.w, vb.h)
    }

    fn resolve_length(&self, aid: AttributeId, state: &converter::State, def: f64) -> f64 {
        debug_assert!(
            !matches!(aid, AttributeId::BaselineShift | AttributeId::FontSize),
            "{} cannot be resolved via this function",
            aid
        );

        if let Some(n) = self.ancestors().find(|n| n.has_attribute(aid)) {
            if let Some(length) = n.attribute(aid) {
                return units::convert_length(length, n, aid, Units::UserSpaceOnUse, state);
            }
        }

        def
    }

    fn resolve_valid_length(
        &self,
        aid: AttributeId,
        state: &converter::State,
        def: f64,
    ) -> Option<NonZeroPositiveF64> {
        let n = self.resolve_length(aid, state, def);
        NonZeroPositiveF64::new(n)
    }

    fn convert_length(
        &self,
        aid: AttributeId,
        object_units: Units,
        state: &converter::State,
        def: svgtypes::Length,
    ) -> f64 {
        units::convert_length(
            self.attribute(aid).unwrap_or(def),
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
    ) -> Option<f64> {
        Some(units::convert_length(
            self.attribute(aid)?,
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
    ) -> f64 {
        self.convert_length(aid, Units::UserSpaceOnUse, state, def)
    }

    fn is_visible_element(&self, opt: &crate::Options) -> bool {
        self.attribute(AttributeId::Display) != Some("none")
            && self.has_valid_transform(AttributeId::Transform)
            && crate::switch::is_condition_passed(*self, opt)
    }
}
