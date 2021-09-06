// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgtypes::Length;

use crate::svgtree::{self, AId};
use crate::{Color, Opacity, PositiveNumber, converter};
use super::{Input, Kind, Primitive};

/// A drop shadow filter primitive.
///
/// This is essentially `feGaussianBlur`, `feOffset` and `feFlood` joined together.
///
/// `feDropShadow` element in the SVG.
#[derive(Clone, Debug)]
pub struct DropShadow {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: Input,

    /// The amount to offset the input graphic along the X-axis.
    pub dx: f64,

    /// The amount to offset the input graphic along the Y-axis.
    pub dy: f64,

    /// A standard deviation along the X-axis.
    ///
    /// `stdDeviation` in the SVG.
    pub std_dev_x: PositiveNumber,

    /// A standard deviation along the Y-axis.
    ///
    /// `stdDeviation` in the SVG.
    pub std_dev_y: PositiveNumber,

    /// A flood color.
    ///
    /// `flood-color` in the SVG.
    pub color: Color,

    /// A flood opacity.
    ///
    /// `flood-opacity` in the SVG.
    pub opacity: Opacity,
}

pub(crate) fn convert(
    fe: svgtree::Node,
    primitives: &[Primitive],
    state: &converter::State,
) -> Kind {
    let (std_dev_x, std_dev_y) = super::gaussian_blur::convert_std_dev_attr(fe, "2 2");

    Kind::DropShadow(DropShadow {
        input: super::resolve_input(fe, AId::In, primitives),
        dx: fe.convert_user_length(AId::Dx, state, Length::new_number(2.0)),
        dy: fe.convert_user_length(AId::Dy, state, Length::new_number(2.0)),
        std_dev_x: std_dev_x.into(),
        std_dev_y: std_dev_y.into(),
        color: fe.attribute(AId::FloodColor).unwrap_or_else(Color::black),
        opacity: fe.attribute(AId::FloodOpacity).unwrap_or_default(),
    })
}
