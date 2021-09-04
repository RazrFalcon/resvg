// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{ImageRefMut, RGBA8, f64_bound};

#[inline]
fn to_normalized_components(pixel: RGBA8) -> (f64, f64, f64, f64) {
    (pixel.r as f64 / 255.0,
     pixel.g as f64 / 255.0,
     pixel.b as f64 / 255.0,
     pixel.a as f64 / 255.0)
}

#[inline]
fn from_normalized(c: f64) -> u8 {
    (f64_bound(0.0, c, 1.0) * 255.0) as u8
}

/// A color matrix used by `color_matrix`.
#[derive(Clone, Copy, Debug)]
#[allow(missing_docs)]
pub enum ColorMatrix<'a> {
    Matrix(&'a [f64; 20]),
    Saturate(f64),
    HueRotate(f64),
    LuminanceToAlpha,
}

/// Applies a color matrix filter.
///
/// Input image pixels should have an **unpremultiplied alpha**.
pub fn color_matrix(
    matrix: ColorMatrix,
    src: ImageRefMut,
) {
    match matrix {
        ColorMatrix::Matrix(m) => {
            for pixel in src.data {
                let (r, g, b, a) = to_normalized_components(*pixel);

                let new_r = r * m[0]  + g * m[1]  + b * m[2]  + a * m[3]  + m[4];
                let new_g = r * m[5]  + g * m[6]  + b * m[7]  + a * m[8]  + m[9];
                let new_b = r * m[10] + g * m[11] + b * m[12] + a * m[13] + m[14];
                let new_a = r * m[15] + g * m[16] + b * m[17] + a * m[18] + m[19];

                pixel.r = from_normalized(new_r);
                pixel.g = from_normalized(new_g);
                pixel.b = from_normalized(new_b);
                pixel.a = from_normalized(new_a);
            }
        }
        ColorMatrix::Saturate(v) => {
            let v = v.max(0.0);
            let m = [
                0.213 + 0.787 * v, 0.715 - 0.715 * v, 0.072 - 0.072 * v,
                0.213 - 0.213 * v, 0.715 + 0.285 * v, 0.072 - 0.072 * v,
                0.213 - 0.213 * v, 0.715 - 0.715 * v, 0.072 + 0.928 * v,
            ];

            for pixel in src.data {
                let (r, g, b, _) = to_normalized_components(*pixel);

                let new_r = r * m[0] + g * m[1] + b * m[2];
                let new_g = r * m[3] + g * m[4] + b * m[5];
                let new_b = r * m[6] + g * m[7] + b * m[8];

                pixel.r = from_normalized(new_r);
                pixel.g = from_normalized(new_g);
                pixel.b = from_normalized(new_b);
            }
        }
        ColorMatrix::HueRotate(angle) => {
            let angle = angle.to_radians();
            let a1 = angle.cos();
            let a2 = angle.sin();
            let m = [
                0.213 + 0.787 * a1 - 0.213 * a2,
                0.715 - 0.715 * a1 - 0.715 * a2,
                0.072 - 0.072 * a1 + 0.928 * a2,
                0.213 - 0.213 * a1 + 0.143 * a2,
                0.715 + 0.285 * a1 + 0.140 * a2,
                0.072 - 0.072 * a1 - 0.283 * a2,
                0.213 - 0.213 * a1 - 0.787 * a2,
                0.715 - 0.715 * a1 + 0.715 * a2,
                0.072 + 0.928 * a1 + 0.072 * a2,
            ];

            for pixel in src.data {
                let (r, g, b, _) = to_normalized_components(*pixel);

                let new_r = r * m[0] + g * m[1] + b * m[2];
                let new_g = r * m[3] + g * m[4] + b * m[5];
                let new_b = r * m[6] + g * m[7] + b * m[8];

                pixel.r = from_normalized(new_r);
                pixel.g = from_normalized(new_g);
                pixel.b = from_normalized(new_b);
            }
        }
        ColorMatrix::LuminanceToAlpha => {
            for pixel in src.data {
                let (r, g, b, _) = to_normalized_components(*pixel);

                let new_a = r * 0.2125 + g * 0.7154 + b * 0.0721;

                pixel.r = 0;
                pixel.g = 0;
                pixel.b = 0;
                pixel.a = from_normalized(new_a);
            }
        }
    }
}
