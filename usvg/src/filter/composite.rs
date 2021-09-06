// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::svgtree::{self, AId};
use crate::{FilterInput, FilterKind, FilterPrimitive};

/// A composite filter primitive.
///
/// `feComposite` element in the SVG.
#[derive(Clone, Debug)]
pub struct FeComposite {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input1: FilterInput,

    /// Identifies input for the given filter primitive.
    ///
    /// `in2` in the SVG.
    pub input2: FilterInput,

    /// A compositing operation.
    ///
    /// `operator` in the SVG.
    pub operator: FeCompositeOperator,
}

/// An images compositing operation.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FeCompositeOperator {
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

pub(crate) fn convert(fe: svgtree::Node, primitives: &[FilterPrimitive]) -> FilterKind {
    let operator = match fe.attribute(AId::Operator).unwrap_or("over") {
        "in"            => FeCompositeOperator::In,
        "out"           => FeCompositeOperator::Out,
        "atop"          => FeCompositeOperator::Atop,
        "xor"           => FeCompositeOperator::Xor,
        "arithmetic"    => {
            FeCompositeOperator::Arithmetic {
                k1: fe.attribute(AId::K1).unwrap_or(0.0),
                k2: fe.attribute(AId::K2).unwrap_or(0.0),
                k3: fe.attribute(AId::K3).unwrap_or(0.0),
                k4: fe.attribute(AId::K4).unwrap_or(0.0),
            }
        }
        _ => FeCompositeOperator::Over,
    };

    let input1 = super::resolve_input(fe, AId::In, primitives);
    let input2 = super::resolve_input(fe, AId::In2, primitives);

    FilterKind::FeComposite(FeComposite {
        operator,
        input1,
        input2,
    })
}
