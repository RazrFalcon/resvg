// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::svgtree::{self, AId};
use super::{Color, Kind, Opacity};

/// A flood filter primitive.
///
/// `feFlood` element in the SVG.
#[derive(Clone, Copy, Debug)]
pub struct Flood {
    /// A flood color.
    ///
    /// `flood-color` in the SVG.
    pub color: Color,

    /// A flood opacity.
    ///
    /// `flood-opacity` in the SVG.
    pub opacity: Opacity,
}

pub(crate) fn convert(fe: svgtree::Node) -> Kind {
    Kind::Flood(Flood {
        color: fe.attribute(AId::FloodColor).unwrap_or_else(Color::black),
        opacity: fe.attribute(AId::FloodOpacity).unwrap_or_default(),
    })
}
