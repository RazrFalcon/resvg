// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use core::cmp;

use crate::{ImageRefMut, f64_bound};

/// A transfer function used `component_transfer`.
///
/// <https://www.w3.org/TR/SVG11/filters.html#transferFuncElements>
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug)]
pub enum TransferFunction<'a> {
    /// Keeps component as is.
    Identity,

    /// Applies a linear interpolation to a component.
    ///
    /// The number list can be empty.
    Table(&'a [f64]),

    /// Applies a step function to a component.
    ///
    /// The number list can be empty.
    Discrete(&'a [f64]),

    /// Applies a linear shift to a component.
    Linear {
        slope: f64,
        intercept: f64,
    },

    /// Applies an exponential shift to a component.
    Gamma {
        amplitude: f64,
        exponent: f64,
        offset: f64,
    },
}

impl<'a> TransferFunction<'a> {
    fn is_dummy(&self) -> bool {
        match self {
            TransferFunction::Identity => true,
            TransferFunction::Table(values) => values.is_empty(),
            TransferFunction::Discrete(values) => values.is_empty(),
            TransferFunction::Linear { .. } => false,
            TransferFunction::Gamma { .. } => false,
        }
    }

    fn apply(&self, c: u8) -> u8 {
        let c = c as f64 / 255.0;
        let c = match self {
            TransferFunction::Identity => {
                c
            }
            TransferFunction::Table(values) => {
                let n = values.len() - 1;
                let k = (c * (n as f64)).floor() as usize;
                let k = cmp::min(k, n);
                if k == n {
                    values[k]
                } else {
                    let vk = values[k];
                    let vk1 = values[k + 1];
                    let k = k as f64;
                    let n = n as f64;
                    vk + (c - k / n) * n * (vk1 - vk)
                }
            }
            TransferFunction::Discrete(values) => {
                let n = values.len();
                let k = (c * (n as f64)).floor() as usize;
                values[cmp::min(k, n - 1)]
            }
            TransferFunction::Linear { slope, intercept } => {
                slope * c + intercept
            }
            TransferFunction::Gamma { amplitude, exponent, offset } => {
                amplitude * c.powf(*exponent) + offset
            }
        };

        (f64_bound(0.0, c, 1.0) * 255.0) as u8
    }
}

/// Applies component transfer functions for each `src` image channel.
///
/// Input image pixels should have an **unpremultiplied alpha**.
pub fn component_transfer(
    func_b: TransferFunction,
    func_g: TransferFunction,
    func_r: TransferFunction,
    func_a: TransferFunction,
    src: ImageRefMut,
) {
    for pixel in src.data {
        if !func_b.is_dummy() {
            pixel.b = func_b.apply(pixel.b);
        }

        if !func_g.is_dummy() {
            pixel.g = func_g.apply(pixel.g);
        }

        if !func_r.is_dummy() {
            pixel.r = func_r.apply(pixel.r);
        }

        if !func_a.is_dummy() {
            pixel.a = func_a.apply(pixel.a);
        }
    }
}
