// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::svgtree::{self, AId};
use crate::{FuzzyZero, NonZeroF64};
use super::{Input, Kind, Primitive};

/// A matrix convolution filter primitive.
///
/// `feConvolveMatrix` element in the SVG.
#[derive(Clone, Debug)]
pub struct ConvolveMatrix {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: Input,

    /// A convolve matrix.
    pub matrix: ConvolveMatrixData,

    /// A matrix divisor.
    ///
    /// `divisor` in the SVG.
    pub divisor: NonZeroF64,

    /// A kernel matrix bias.
    ///
    /// `bias` in the SVG.
    pub bias: f64,

    /// An edges processing mode.
    ///
    /// `edgeMode` in the SVG.
    pub edge_mode: EdgeMode,

    /// An alpha preserving flag.
    ///
    /// `preserveAlpha` in the SVG.
    pub preserve_alpha: bool,
}

/// A convolve matrix representation.
///
/// Used primarily by [`ConvolveMatrix`].
#[derive(Clone, Debug)]
pub struct ConvolveMatrixData {
    x: u32,
    y: u32,
    columns: u32,
    rows: u32,
    data: Vec<f64>,
}

impl ConvolveMatrixData {
    /// Creates a new `ConvolveMatrixData`.
    ///
    /// Returns `None` when:
    ///
    /// - `columns` * `rows` != `data.len()`
    /// - `target_x` >= `columns`
    /// - `target_y` >= `rows`
    pub fn new(target_x: u32, target_y: u32, columns: u32, rows: u32, data: Vec<f64>) -> Option<Self> {
        if (columns * rows) as usize != data.len()
           || target_x >= columns
           || target_y >= rows
        {
            return None;
        }

        Some(ConvolveMatrixData {
            x: target_x,
            y: target_y,
            columns,
            rows,
            data,
        })
    }

    /// Returns a matrix's X target.
    ///
    /// `targetX` in the SVG.
    #[inline]
    pub fn target_x(&self) -> u32 {
        self.x
    }

    /// Returns a matrix's Y target.
    ///
    /// `targetY` in the SVG.
    #[inline]
    pub fn target_y(&self) -> u32 {
        self.y
    }

    /// Returns a number of columns in the matrix.
    ///
    /// Part of the `order` attribute in the SVG.
    #[inline]
    pub fn columns(&self) -> u32 {
        self.columns
    }

    /// Returns a number of rows in the matrix.
    ///
    /// Part of the `order` attribute in the SVG.
    #[inline]
    pub fn rows(&self) -> u32 {
        self.rows
    }

    /// Returns a matrix value at the specified position.
    ///
    /// # Panics
    ///
    /// - When position is out of bounds.
    #[inline]
    pub fn get(&self, x: u32, y: u32) -> f64 {
        self.data[(y * self.columns + x) as usize]
    }

    /// Returns a reference to an internal data.
    #[inline]
    pub fn data(&self) -> &[f64] {
        &self.data
    }
}

/// An edges processing mode.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EdgeMode {
    None,
    Duplicate,
    Wrap,
}

pub(crate) fn convert(fe: svgtree::Node, primitives: &[Primitive]) -> Option<Kind> {
    fn parse_target(target: Option<f64>, order: u32) -> Option<u32> {
        let default_target = (order as f32 / 2.0).floor() as u32;
        let target = target.unwrap_or(default_target as f64) as i32;
        if target < 0 || target >= order as i32 {
            None
        } else {
            Some(target as u32)
        }
    }

    let mut order_x = 3;
    let mut order_y = 3;
    if let Some(value) = fe.attribute::<&str>(AId::Order) {
        let mut s = svgtypes::NumberListParser::from(value);
        let x = s.next().and_then(|a| a.ok()).map(|n| n as i32).unwrap_or(3);
        let y = s.next().and_then(|a| a.ok()).map(|n| n as i32).unwrap_or(x);
        if x > 0 && y > 0 {
            order_x = x as u32;
            order_y = y as u32;
        }
    }

    let mut matrix = Vec::new();
    if let Some(list) = fe.attribute::<&Vec<f64>>(AId::KernelMatrix) {
        if list.len() == (order_x * order_y) as usize {
            matrix = list.clone();
        }
    }

    let mut kernel_sum: f64 = matrix.iter().sum();
    // Round up to prevent float precision issues.
    kernel_sum = (kernel_sum * 1_000_000.0).round() / 1_000_000.0;
    if kernel_sum.is_fuzzy_zero() {
        kernel_sum = 1.0;
    }

    let divisor = fe.attribute::<f64>(AId::Divisor).unwrap_or(kernel_sum);
    if divisor.is_fuzzy_zero() {
        return None;
    }

    let bias = fe.attribute(AId::Bias).unwrap_or(0.0);

    let target_x = parse_target(fe.attribute(AId::TargetX), order_x)?;
    let target_y = parse_target(fe.attribute(AId::TargetY), order_y)?;

    let kernel_matrix = ConvolveMatrixData::new(
        target_x, target_y, order_x, order_y, matrix,
    )?;

    let edge_mode = match fe.attribute(AId::EdgeMode).unwrap_or("duplicate") {
        "none" => EdgeMode::None,
        "wrap" => EdgeMode::Wrap,
        _      => EdgeMode::Duplicate,
    };

    let preserve_alpha = fe.attribute(AId::PreserveAlpha).unwrap_or("false") == "true";

    Some(Kind::ConvolveMatrix(ConvolveMatrix {
        input: super::resolve_input(fe, AId::In, primitives),
        matrix: kernel_matrix,
        divisor: NonZeroF64::new(divisor).unwrap(),
        bias,
        edge_mode,
        preserve_alpha,
    }))
}
