// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use super::{Input, Kind, Primitive};
use crate::svgtree::{self, AId};

/// An offset filter primitive.
///
/// `feOffset` element in the SVG.
#[derive(Clone, Debug)]
pub struct Offset {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: Input,

    /// The amount to offset the input graphic along the X-axis.
    pub dx: f64,

    /// The amount to offset the input graphic along the Y-axis.
    pub dy: f64,
}

pub(crate) fn convert(fe: svgtree::Node, primitives: &[Primitive]) -> Kind {
    Kind::Offset(Offset {
        input: super::resolve_input(fe, AId::In, primitives),
        dx: fe.attribute::<f64>(AId::Dx).unwrap_or(0.0),
        dy: fe.attribute::<f64>(AId::Dy).unwrap_or(0.0),
    })
}
