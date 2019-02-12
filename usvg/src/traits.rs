// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::f64;
use std::fmt::Display;

// external
use svgdom::{
    AspectRatio,
    Attributes,
    Color,
    Document,
    Length,
    LengthList,
    Node,
    NumberList,
    Path,
    Points,
    Transform,
    ViewBox,
};

// self
use short::{
    AId,
    AValue,
    EId,
};
use geom::*;
use utils;


pub trait GetViewBox {
    fn get_viewbox(&self) -> Option<Rect>;
    fn get_viewbox_transform(&self) -> Option<Transform>;
}

impl GetViewBox for Node {
    fn get_viewbox(&self) -> Option<Rect> {
        self.attributes()
            .get_type::<ViewBox>(AId::ViewBox)
            .map(|vb| (vb.x, vb.y, vb.w, vb.h).into())
    }

    fn get_viewbox_transform(&self) -> Option<Transform> {
        let size = {
            let attrs = self.attributes();
            let w = try_opt!(attrs.get_number(AId::Width), None);
            let h = try_opt!(attrs.get_number(AId::Height), None);
            Size::new(w, h)
        };

        let vb = try_opt!(self.get_viewbox(), None);
        let aspect = match self.attributes().get_value(AId::PreserveAspectRatio) {
            Some(&AValue::AspectRatio(aspect)) => aspect,
            _ => AspectRatio::default(),
        };

        Some(utils::view_box_to_transform(vb, aspect, size))
    }
}


pub trait GetDefsNode {
    fn defs_element(&self) -> Option<Node>;
}

impl GetDefsNode for Document {
    fn defs_element(&self) -> Option<Node> {
        let svg = match self.svg_element() {
            Some(svg) => svg.clone(),
            None => return None,
        };

        match svg.first_child() {
            Some(child) => {
                if child.is_tag_name(EId::Defs) {
                    Some(child.clone())
                } else {
                    warn!("The first child of the 'svg' element should be 'defs'. Found '{:?}' instead.",
                          child.tag_name());
                    None
                }
            }
            None => {
                None
            }
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

impl_from_value!(Color, Color);
impl_from_value!(f64, Number);
impl_from_value!(Length, Length);
impl_from_value!(LengthList, LengthList);
impl_from_value!(NumberList, NumberList);
impl_from_value!(Path, Path);
impl_from_value!(Transform, Transform);
impl_from_value!(ViewBox, ViewBox);
impl_from_value!(Points, Points);
impl_from_value!(AspectRatio, AspectRatio);

impl FromValue for str {
    fn get(v: &AValue) -> Option<&Self> {
        match v {
            &AValue::String(ref s) => Some(s.as_str()),
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

    fn get_number(&self, id: AId) -> Option<f64> {
        self.get_type(id).cloned()
    }

    fn get_number_or(&self, id: AId, def: f64) -> f64 {
        self.get_number(id).unwrap_or(def)
    }

    fn get_length(&self, id: AId) -> Option<Length> {
        self.get_type(id).cloned()
    }

    fn get_transform(&self, id: AId) -> Option<Transform> {
        self.get_type(id).cloned()
    }

    fn get_number_list(&self, id: AId) -> Option<&NumberList> {
        self.get_type(id)
    }

    fn get_color(&self, id: AId) -> Option<Color> {
        self.get_type(id).cloned()
    }

    fn get_path(&self, id: AId) -> Option<&Path> {
        self.get_type(id)
    }

    fn get_points(&self, id: AId) -> Option<&Points> {
        self.get_type(id)
    }

    fn get_str(&self, id: AId) -> Option<&str> {
        self.get_type(id)
    }

    fn get_str_or<'a>(&'a self, id: AId, def: &'a str) -> &'a str {
        self.get_str(id).unwrap_or(def)
    }
}

impl GetValue for Attributes {
    fn get_type<T: FromValue + ?Sized>(&self, id: AId) -> Option<&T> {
        self.get_value(id).and_then(|av| FromValue::get(av))
    }
}


pub trait FindAttribute {
    fn find_attribute<T: FromValue + Display + Clone>(&self, id: AId) -> Option<T>;
    fn find_node_with_attribute(&self, id: AId) -> Option<Node>;
}

impl FindAttribute for Node {
    fn find_attribute<T: FromValue + Display + Clone>(&self, id: AId) -> Option<T> {
        for n in self.ancestors() {
            if n.has_attribute(id) {
                return FromValue::get(n.attributes().get_value(id).unwrap()).cloned();
            }
        }

        None
    }

    fn find_node_with_attribute(&self, id: AId) -> Option<Node> {
        for n in self.ancestors() {
            if n.has_attribute(id) {
                return Some(n.clone())
            }
        }

        None
    }
}


pub trait AppendTransform {
    fn append_transform(&mut self, ts: Transform);
    fn prepend_transform(&mut self, ts: Transform);
}

impl AppendTransform for Node {
    fn append_transform(&mut self, ts: Transform) {
        let mut curr_ts = self.attributes().get_transform(AId::Transform).unwrap_or_default();
        curr_ts.append(&ts);
        self.set_attribute((AId::Transform, curr_ts));
    }

    fn prepend_transform(&mut self, ts: Transform) {
        let mut ts = ts.clone();
        let curr_ts = self.attributes().get_transform(AId::Transform).unwrap_or_default();
        ts.append(&curr_ts);
        self.set_attribute((AId::Transform, ts));
    }
}


pub trait AttributeExt {
    fn move_attribute_to(&mut self, aid: AId, to: &mut Self);
    fn copy_attribute_to(&self, aid: AId, to: &mut Self);
}

impl AttributeExt for Node {
    fn move_attribute_to(&mut self, aid: AId, to: &mut Self) {
        self.copy_attribute_to(aid, to);
        self.remove_attribute(aid);
    }

    fn copy_attribute_to(&self, aid: AId, to: &mut Self) {
        match self.attributes().get(aid) {
            Some(attr) => to.set_attribute(attr.clone()),
            None => to.remove_attribute(aid),
        }
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
