// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::str::FromStr;

// external
use svgdom;

// self
use crate::tree;
use super::prelude::*;
use super::units;


pub trait SvgNodeExt {
    fn find_attribute<T: FromValue + Clone>(&self, aid: AId) -> Option<T>;
    fn find_node_with_attribute(&self, aid: AId) -> Option<svgdom::Node>;
    fn try_find_enum<T: FromStr>(&self, aid: AId) -> Option<T>;
    fn find_enum<T: FromStr + Default>(&self, aid: AId) -> T;
    fn resolve_length(&self, aid: AId, state: &State, def: f64) -> f64;
    fn convert_length(&self, aid: AId, object_units: tree::Units, state: &State, def: Length) -> f64;
    fn try_convert_length(&self, aid: AId, object_units: tree::Units, state: &State) -> Option<f64>;
    fn convert_user_length(&self, aid: AId, state: &State, def: Length) -> f64;
    fn try_convert_user_length(&self, aid: AId, state: &State) -> Option<f64>;
    fn convert_opacity(&self, aid: AId) -> tree::Opacity;
    fn href_iter(&self) -> HrefIter;
    fn move_attribute_to(&mut self, aid: AId, to: &mut Self);
    fn copy_attribute_to(&self, aid: AId, to: &mut Self);
    fn try_set_attribute(&mut self, attr: &svgdom::Attribute);
    fn get_viewbox(&self) -> Option<Rect>;
    fn is_valid_transform(&self, aid: AId) -> bool;
}

impl SvgNodeExt for svgdom::Node {
    fn find_attribute<T: FromValue + Clone>(&self, aid: AId) -> Option<T> {
        for n in self.ancestors() {
            if let Some(v) = n.attributes().get_value(aid) {
                return FromValue::get(v).cloned();
            }
        }

        None
    }

    fn find_node_with_attribute(&self, aid: AId) -> Option<svgdom::Node> {
        for n in self.ancestors() {
            if n.has_attribute(aid) {
                return Some(n.clone())
            }
        }

        None
    }

    fn try_find_enum<T: FromStr>(&self, aid: AId) -> Option<T> {
        for n in self.ancestors() {
            if n.has_attribute(aid) {
                let attrs = n.attributes();
                if let Some(s) = attrs.get_str(aid) {
                    if let Ok(v) = T::from_str(s) {
                        return Some(v);
                    }
                }

                // No reason to go further.
                break;
            }
        }

        None
    }

    fn find_enum<T: FromStr + Default>(&self, aid: AId) -> T {
        self.try_find_enum(aid).unwrap_or_default()
    }

    fn resolve_length(&self, aid: AId, state: &State, def: f64) -> f64 {
        let is_inheritable = match aid {
              AId::BaselineShift
            | AId::FloodOpacity
            | AId::FontSize // sort of
            | AId::Opacity
            | AId::StopOpacity => false,
            _ => true,
        };

        debug_assert!(is_inheritable);

        for n in self.ancestors() {
            match n.attributes().get_value(aid) {
                Some(AValue::Number(num)) => {
                    return *num;
                }
                Some(AValue::Length(length)) => {
                    return units::convert_length(*length, &n, aid, tree::Units::UserSpaceOnUse, state);
                }
                Some(_) => {
                    return def;
                }
                None => {}
            }
        }

        def
    }

    fn convert_length(&self, aid: AId, object_units: tree::Units, state: &State, def: Length) -> f64 {
        let length = self.attributes().get_length_or(aid, def);
        units::convert_length(length, self, aid, object_units, state)
    }

    fn try_convert_length(&self, aid: AId, object_units: tree::Units, state: &State) -> Option<f64> {
        let length = self.attributes().get_length(aid)?;
        Some(units::convert_length(length, self, aid, object_units, state))
    }

    fn convert_user_length(&self, aid: AId, state: &State, def: Length) -> f64 {
        self.convert_length(aid, tree::Units::UserSpaceOnUse, state, def)
    }

    fn try_convert_user_length(&self, aid: AId, state: &State) -> Option<f64> {
        self.try_convert_length(aid, tree::Units::UserSpaceOnUse, state)
    }

    fn convert_opacity(&self, aid: AId) -> tree::Opacity {
        let opacity = match self.attributes().get_value(aid) {
            Some(AValue::Number(n)) => *n,
            _ => 1.0,
        };

        f64_bound(0.0, opacity, 1.0).into()
    }

    fn href_iter(&self) -> HrefIter {
        HrefIter {
            origin: self.clone(),
            curr: self.clone(),
            is_first: true,
            is_finished: false,
        }
    }

    fn move_attribute_to(&mut self, aid: AId, to: &mut Self) {
        self.copy_attribute_to(aid, to);
        self.remove_attribute(aid);
    }

    fn copy_attribute_to(&self, aid: AId, to: &mut Self) {
        match self.attributes().get(aid) {
            Some(attr) => to.try_set_attribute(&attr),
            None => to.remove_attribute(aid),
        }
    }

    fn try_set_attribute(&mut self, attr: &svgdom::Attribute) {
        match self.set_attribute_checked(attr.clone()) {
            Ok(_) => {}
            Err(_) => {
                let id = if self.has_id() { format!("#{}", self.id()) } else { String::new() };
                warn!("Failed to set {} on {}{}.", attr, self.tag_name(), id);
            }
        }
    }

    fn get_viewbox(&self) -> Option<Rect> {
        let vb: svgdom::ViewBox = self.attributes().get_type(AId::ViewBox).cloned()?;
        Rect::new(vb.x, vb.y, vb.w, vb.h)
    }

    fn is_valid_transform(&self, aid: AId) -> bool {
        if let Some(AValue::Transform(ts)) = self.attributes().get_value(aid).cloned() {
            let (sx, sy) = ts.get_scale();
            if sx.fuzzy_eq(&0.0) || sy.fuzzy_eq(&0.0) {
                return false;
            }
        }

        true
    }
}


pub struct HrefIter {
    origin: svgdom::Node,
    curr: svgdom::Node,
    is_first: bool,
    is_finished: bool,
}

impl Iterator for HrefIter {
    type Item = svgdom::Node;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_finished {
            return None;
        }

        if self.is_first {
            self.is_first = false;
            return Some(self.curr.clone());
        }

        let av = self.curr.attributes().get_value(AId::Href).cloned();
        if let Some(AValue::Link(link)) = av {
            if link == self.curr || link == self.origin {
                warn!("Element '#{}' cannot reference itself via 'xlink:href'.", self.origin.id());
                self.is_finished = true;
                return None;
            }

            self.curr = link.clone();
            Some(link.clone())
        } else {
            None
        }
    }
}


pub trait FromValue {
    fn get(v: &AValue) -> Option<&Self>;
}

macro_rules! impl_from_value {
    ($rtype:ty, $etype:ident) => (
        impl FromValue for $rtype {
            fn get(v: &AValue) -> Option<&Self> {
                if let AValue::$etype(ref vv) = *v { Some(vv) } else { None }
            }
        }
    )
}

impl_from_value!(svgdom::Color, Color);
impl_from_value!(Length, Length);
impl_from_value!(svgdom::NumberList, NumberList);
impl_from_value!(svgdom::Transform, Transform);
impl_from_value!(svgdom::ViewBox, ViewBox);
impl_from_value!(svgdom::AspectRatio, AspectRatio);

impl FromValue for str {
    fn get(v: &AValue) -> Option<&Self> {
        match v {
            AValue::String(ref s) => Some(s.as_str()),
            _ => None,
        }
    }
}

impl FromValue for AValue {
    fn get(v: &AValue) -> Option<&Self> {
        Some(v)
    }
}


pub trait GetValue {
    fn get_type<T: FromValue + ?Sized>(&self, id: AId) -> Option<&T>;

    fn get_length(&self, id: AId) -> Option<Length> {
        self.get_type(id).cloned()
    }

    fn get_length_or(&self, id: AId, def: Length) -> Length {
        self.get_length(id).unwrap_or(def)
    }

    fn get_transform(&self, id: AId) -> svgdom::Transform {
        let ts: svgdom::Transform = try_opt!(self.get_type(id).cloned(),
                                             svgdom::Transform::default());

        let (sx, sy) = ts.get_scale();
        if sx.fuzzy_eq(&0.0) || sy.fuzzy_eq(&0.0) {
            svgdom::Transform::default()
        } else {
            ts
        }
    }

    fn get_number_list(&self, id: AId) -> Option<&svgdom::NumberList> {
        self.get_type(id)
    }

    fn get_color(&self, id: AId) -> Option<svgdom::Color> {
        self.get_type(id).cloned()
    }

    fn get_str(&self, id: AId) -> Option<&str> {
        self.get_type(id)
    }

    fn get_str_or<'a>(&'a self, id: AId, def: &'a str) -> &'a str {
        self.get_str(id).unwrap_or(def)
    }
}

impl GetValue for svgdom::Attributes {
    fn get_type<T: FromValue + ?Sized>(&self, id: AId) -> Option<&T> {
        self.get_value(id).and_then(|av| FromValue::get(av))
    }
}


/// Checks that type has a default value.
pub trait IsDefault: Default {
    /// Checks that type has a default value.
    fn is_default(&self) -> bool;
}

impl<T: Default + PartialEq + Copy> IsDefault for T {
    fn is_default(&self) -> bool {
        *self == Self::default()
    }
}


/// Checks that the current number is > 0.
pub trait IsValidLength {
    /// Checks that the current number is > 0.
    fn is_valid_length(&self) -> bool;
}

impl IsValidLength for f64 {
    fn is_valid_length(&self) -> bool {
        *self > 0.0
    }
}
