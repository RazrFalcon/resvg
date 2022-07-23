// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use strict_num::PositiveF64;

use crate::svgtree::{self, AId};
use super::{Input, Kind, Primitive};

/// A Gaussian blur filter primitive.
///
/// `feGaussianBlur` element in the SVG.
#[derive(Clone, Debug)]
pub struct GaussianBlur {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: Input,

    /// A standard deviation along the X-axis.
    ///
    /// `stdDeviation` in the SVG.
    pub std_dev_x: PositiveF64,

    /// A standard deviation along the Y-axis.
    ///
    /// `stdDeviation` in the SVG.
    pub std_dev_y: PositiveF64,
}

pub(crate) fn convert(fe: svgtree::Node, primitives: &[Primitive]) -> Kind {
    let (std_dev_x, std_dev_y) = convert_std_dev_attr(fe, "0 0");
    Kind::GaussianBlur(GaussianBlur {
        input: super::resolve_input(fe, AId::In, primitives),
        std_dev_x,
        std_dev_y,
    })
}

pub(crate) fn convert_std_dev_attr(fe: svgtree::Node, default: &str) -> (PositiveF64, PositiveF64) {
    let text = fe.attribute::<&str>(AId::StdDeviation).unwrap_or(default);
    let mut parser = svgtypes::NumberListParser::from(text);

    let n1 = parser.next().and_then(|n| n.ok());
    let n2 = parser.next().and_then(|n| n.ok());
    // `stdDeviation` must have no more than two values.
    // Otherwise we should fallback to `0 0`.
    let n3 = parser.next().and_then(|n| n.ok());

    let (std_dev_x, std_dev_y) = match (n1, n2, n3) {
        (Some(n1), Some(n2), None) => (n1, n2),
        (Some(n1), None, None) => (n1, n1),
        _ => (0.0, 0.0),
    };

    let std_dev_x = PositiveF64::new(std_dev_x).unwrap_or(PositiveF64::ZERO);
    let std_dev_y = PositiveF64::new(std_dev_y).unwrap_or(PositiveF64::ZERO);

    (std_dev_x, std_dev_y)
}
