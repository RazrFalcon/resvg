// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::svgtree::{self, AId};
use super::{Input, Kind, Primitive};

/// A blend filter primitive.
///
/// `feBlend` element in the SVG.
#[derive(Clone, Debug)]
pub struct Blend {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input1: Input,

    /// Identifies input for the given filter primitive.
    ///
    /// `in2` in the SVG.
    pub input2: Input,

    /// A blending mode.
    ///
    /// `mode` in the SVG.
    pub mode: BlendMode,
}

/// An images blending mode.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

pub(crate) fn convert(fe: svgtree::Node, primitives: &[Primitive]) -> Kind {
    let mode = match fe.attribute(AId::Mode).unwrap_or("normal") {
        "normal" => BlendMode::Normal,
        "multiply" => BlendMode::Multiply,
        "screen" => BlendMode::Screen,
        "overlay" => BlendMode::Overlay,
        "darken" => BlendMode::Darken,
        "lighten" => BlendMode::Lighten,
        "color-dodge" => BlendMode::ColorDodge,
        "color-burn" => BlendMode::ColorBurn,
        "hard-light" => BlendMode::HardLight,
        "soft-light" => BlendMode::SoftLight,
        "difference" => BlendMode::Difference,
        "exclusion" => BlendMode::Exclusion,
        "hue" => BlendMode::Hue,
        "saturation" => BlendMode::Saturation,
        "color" => BlendMode::Color,
        "luminosity" => BlendMode::Luminosity,
        _ => BlendMode::Normal,
    };

    let input1 = super::resolve_input(fe, AId::In, primitives);
    let input2 = super::resolve_input(fe, AId::In2, primitives);

    Kind::Blend(Blend {
        mode,
        input1,
        input2,
    })
}
