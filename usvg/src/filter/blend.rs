// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::svgtree::{self, AId};
use super::{FilterInput, FilterKind, FilterPrimitive};

/// A blend filter primitive.
///
/// `feBlend` element in the SVG.
#[derive(Clone, Debug)]
pub struct FeBlend {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input1: FilterInput,

    /// Identifies input for the given filter primitive.
    ///
    /// `in2` in the SVG.
    pub input2: FilterInput,

    /// A blending mode.
    ///
    /// `mode` in the SVG.
    pub mode: FeBlendMode,
}

/// An images blending mode.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FeBlendMode {
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

pub(crate) fn convert(fe: svgtree::Node, primitives: &[FilterPrimitive]) -> FilterKind {
    let mode = match fe.attribute(AId::Mode).unwrap_or("normal") {
        "normal" => FeBlendMode::Normal,
        "multiply" => FeBlendMode::Multiply,
        "screen" => FeBlendMode::Screen,
        "overlay" => FeBlendMode::Overlay,
        "darken" => FeBlendMode::Darken,
        "lighten" => FeBlendMode::Lighten,
        "color-dodge" => FeBlendMode::ColorDodge,
        "color-burn" => FeBlendMode::ColorBurn,
        "hard-light" => FeBlendMode::HardLight,
        "soft-light" => FeBlendMode::SoftLight,
        "difference" => FeBlendMode::Difference,
        "exclusion" => FeBlendMode::Exclusion,
        "hue" => FeBlendMode::Hue,
        "saturation" => FeBlendMode::Saturation,
        "color" => FeBlendMode::Color,
        "luminosity" => FeBlendMode::Luminosity,
        _ => FeBlendMode::Normal,
    };

    let input1 = super::resolve_input(fe, AId::In, primitives);
    let input2 = super::resolve_input(fe, AId::In2, primitives);

    FilterKind::FeBlend(FeBlend {
        mode,
        input1,
        input2,
    })
}
