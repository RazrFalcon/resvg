// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::svgtree::{self, AId};
use crate::{FuzzyZero, PositiveNumber};
use super::{Input, Kind, Primitive};

/// A morphology filter primitive.
///
/// `feMorphology` element in the SVG.
#[derive(Clone, Debug)]
pub struct Morphology {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: Input,

    /// A filter operator.
    ///
    /// `operator` in the SVG.
    pub operator: MorphologyOperator,

    /// A filter radius along the X-axis.
    ///
    /// A value of zero disables the effect of the given filter primitive.
    ///
    /// `radius` in the SVG.
    pub radius_x: PositiveNumber,

    /// A filter radius along the Y-axis.
    ///
    /// A value of zero disables the effect of the given filter primitive.
    ///
    /// `radius` in the SVG.
    pub radius_y: PositiveNumber,
}

/// A morphology operation.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MorphologyOperator {
    Erode,
    Dilate,
}

pub(crate) fn convert(fe: svgtree::Node, primitives: &[Primitive]) -> Kind {
    let operator = match fe.attribute(AId::Operator).unwrap_or("erode") {
        "dilate" => MorphologyOperator::Dilate,
        _        => MorphologyOperator::Erode,
    };

    // Both radius are zero by default.
    let mut radius_x = PositiveNumber::new(0.0);
    let mut radius_y = PositiveNumber::new(0.0);
    if let Some(list) = fe.attribute::<&Vec<f64>>(AId::Radius) {
        let mut rx = 0.0;
        let mut ry = 0.0;
        if list.len() == 2 {
            rx = list[0];
            ry = list[1];
        } else if list.len() == 1 {
            rx = list[0];
            ry = list[0]; // The same as `rx`.
        }

        // If only one of the values is zero, reset it to 1.0
        // This is not specified in the spec, but this is how Chrome and Firefox works.
        if rx.is_fuzzy_zero() && !ry.is_fuzzy_zero() {
            rx = 1.0;
        }
        if !rx.is_fuzzy_zero() && ry.is_fuzzy_zero() {
            ry = 1.0;
        }

        // Both values must be positive.
        if rx.is_sign_positive() && ry.is_sign_positive() {
            radius_x = PositiveNumber::new(rx);
            radius_y = PositiveNumber::new(ry);
        }
    }

    Kind::Morphology(Morphology {
        input: super::resolve_input(fe, AId::In, primitives),
        operator,
        radius_x,
        radius_y,
    })
}
