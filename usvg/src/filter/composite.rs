// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::svgtree::{self, AId};
use super::{Input, Kind, Primitive};

/// A composite filter primitive.
///
/// `feComposite` element in the SVG.
#[derive(Clone, Debug)]
pub struct Composite {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input1: Input,

    /// Identifies input for the given filter primitive.
    ///
    /// `in2` in the SVG.
    pub input2: Input,

    /// A compositing operation.
    ///
    /// `operator` in the SVG.
    pub operator: CompositeOperator,
}

/// An images compositing operation.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CompositeOperator {
    Over,
    In,
    Out,
    Atop,
    Xor,
    Arithmetic {
        k1: f64,
        k2: f64,
        k3: f64,
        k4: f64,
    },
}

pub(crate) fn convert(fe: svgtree::Node, primitives: &[Primitive]) -> Kind {
    let operator = match fe.attribute(AId::Operator).unwrap_or("over") {
        "in"            => CompositeOperator::In,
        "out"           => CompositeOperator::Out,
        "atop"          => CompositeOperator::Atop,
        "xor"           => CompositeOperator::Xor,
        "arithmetic"    => {
            CompositeOperator::Arithmetic {
                k1: fe.attribute(AId::K1).unwrap_or(0.0),
                k2: fe.attribute(AId::K2).unwrap_or(0.0),
                k3: fe.attribute(AId::K3).unwrap_or(0.0),
                k4: fe.attribute(AId::K4).unwrap_or(0.0),
            }
        }
        _ => CompositeOperator::Over,
    };

    let input1 = super::resolve_input(fe, AId::In, primitives);
    let input2 = super::resolve_input(fe, AId::In2, primitives);

    Kind::Composite(Composite {
        operator,
        input1,
        input2,
    })
}
