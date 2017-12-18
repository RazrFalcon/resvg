// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt;
use std::path::PathBuf;

use svgdom::types::{
    Transform,
    Color,
};

use math::{
    Rect,
};

use super::attribute::*;


pub struct Element {
    pub id: String,
    pub data: Type,
    pub transform: Transform,
}

pub enum Type {
    Path(Path),
    Text(Text),
    Image(Image),
    Group(Group),
}

pub struct RefElement {
    pub id: String,
    pub data: RefType,
}

pub enum RefType {
    LinearGradient(LinearGradient),
    RadialGradient(RadialGradient),
}

pub struct Path {
    pub fill: Option<Fill>,
    pub stroke: Option<Stroke>,
    /// All segments are in absolute coordinates.
    pub d: Vec<PathSegment>,
}

pub struct BaseGradient {
    pub units: GradientUnits,
    pub transform: Transform,
    pub spread_method: SpreadMethod,
    pub stops: Vec<Stop>,
}

pub struct LinearGradient {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub d: BaseGradient,
}

impl fmt::Debug for LinearGradient {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
            "LinearGradient(\n  \
               x1: {} y1: {}\n  \
               x2: {} y2: {}\n  \
               units: {:?}\n  \
               transform: {}\n  \
               spread: {:?}\n",
            self.x1, self.y1, self.x2, self.y2,
            self.d.units, self.d.transform, self.d.spread_method
        )?;

        for stop in &self.d.stops {
            write!(f, "    {:?}\n", stop)?;
        }

        write!(f, ")")
    }
}

pub struct RadialGradient {
    pub cx: f64,
    pub cy: f64,
    pub r: f64,
    pub fx: f64,
    pub fy: f64,
    pub d: BaseGradient,
}

impl fmt::Debug for RadialGradient {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
            "RadialGradient(\n  \
               cx: {} cy: {}\n  \
               fx: {} fy: {}\n  \
               r: {}\n  \
               units: {:?}\n  \
               transform: {}\n  \
               spread: {:?}\n",
            self.cx, self.cy, self.fx, self.fy, self.r,
            self.d.units, self.d.transform, self.d.spread_method
        )?;

        for stop in &self.d.stops {
            write!(f, "    {:?}\n", stop)?;
        }

        write!(f, ")")
    }
}

pub struct Stop {
    pub offset: f64,
    pub color: Color,
    pub opacity: f64,
}

impl fmt::Debug for Stop {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Stop(offset: {:?}, color: {}, opacity: {:?})",
               self.offset, self.color, self.opacity)
    }
}

pub struct Text {
    pub children: Vec<TextChunk>,
}

pub struct TextChunk {
    pub x: f64,
    pub y: f64,
    pub anchor: TextAnchor,
    pub children: Vec<TSpan>
}

// TODO: dx, dy
#[derive(Clone)]
pub struct TSpan {
    pub fill: Option<Fill>,
    pub stroke: Option<Stroke>,
    pub font: Font,
    pub decoration: TextDecoration,
    pub text: String,
}

pub struct Image {
    pub rect: Rect,
    pub data: ImageData,
}

pub enum ImageData {
    Path(PathBuf),
    Raw(Vec<u8>, ImageDataKind),
}

#[derive(Copy,Clone,PartialEq)]
pub enum ImageDataKind {
    PNG,
    JPEG,
}

// TODO: no need for a separate vector
pub struct Group {
    pub opacity: Option<f64>,
    pub children: Vec<Element>,
}
