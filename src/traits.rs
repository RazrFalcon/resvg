// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::f64;
use std::fmt::Display;

// external
use svgdom::{
    path,
    AspectRatio,
    Attributes,
    Color,
    Document,
    Length,
    LengthList,
    Node,
    NumberList,
    Points,
    Transform,
    ValueId,
    ViewBox,
};
use usvg::tree;

// self
use short::{
    AId,
    AValue,
    EId,
};
use geom::*;


pub trait GetViewBox {
    fn get_viewbox(&self) -> Option<Rect>;
}

impl GetViewBox for Node {
    fn get_viewbox(&self) -> Option<Rect> {
        self.attributes()
            .get_type::<ViewBox>(AId::ViewBox)
            .map(|vb| Rect::from_xywh(vb.x, vb.y, vb.w, vb.h))
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


pub trait FromValue: Sized {
    fn get(v: &AValue) -> Option<&Self>;
}

macro_rules! impl_from_value {
    ($rtype:ty, $etype:ident) => (
        impl FromValue for $rtype {
            fn get(v: &AValue) -> Option<&Self> {
                if let &AValue::$etype(ref vv) = v { Some(vv) } else { None }
            }
        }
    )
}

impl_from_value!(Color, Color);
impl_from_value!(f64, Number);
impl_from_value!(Length, Length);
impl_from_value!(LengthList, LengthList);
impl_from_value!(NumberList, NumberList);
impl_from_value!(path::Path, Path);
impl_from_value!(String, String);
impl_from_value!(Transform, Transform);
impl_from_value!(ValueId, PredefValue);
impl_from_value!(ViewBox, ViewBox);
impl_from_value!(Points, Points);
impl_from_value!(AspectRatio, AspectRatio);

impl FromValue for AValue {
    fn get(v: &AValue) -> Option<&Self> {
        Some(v)
    }
}


pub trait GetValue {
    fn get_type<T: FromValue>(&self, id: AId) -> Option<&T>;

    fn get_number(&self, id: AId) -> Option<f64> {
        self.get_type(id).map(|v| *v)
    }

    fn get_length(&self, id: AId) -> Option<Length> {
        self.get_type(id).map(|v| *v)
    }

    fn get_transform(&self, id: AId) -> Option<Transform> {
        self.get_type(id).map(|v| *v)
    }

    fn get_number_list(&self, id: AId) -> Option<&NumberList> {
        self.get_type(id)
    }

    fn get_predef(&self, id: AId) -> Option<ValueId> {
        self.get_type(id).map(|v| *v)
    }

    fn get_color(&self, id: AId) -> Option<Color> {
        self.get_type(id).map(|v| *v)
    }

    fn get_path(&self, id: AId) -> Option<&path::Path> {
        self.get_type(id)
    }

    fn get_points(&self, id: AId) -> Option<&Points> {
        self.get_type(id)
    }

    fn get_string(&self, id: AId) -> Option<&String> {
        self.get_type(id)
    }
}

impl GetValue for Attributes {
    fn get_type<T: FromValue>(&self, id: AId) -> Option<&T> {
        match self.get_value(id) {
            Some(av) => {
                FromValue::get(av)
            }
            None => {
                trace!("Type mismatch.");
                None
            }
        }
    }
}


pub trait FindAttribute {
    fn find_attribute<T: FromValue + Display + Clone>(&self, id: AId) -> Option<T>;
    fn find_attribute_with_node<T: FromValue + Display + Clone>(&self, id: AId) -> Option<(Node, T)>;
}

impl FindAttribute for Node {
    fn find_attribute<T: FromValue + Display + Clone>(&self, id: AId) -> Option<T> {
        self.find_attribute_with_node(id).map(|v| v.1)
    }

    fn find_attribute_with_node<T: FromValue + Display + Clone>(&self, id: AId) -> Option<(Node, T)> {
        for n in self.parents_with_self() {
            if n.has_attribute(id) {
                let v = FromValue::get(n.attributes().get_value(id).unwrap()).cloned();
                return match v {
                    Some(v) => Some((n.clone(), v)),
                    None => None,
                };
            }
        }

        None
    }
}


pub trait ConvTransform<T> {
    fn to_native(&self) -> T;
    fn from_native(&T) -> Self;
}


pub trait TransformFromBBox {
    fn from_bbox(bbox: Rect) -> Self;
}

impl TransformFromBBox for tree::Transform {
    fn from_bbox(bbox: Rect) -> Self {
        Self::new(bbox.width(), 0.0, 0.0, bbox.height(), bbox.x(), bbox.y())
    }
}
