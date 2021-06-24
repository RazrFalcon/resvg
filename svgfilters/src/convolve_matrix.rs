// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use alloc::vec;

use crate::{ImageRefMut, FuzzyZero, RGBA8, f64_bound};

/// An edges processing mode used by `convolve_matrix`.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EdgeMode {
    None,
    Duplicate,
    Wrap,
}

/// A convolve matrix used by `convolve_matrix`.
#[derive(Clone, Debug)]
pub struct ConvolveMatrix<'a> {
    x: u32,
    y: u32,
    columns: u32,
    rows: u32,
    data: &'a [f64],
}

impl<'a> ConvolveMatrix<'a> {
    /// Creates a new `ConvolveMatrix`.
    ///
    /// Returns `None` when:
    ///
    /// - `columns` * `rows` != `data.len()`
    /// - `target_x` >= `columns`
    /// - `target_y` >= `rows`
    pub fn new(target_x: u32, target_y: u32, columns: u32, rows: u32, data: &'a [f64]) -> Option<Self> {
        if    (columns * rows) as usize != data.len()
           || target_x >= columns
           || target_y >= rows
        {
            return None;
        }

        Some(ConvolveMatrix {
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
        self.data
    }
}

/// Applies a convolve matrix.
///
/// Input image pixels should have a **premultiplied alpha** when `preserve_alpha=false`.
///
/// # Panics
///
/// When `divisor` is zero.
///
/// # Allocations
///
/// This method will allocate a copy of the `src` image as a back buffer.
pub fn convolve_matrix(
    matrix: ConvolveMatrix,
    divisor: f64,
    bias: f64,
    edge_mode: EdgeMode,
    preserve_alpha: bool,
    src: ImageRefMut,
) {
    assert!(!divisor.is_fuzzy_zero());

    fn bound(min: i32, val: i32, max: i32) -> i32 {
        core::cmp::max(min, core::cmp::min(max, val))
    }

    let width_max = src.width as i32 - 1;
    let height_max = src.height as i32 - 1;

    let mut buf = vec![RGBA8::default(); src.data.len()];
    let mut buf = ImageRefMut::new(&mut buf, src.width, src.height);
    let mut x = 0;
    let mut y = 0;
    for in_p in src.data.iter() {
        let mut new_r = 0.0;
        let mut new_g = 0.0;
        let mut new_b = 0.0;
        let mut new_a = 0.0;
        for oy in 0..matrix.rows() {
            for ox in 0..matrix.columns() {
                let mut tx = x as i32 - matrix.target_x() as i32 + ox as i32;
                let mut ty = y as i32 - matrix.target_y() as i32 + oy as i32;

                match edge_mode {
                    EdgeMode::None => {
                        if tx < 0 || tx > width_max || ty < 0 || ty > height_max {
                            continue;
                        }
                    }
                    EdgeMode::Duplicate => {
                        tx = bound(0, tx, width_max);
                        ty = bound(0, ty, height_max);
                    }
                    EdgeMode::Wrap => {
                        while tx < 0 {
                            tx += src.width as i32;
                        }
                        tx %= src.width as i32;

                        while ty < 0 {
                            ty += src.height as i32;
                        }
                        ty %= src.height as i32;
                    }
                }

                let k = matrix.get(matrix.columns() - ox - 1,
                                   matrix.rows() - oy - 1);

                let p = src.pixel_at(tx as u32, ty as u32);
                new_r += (p.r as f64) / 255.0 * k;
                new_g += (p.g as f64) / 255.0 * k;
                new_b += (p.b as f64) / 255.0 * k;

                if !preserve_alpha {
                    new_a += (p.a as f64) / 255.0 * k;
                }
            }
        }

        if preserve_alpha {
            new_a = in_p.a as f64 / 255.0;
        } else {
            new_a = new_a / divisor + bias;
        }

        let bounded_new_a = f64_bound(0.0, new_a, 1.0);

        let calc = |x| {
            let x = x / divisor + bias * new_a;

            let x = if preserve_alpha {
                f64_bound(0.0, x, 1.0) * bounded_new_a
            } else {
                f64_bound(0.0, x, bounded_new_a)
            };

            (x * 255.0 + 0.5) as u8
        };

        let out_p = buf.pixel_at_mut(x, y);
        out_p.r = calc(new_r);
        out_p.g = calc(new_g);
        out_p.b = calc(new_b);
        out_p.a = (bounded_new_a * 255.0 + 0.5) as u8;

        x += 1;
        if x == src.width {
            x = 0;
            y += 1;
        }
    }

    // Do not use `mem::swap` because `data` referenced via FFI.
    src.data.copy_from_slice(buf.data);
}
