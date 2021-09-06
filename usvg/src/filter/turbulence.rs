// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::svgtree::{self, AId};
use crate::{Point, PositiveNumber};
use super::Kind;

/// A turbulence generation filter primitive.
///
/// `feTurbulence` element in the SVG.
#[derive(Clone, Copy, Debug)]
pub struct Turbulence {
    /// Identifies the base frequency for the noise function.
    ///
    /// `baseFrequency` in the SVG.
    pub base_frequency: Point<PositiveNumber>,

    /// Identifies the number of octaves for the noise function.
    ///
    /// `numOctaves` in the SVG.
    pub num_octaves: u32,

    /// The starting number for the pseudo random number generator.
    ///
    /// `seed` in the SVG.
    pub seed: i32,

    /// Smooth transitions at the border of tiles.
    ///
    /// `stitchTiles` in the SVG.
    pub stitch_tiles: bool,

    /// Indicates whether the filter primitive should perform a noise or turbulence function.
    ///
    /// `type` in the SVG.
    pub kind: TurbulenceKind,
}

/// A turbulence kind for the `feTurbulence` filter.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TurbulenceKind {
    FractalNoise,
    Turbulence,
}

pub(crate) fn convert(fe: svgtree::Node) -> Kind {
    let mut base_frequency = Point::new(0.0.into(), 0.0.into());
    if let Some(list) = fe.attribute::<&Vec<f64>>(AId::BaseFrequency) {
        let mut x = 0.0;
        let mut y = 0.0;
        if list.len() == 2 {
            x = list[0];
            y = list[1];
        } else if list.len() == 1 {
            x = list[0];
            y = list[0]; // The same as `x`.
        }

        if x.is_sign_positive() && y.is_sign_positive() {
            base_frequency = Point::new(x.into(), y.into());
        }
    }

    let mut num_octaves = fe.attribute(AId::NumOctaves).unwrap_or(1.0);
    if num_octaves.is_sign_negative() {
        num_octaves = 0.0;
    }

    let kind = match fe.attribute(AId::Type).unwrap_or("turbulence") {
        "fractalNoise" => TurbulenceKind::FractalNoise,
        _              => TurbulenceKind::Turbulence,
    };

    Kind::Turbulence(Turbulence {
        base_frequency,
        num_octaves: num_octaves.round() as u32,
        seed: fe.attribute(AId::Seed).unwrap_or(0.0).trunc() as i32,
        stitch_tiles: fe.attribute(AId::StitchTiles) == Some("stitch"),
        kind,
    })
}
