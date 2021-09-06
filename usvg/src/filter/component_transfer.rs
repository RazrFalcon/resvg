// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::svgtree::{self, AId, EId};
use super::{Input, Kind, Primitive};

/// A component-wise remapping filter primitive.
///
/// `feComponentTransfer` element in the SVG.
#[derive(Clone, Debug)]
pub struct ComponentTransfer {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: Input,

    /// `feFuncR` in the SVG.
    pub func_r: TransferFunction,

    /// `feFuncG` in the SVG.
    pub func_g: TransferFunction,

    /// `feFuncB` in the SVG.
    pub func_b: TransferFunction,

    /// `feFuncA` in the SVG.
    pub func_a: TransferFunction,
}

/// A transfer function used by `FeComponentTransfer`.
///
/// <https://www.w3.org/TR/SVG11/filters.html#transferFuncElements>
#[derive(Clone, Debug)]
pub enum TransferFunction {
    /// Keeps a component as is.
    Identity,

    /// Applies a linear interpolation to a component.
    ///
    /// The number list can be empty.
    Table(Vec<f64>),

    /// Applies a step function to a component.
    ///
    /// The number list can be empty.
    Discrete(Vec<f64>),

    /// Applies a linear shift to a component.
    #[allow(missing_docs)]
    Linear {
        slope: f64,
        intercept: f64,
    },

    /// Applies an exponential shift to a component.
    #[allow(missing_docs)]
    Gamma {
        amplitude: f64,
        exponent: f64,
        offset: f64,
    },
}

pub(crate) fn convert(fe: svgtree::Node, primitives: &[Primitive]) -> Kind {
    let mut kind = ComponentTransfer {
        input: super::resolve_input(fe, AId::In, primitives),
        func_r: TransferFunction::Identity,
        func_g: TransferFunction::Identity,
        func_b: TransferFunction::Identity,
        func_a: TransferFunction::Identity,
    };

    for child in fe.children().filter(|n| n.is_element()) {
        if let Some(func) = convert_transfer_function(child) {
            match child.tag_name().unwrap() {
                EId::FeFuncR => kind.func_r = func,
                EId::FeFuncG => kind.func_g = func,
                EId::FeFuncB => kind.func_b = func,
                EId::FeFuncA => kind.func_a = func,
                _ => {}
            }
        }
    }

    Kind::ComponentTransfer(kind)
}

fn convert_transfer_function(node: svgtree::Node) -> Option<TransferFunction> {
    match node.attribute(AId::Type)? {
        "identity" => {
            Some(TransferFunction::Identity)
        }
        "table" => {
            match node.attribute::<&Vec<f64>>(AId::TableValues) {
                Some(values) => Some(TransferFunction::Table(values.clone())),
                None => Some(TransferFunction::Table(Vec::new())),
            }
        }
        "discrete" => {
            match node.attribute::<&Vec<f64>>(AId::TableValues) {
                Some(values) => Some(TransferFunction::Discrete(values.clone())),
                None => Some(TransferFunction::Discrete(Vec::new())),
            }
        }
        "linear" => {
            Some(TransferFunction::Linear {
                slope: node.attribute(AId::Slope).unwrap_or(1.0),
                intercept: node.attribute(AId::Intercept).unwrap_or(0.0),
            })
        }
        "gamma" => {
            Some(TransferFunction::Gamma {
                amplitude: node.attribute(AId::Amplitude).unwrap_or(1.0),
                exponent: node.attribute(AId::Exponent).unwrap_or(1.0),
                offset: node.attribute(AId::Offset).unwrap_or(0.0),
            })
        }
        _ => None,
    }
}
