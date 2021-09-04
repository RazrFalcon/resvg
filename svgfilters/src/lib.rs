// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/*!
`svgfilters` provides low-level [SVG filters](https://www.w3.org/TR/SVG11/filters.html)
implementation.

`svgfilters` doesn't implement the whole filters workflow, just operations on raster images.
Filter region calculation, image colors (un)premultiplication, input validation,
filter primitives order, transformations, etc. should be implemented by the caller.

## Implemented filters

- [feColorMatrix](https://www.w3.org/TR/SVG11/filters.html#feColorMatrixElement)
- [feComponentTransfer](https://www.w3.org/TR/SVG11/filters.html#feComponentTransferElement)
- [feComposite](https://www.w3.org/TR/SVG11/filters.html#feCompositeElement)
  Only the arithmetic operator is supported since other one are pretty common
  and should be implemented by the 2D library itself.
- [feConvolveMatrix](https://www.w3.org/TR/SVG11/filters.html#feConvolveMatrixElement)
- [feDiffuseLighting](https://www.w3.org/TR/SVG11/filters.html#feDiffuseLightingElement)
- [feDisplacementMap](https://www.w3.org/TR/SVG11/filters.html#feDisplacementMapElement)
- [feGaussianBlur](https://www.w3.org/TR/SVG11/filters.html#feGaussianBlurElement)
  Box blur and IIR blur variants are available.
- [feMorphology](https://www.w3.org/TR/SVG11/filters.html#feMorphologyElement)
- [feSpecularLighting](https://www.w3.org/TR/SVG11/filters.html#feSpecularLightingElement)
- [feTurbulence](https://www.w3.org/TR/SVG11/filters.html#feTurbulenceElement)

## Unimplemented filters

- [feFlood](https://www.w3.org/TR/SVG11/filters.html#feFloodElement),
  because it's just a simple fill.
- [feImage](https://www.w3.org/TR/SVG11/filters.html#feImageElement),
  because it can be implemented only by a caller.
- [feTile](https://www.w3.org/TR/SVG11/filters.html#feTileElement),
  because it's basically a fill with pattern.
- [feMerge](https://www.w3.org/TR/SVG11/filters.html#feMergeElement),
  because it's just a layer compositing and a 2D library will be faster.
- [feOffset](https://www.w3.org/TR/SVG11/filters.html#feOffsetElement),
  because it's just a layer compositing with offset.

## Performance

The library isn't well optimized yet, but it's mostly allocation free.
Some methods will allocate necessary, temporary buffers which will be reflected in the documentation.
But majority of methods will work on provided buffers.
*/

#![doc(html_root_url = "https://docs.rs/svgfilters/0.2.0")]

#![no_std]

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::too_many_arguments)]

extern crate alloc;

use float_cmp::ApproxEqUlps;
pub use rgb::{RGB8, RGBA8};

mod box_blur;
mod color_matrix;
mod component_transfer;
mod composite;
mod convolve_matrix;
mod displacement_map;
mod iir_blur;
mod lighting;
mod morphology;
mod turbulence;

pub use box_blur::box_blur;
pub use color_matrix::{ColorMatrix, color_matrix};
pub use component_transfer::{TransferFunction, component_transfer};
pub use composite::arithmetic_composite;
pub use convolve_matrix::{ConvolveMatrix, EdgeMode, convolve_matrix};
pub use displacement_map::{ColorChannel, displacement_map};
pub use iir_blur::iir_blur;
pub use lighting::{LightSource, diffuse_lighting, specular_lighting};
pub use morphology::{MorphologyOperator, morphology};
pub use turbulence::turbulence;


/// An image reference.
///
/// Image pixels should be stored in RGBA order.
///
/// Some filters will require premultipled channels, some not.
/// See specific filter documentation for details.
#[derive(Clone, Copy)]
pub struct ImageRef<'a> {
    data: &'a [RGBA8],
    width: u32,
    height: u32,
}

impl<'a> ImageRef<'a> {
    /// Creates a new image reference.
    ///
    /// Doesn't clone the provided data.
    #[inline]
    pub fn new(data: &'a [RGBA8], width: u32, height: u32) -> Self {
        ImageRef { data, width, height }
    }

    #[inline]
    fn alpha_at(&self, x: u32, y: u32) -> i16 {
        self.data[(self.width * y + x) as usize].a as i16
    }
}


/// A mutable `ImageRef` variant.
pub struct ImageRefMut<'a> {
    data: &'a mut [RGBA8],
    width: u32,
    height: u32,
}

impl<'a> ImageRefMut<'a> {
    /// Creates a new mutable image reference.
    ///
    /// Doesn't clone the provided data.
    #[inline]
    pub fn new(data: &'a mut [RGBA8], width: u32, height: u32) -> Self {
        ImageRefMut { data, width, height }
    }

    #[inline]
    fn pixel_at(&self, x: u32, y: u32) -> RGBA8 {
        self.data[(self.width * y + x) as usize]
    }

    #[inline]
    fn pixel_at_mut(&mut self, x: u32, y: u32) -> &mut RGBA8 {
        &mut self.data[(self.width * y + x) as usize]
    }
}


/// Multiplies provided pixels alpha.
pub fn multiply_alpha(data: &mut [RGBA8]) {
    for p in data {
        let a = p.a as f64 / 255.0;
        p.b = (p.b as f64 * a + 0.5) as u8;
        p.g = (p.g as f64 * a + 0.5) as u8;
        p.r = (p.r as f64 * a + 0.5) as u8;
    }
}

/// Demultiplies provided pixels alpha.
pub fn demultiply_alpha(data: &mut [RGBA8]) {
    for p in data {
        let a = p.a as f64 / 255.0;
        p.b = (p.b as f64 / a + 0.5) as u8;
        p.g = (p.g as f64 / a + 0.5) as u8;
        p.r = (p.r as f64 / a + 0.5) as u8;
    }
}


/// Precomputed sRGB to LinearRGB table.
///
/// Since we are storing the result in `u8`, there is no need to compute those
/// values each time. Mainly because it's very expensive.
///
/// ```text
/// if (C_srgb <= 0.04045)
///     C_lin = C_srgb / 12.92;
///  else
///     C_lin = pow((C_srgb + 0.055) / 1.055, 2.4);
/// ```
///
/// Thanks to librsvg for the idea.
const SRGB_TO_LINEAR_RGB_TABLE: &[u8; 256] = &[
    0,   0,   0,   0,   0,   0,  0,    1,   1,   1,   1,   1,   1,   1,   1,   1,
    1,   1,   2,   2,   2,   2,  2,    2,   2,   2,   3,   3,   3,   3,   3,   3,
    4,   4,   4,   4,   4,   5,  5,    5,   5,   6,   6,   6,   6,   7,   7,   7,
    8,   8,   8,   8,   9,   9,  9,   10,  10,  10,  11,  11,  12,  12,  12,  13,
    13,  13,  14,  14,  15,  15,  16,  16,  17,  17,  17,  18,  18,  19,  19,  20,
    20,  21,  22,  22,  23,  23,  24,  24,  25,  25,  26,  27,  27,  28,  29,  29,
    30,  30,  31,  32,  32,  33,  34,  35,  35,  36,  37,  37,  38,  39,  40,  41,
    41,  42,  43,  44,  45,  45,  46,  47,  48,  49,  50,  51,  51,  52,  53,  54,
    55,  56,  57,  58,  59,  60,  61,  62,  63,  64,  65,  66,  67,  68,  69,  70,
    71,  72,  73,  74,  76,  77,  78,  79,  80,  81,  82,  84,  85,  86,  87,  88,
    90,  91,  92,  93,  95,  96,  97,  99, 100, 101, 103, 104, 105, 107, 108, 109,
    111, 112, 114, 115, 116, 118, 119, 121, 122, 124, 125, 127, 128, 130, 131, 133,
    134, 136, 138, 139, 141, 142, 144, 146, 147, 149, 151, 152, 154, 156, 157, 159,
    161, 163, 164, 166, 168, 170, 171, 173, 175, 177, 179, 181, 183, 184, 186, 188,
    190, 192, 194, 196, 198, 200, 202, 204, 206, 208, 210, 212, 214, 216, 218, 220,
    222, 224, 226, 229, 231, 233, 235, 237, 239, 242, 244, 246, 248, 250, 253, 255,
];

/// Precomputed LinearRGB to sRGB table.
///
/// Since we are storing the result in `u8`, there is no need to compute those
/// values each time. Mainly because it's very expensive.
///
/// ```text
/// if (C_lin <= 0.0031308)
///     C_srgb = C_lin * 12.92;
/// else
///     C_srgb = 1.055 * pow(C_lin, 1.0 / 2.4) - 0.055;
/// ```
///
/// Thanks to librsvg for the idea.
const LINEAR_RGB_TO_SRGB_TABLE: &[u8; 256] = &[
    0,  13,  22,  28,  34,  38,  42,  46,  50,  53,  56,  59,  61,  64,  66,  69,
    71,  73,  75,  77,  79,  81,  83,  85,  86,  88,  90,  92,  93,  95,  96,  98,
    99, 101, 102, 104, 105, 106, 108, 109, 110, 112, 113, 114, 115, 117, 118, 119,
    120, 121, 122, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135, 136,
    137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148, 148, 149, 150, 151,
    152, 153, 154, 155, 155, 156, 157, 158, 159, 159, 160, 161, 162, 163, 163, 164,
    165, 166, 167, 167, 168, 169, 170, 170, 171, 172, 173, 173, 174, 175, 175, 176,
    177, 178, 178, 179, 180, 180, 181, 182, 182, 183, 184, 185, 185, 186, 187, 187,
    188, 189, 189, 190, 190, 191, 192, 192, 193, 194, 194, 195, 196, 196, 197, 197,
    198, 199, 199, 200, 200, 201, 202, 202, 203, 203, 204, 205, 205, 206, 206, 207,
    208, 208, 209, 209, 210, 210, 211, 212, 212, 213, 213, 214, 214, 215, 215, 216,
    216, 217, 218, 218, 219, 219, 220, 220, 221, 221, 222, 222, 223, 223, 224, 224,
    225, 226, 226, 227, 227, 228, 228, 229, 229, 230, 230, 231, 231, 232, 232, 233,
    233, 234, 234, 235, 235, 236, 236, 237, 237, 238, 238, 238, 239, 239, 240, 240,
    241, 241, 242, 242, 243, 243, 244, 244, 245, 245, 246, 246, 246, 247, 247, 248,
    248, 249, 249, 250, 250, 251, 251, 251, 252, 252, 253, 253, 254, 254, 255, 255,
];

/// Converts input pixel from sRGB into LinearRGB.
///
/// Provided pixels should have an **unpremultiplied alpha**.
///
/// RGB channels order of the input image doesn't matter, but alpha channel must be the last one.
pub fn into_linear_rgb(data: &mut [RGBA8]) {
    for p in data {
        p.r = SRGB_TO_LINEAR_RGB_TABLE[p.r as usize];
        p.g = SRGB_TO_LINEAR_RGB_TABLE[p.g as usize];
        p.b = SRGB_TO_LINEAR_RGB_TABLE[p.b as usize];
    }
}

/// Converts input pixel from LinearRGB into sRGB.
///
/// Provided pixels should have an **unpremultiplied alpha**.
///
/// RGB channels order of the input image doesn't matter, but alpha channel must be the last one.
pub fn from_linear_rgb(data: &mut [RGBA8]) {
    for p in data {
        p.r = LINEAR_RGB_TO_SRGB_TABLE[p.r as usize];
        p.g = LINEAR_RGB_TO_SRGB_TABLE[p.g as usize];
        p.b = LINEAR_RGB_TO_SRGB_TABLE[p.b as usize];
    }
}


// TODO: https://github.com/rust-lang/rust/issues/44095
#[inline]
fn f64_bound(min: f64, val: f64, max: f64) -> f64 {
    debug_assert!(min.is_finite());
    debug_assert!(val.is_finite());
    debug_assert!(max.is_finite());

    if val > max {
        max
    } else if val < min {
        min
    } else {
        val
    }
}


trait FuzzyEq<Rhs: ?Sized = Self> {
    fn fuzzy_eq(&self, other: &Rhs) -> bool;

    #[inline]
    fn fuzzy_ne(&self, other: &Rhs) -> bool {
        !self.fuzzy_eq(other)
    }
}

trait FuzzyZero: FuzzyEq {
    fn is_fuzzy_zero(&self) -> bool;
}

impl FuzzyEq for f64 {
    #[inline]
    fn fuzzy_eq(&self, other: &f64) -> bool {
        self.approx_eq_ulps(other, 4)
    }
}

impl FuzzyZero for f64 {
    #[inline]
    fn is_fuzzy_zero(&self) -> bool {
        self.fuzzy_eq(&0.0)
    }
}
