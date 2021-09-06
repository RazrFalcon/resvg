// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::svgtree::{self, AId};
use crate::PositiveNumber;
use super::{Input, Kind, Primitive};

/// A color matrix filter primitive.
///
/// `feColorMatrix` element in the SVG.
#[derive(Clone, Debug)]
pub struct ColorMatrix {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: Input,

    /// A matrix kind.
    ///
    /// `type` in the SVG.
    pub kind: ColorMatrixKind,
}

/// A color matrix filter primitive kind.
#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub enum ColorMatrixKind {
    Matrix(Vec<f64>), // Guarantee to have 20 numbers.
    Saturate(PositiveNumber),
    HueRotate(f64),
    LuminanceToAlpha,
}

impl Default for ColorMatrixKind {
    fn default() -> Self {
        ColorMatrixKind::Matrix(vec![
            1.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 1.0, 0.0,
        ])
    }
}

pub(crate) fn convert(fe: svgtree::Node, primitives: &[Primitive]) -> Kind {
    let kind = convert_color_matrix_kind(fe).unwrap_or_default();
    Kind::ColorMatrix(ColorMatrix {
        input: super::resolve_input(fe, AId::In, primitives),
        kind,
    })
}

fn convert_color_matrix_kind(fe: svgtree::Node) -> Option<ColorMatrixKind> {
    match fe.attribute(AId::Type) {
        Some("saturate") => {
            if let Some(list) = fe.attribute::<&Vec<f64>>(AId::Values) {
                if !list.is_empty() {
                    let n = crate::utils::f64_bound(0.0, list[0], 1.0);
                    return Some(ColorMatrixKind::Saturate(n.into()));
                } else {
                    return Some(ColorMatrixKind::Saturate(1.0.into()));
                }
            }
        }
        Some("hueRotate") => {
            if let Some(list) = fe.attribute::<&Vec<f64>>(AId::Values) {
                if !list.is_empty() {
                    return Some(ColorMatrixKind::HueRotate(list[0]));
                } else {
                    return Some(ColorMatrixKind::HueRotate(0.0));
                }
            }
        }
        Some("luminanceToAlpha") => {
            return Some(ColorMatrixKind::LuminanceToAlpha);
        }
        _ => {
            // Fallback to `matrix`.
            if let Some(list) = fe.attribute::<&Vec<f64>>(AId::Values) {
                if list.len() == 20 {
                    return Some(ColorMatrixKind::Matrix(list.clone()));
                }
            }
        }
    }

    None
}
