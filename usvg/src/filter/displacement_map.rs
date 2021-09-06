// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::svgtree::{self, AId};
use super::{Input, Kind, Primitive};

/// A displacement map filter primitive.
///
/// `feDisplacementMap` element in the SVG.
#[derive(Clone, Debug)]
pub struct DisplacementMap {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input1: Input,

    /// Identifies input for the given filter primitive.
    ///
    /// `in2` in the SVG.
    pub input2: Input,

    /// Scale factor.
    ///
    /// `scale` in the SVG.
    pub scale: f64,

    /// Indicates a source color channel along the X-axis.
    ///
    /// `xChannelSelector` in the SVG.
    pub x_channel_selector: ColorChannel,

    /// Indicates a source color channel along the Y-axis.
    ///
    /// `yChannelSelector` in the SVG.
    pub y_channel_selector: ColorChannel,
}

/// A color channel.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ColorChannel {
    R,
    G,
    B,
    A,
}

pub(crate) fn convert(fe: svgtree::Node, primitives: &[Primitive]) -> Kind {
    let parse_channel = |aid| {
        match fe.attribute(aid).unwrap_or("A") {
            "R" => ColorChannel::R,
            "G" => ColorChannel::G,
            "B" => ColorChannel::B,
            _   => ColorChannel::A,
        }
    };

    Kind::DisplacementMap(DisplacementMap {
        input1: super::resolve_input(fe, AId::In, primitives),
        input2: super::resolve_input(fe, AId::In2, primitives),
        scale: fe.attribute(AId::Scale).unwrap_or(0.0),
        x_channel_selector: parse_channel(AId::XChannelSelector),
        y_channel_selector: parse_channel(AId::YChannelSelector),
    })
}
