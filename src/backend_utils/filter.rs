// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::rc::Rc;

use log::warn;
use usvg::ColorInterpolation as ColorSpace;

use crate::prelude::*;


pub enum Error {
    AllocFailed,
    InvalidRegion,
}


pub trait ImageExt: Sized {
    fn width(&self) -> u32;
    fn height(&self) -> u32;

    fn try_clone(&self) -> Result<Self, Error>;
    fn clip(&mut self, region: ScreenRect);
    fn clear(&mut self);

    fn into_srgb(&mut self);
    fn into_linear_rgb(&mut self);
}


pub struct Image<T: ImageExt> {
    /// Filter primitive result.
    ///
    /// All images have the same size which is equal to the current filter region.
    pub image: Rc<T>,

    /// Image's region that has actual data.
    ///
    /// Region is in global coordinates and not in `image` one.
    ///
    /// Image's content outside this region will be transparent/cleared.
    ///
    /// Currently used only for `feTile`.
    pub region: ScreenRect,

    /// The current color space.
    pub color_space: ColorSpace,
}

impl<T: ImageExt> Image<T> {
    pub fn from_image(image: T, color_space: ColorSpace) -> Self {
        let (w, h) = (image.width(), image.height());
        Image {
            image: Rc::new(image),
            region: ScreenRect::new(0, 0, w, h).unwrap(),
            color_space,
        }
    }

    pub fn into_color_space(self, color_space: ColorSpace) -> Result<Self, Error> {
        if color_space != self.color_space {
            let region = self.region;

            let mut image = self.take()?;

            match color_space {
                ColorSpace::SRGB => image.into_srgb(),
                ColorSpace::LinearRGB => image.into_linear_rgb(),
            }

            Ok(Image {
                image: Rc::new(image),
                region,
                color_space,
            })
        } else {
            Ok(self.clone())
        }
    }

    pub fn take(self) -> Result<T, Error> {
        match Rc::try_unwrap(self.image) {
            Ok(v) => Ok(v),
            Err(v) => v.try_clone(),
        }
    }

    pub fn width(&self) -> u32 {
        self.image.width()
    }

    pub fn height(&self) -> u32 {
        self.image.height()
    }

    pub fn as_ref(&self) -> &T {
        &self.image
    }
}

// Do not use Clone derive because of https://github.com/rust-lang/rust/issues/26925
impl<T: ImageExt> Clone for Image<T> {
    fn clone(&self) -> Self {
        Image {
            image: self.image.clone(),
            region: self.region,
            color_space: self.color_space,
        }
    }
}


pub struct FilterResult<T: ImageExt> {
    pub name: String,
    pub image: Image<T>,
}


pub trait Filter<T: ImageExt> {
    fn apply(
        filter: &usvg::Filter,
        bbox: Option<Rect>,
        ts: &usvg::Transform,
        opt: &Options,
        canvas: &mut T,
    ) {
        let res = Self::_apply(filter, bbox, ts, opt, canvas);

        // Clear on error.
        if res.is_err() {
            canvas.clear();
        }

        match res {
            Ok(_) => {}
            Err(Error::AllocFailed) => {
                warn!(
                    "Memory allocation failed while processing the '{}' filter. Skipped.",
                    filter.id
                );
            }
            Err(Error::InvalidRegion) => {
                warn!("Filter '{}' has an invalid region.", filter.id);
            }
        }
    }

    fn _apply(
        filter: &usvg::Filter,
        bbox: Option<Rect>,
        ts: &usvg::Transform,
        opt: &Options,
        canvas: &mut T,
    ) -> Result<(), Error> {
        let mut results = Vec::new();

        let canvas_rect = ScreenRect::new(0, 0, canvas.width(), canvas.height()).unwrap();
        let region = calc_region(filter, bbox, ts, canvas_rect)?;

        for primitive in &filter.children {
            let cs = primitive.color_interpolation;
            let subregion = calc_subregion(filter, primitive, bbox, region, ts, &results)?;

            let mut result = match primitive.kind {
                usvg::FilterKind::FeBlend(ref fe) => {
                    let input1 = Self::get_input(&fe.input1, region, &results, canvas)?;
                    let input2 = Self::get_input(&fe.input2, region, &results, canvas)?;
                    Self::apply_blend(fe, cs, region, input1, input2)
                }
                usvg::FilterKind::FeFlood(ref fe) => {
                    Self::apply_flood(fe, region)
                }
                usvg::FilterKind::FeGaussianBlur(ref fe) => {
                    let input = Self::get_input(&fe.input, region, &results, canvas)?;
                    Self::apply_blur(fe, filter.primitive_units, cs, bbox, ts, input)
                }
                usvg::FilterKind::FeOffset(ref fe) => {
                    let input = Self::get_input(&fe.input, region, &results, canvas)?;
                    Self::apply_offset(fe, filter.primitive_units, bbox, ts, input)
                }
                usvg::FilterKind::FeComposite(ref fe) => {
                    let input1 = Self::get_input(&fe.input1, region, &results, canvas)?;
                    let input2 = Self::get_input(&fe.input2, region, &results, canvas)?;
                    Self::apply_composite(fe, cs, region, input1, input2)
                }
                usvg::FilterKind::FeMerge(ref fe) => {
                    Self::apply_merge(fe, cs, region, &results, canvas)
                }
                usvg::FilterKind::FeTile(ref fe) => {
                    let input = Self::get_input(&fe.input, region, &results, canvas)?;
                    Self::apply_tile(input, region)
                }
                usvg::FilterKind::FeImage(ref fe) => {
                    Self::apply_image(fe, region, subregion, opt)
                }
            }?;

            if region != subregion {
                // Clip result.

                // TODO: explain
                let subregion2 = if let usvg::FilterKind::FeOffset(..) = primitive.kind {
                    // We do not support clipping on feOffset.
                    region.translate_to(0, 0)
                } else {
                    subregion.translate(-region.x(), -region.y())
                };

                let color_space = result.color_space;
                let mut buffer = result.take()?;
                buffer.clip(subregion2);

                result = Image {
                    image: Rc::new(buffer),
                    region: subregion,
                    color_space,
                };
            }

            results.push(FilterResult {
                name: primitive.result.clone(),
                image: result,
            });
        }

        if let Some(res) = results.pop() {
            Self::apply_to_canvas(res.image, region, canvas)?;
        }

        Ok(())
    }

    fn get_input(
        input: &usvg::FilterInput,
        region: ScreenRect,
        results: &[FilterResult<T>],
        canvas: &T,
    ) -> Result<Image<T>, Error>;

    fn apply_blur(
        fe: &usvg::FeGaussianBlur,
        units: usvg::Units,
        cs: ColorSpace,
        bbox: Option<Rect>,
        ts: &usvg::Transform,
        input: Image<T>,
    ) -> Result<Image<T>, Error>;

    fn apply_offset(
        fe: &usvg::FeOffset,
        units: usvg::Units,
        bbox: Option<Rect>,
        ts: &usvg::Transform,
        input: Image<T>,
    ) -> Result<Image<T>, Error>;

    fn apply_blend(
        fe: &usvg::FeBlend,
        cs: ColorSpace,
        region: ScreenRect,
        input1: Image<T>,
        input2: Image<T>,
    ) -> Result<Image<T>, Error>;

    fn apply_composite(
        fe: &usvg::FeComposite,
        cs: ColorSpace,
        region: ScreenRect,
        input1: Image<T>,
        input2: Image<T>,
    ) -> Result<Image<T>, Error>;

    fn apply_merge(
        fe: &usvg::FeMerge,
        cs: ColorSpace,
        region: ScreenRect,
        results: &[FilterResult<T>],
        canvas: &T,
    ) -> Result<Image<T>, Error>;

    fn apply_flood(
        fe: &usvg::FeFlood,
        region: ScreenRect,
    ) -> Result<Image<T>, Error>;

    fn apply_tile(
        input: Image<T>,
        region: ScreenRect,
    ) -> Result<Image<T>, Error>;

    fn apply_image(
        fe: &usvg::FeImage,
        region: ScreenRect,
        subregion: ScreenRect,
        opt: &Options,
    ) -> Result<Image<T>, Error>;

    fn apply_to_canvas(
        input: Image<T>,
        region: ScreenRect,
        canvas: &mut T,
    ) -> Result<(), Error>;

    fn resolve_std_dev(
        fe: &usvg::FeGaussianBlur,
        units: usvg::Units,
        bbox: Option<Rect>,
        ts: &usvg::Transform,
    ) -> Option<(f64, f64)> {
        // 'A negative value or a value of zero disables the effect of the given filter primitive
        // (i.e., the result is the filter input image).'
        if fe.std_dev_x.is_zero() && fe.std_dev_y.is_zero() {
            return None;
        }

        let (sx, sy) = ts.get_scale();

        let (std_dx, std_dy) = if units == usvg::Units::ObjectBoundingBox {
            let bbox = bbox?;

            (
                fe.std_dev_x.value() * sx * bbox.width(),
                fe.std_dev_y.value() * sy * bbox.height()
            )
        } else {
            (
                fe.std_dev_x.value() * sx,
                fe.std_dev_y.value() * sy
            )
        };

        if std_dx.is_fuzzy_zero() && std_dy.is_fuzzy_zero() {
            None
        } else {
            Some((std_dx, std_dy))
        }
    }

    fn resolve_offset(
        fe: &usvg::FeOffset,
        units: usvg::Units,
        bbox: Option<Rect>,
        ts: &usvg::Transform,
    ) -> Option<(f64, f64)> {
        let (sx, sy) = ts.get_scale();

        let (dx, dy) = if units == usvg::Units::ObjectBoundingBox {
            let bbox = bbox?;

            (
                fe.dx * sx * bbox.width(),
                fe.dy * sy * bbox.height()
            )
        } else {
            (
                fe.dx * sx,
                fe.dy * sy
            )
        };

        if dx.is_fuzzy_zero() && dy.is_fuzzy_zero() {
            None
        } else {
            Some((dx, dy))
        }
    }
}


pub mod blur {
    use rgb::{RGBA8, FromSlice, ComponentSlice};

    struct BlurData {
        width: usize,
        height: usize,
        sigma_x: f64,
        sigma_y: f64,
        steps: usize,
    }

    /// Blurs an input image using IIR Gaussian filter.
    pub fn apply(
        data: &mut [u8],
        width: u32,
        height: u32,
        sigma_x: f64,
        sigma_y: f64,
        steps: u8,
    ) {
        assert_ne!(steps, 0);

        let buf_size = (width * height) as usize;
        let mut buf = vec![0.0; buf_size];
        let buf = &mut buf;

        // We convert number types to prevent redundant types conversion.
        let d = BlurData {
            width: width as usize,
            height: height as usize,
            sigma_x,
            sigma_y,
            steps: steps as usize,
        };

        let alpha_channel = &gaussian_alpha(data, &d, buf);
        gaussian_channel(data, &d, 0, alpha_channel, buf);
        gaussian_channel(data, &d, 1, alpha_channel, buf);
        gaussian_channel(data, &d, 2, alpha_channel, buf);
        set_alpha(alpha_channel, data);
    }

    fn gaussian_alpha(
        data: &[u8],
        d: &BlurData,
        buf: &mut Vec<f64>,
    ) -> Vec<f64> {
        for (i, p) in data.as_rgba().iter().enumerate() {
            buf[i] = p.a as f64;
        }

        gaussianiir2d(d, buf);

        buf.to_owned()
    }

    fn gaussian_channel(
        data: &mut [u8],
        d: &BlurData,
        channel: usize,
        alpha_channel: &[f64],
        buf: &mut Vec<f64>,
    ) {
        let mut i = 0;
        for p in data.as_rgba() {
            let c = p.as_slice()[channel] as f64;
            let a = p.a as f64;

            buf[i] = c * a;

            i += 1;
        }

        gaussianiir2d(d, buf);

        // Normalization of the selected channel according to the alpha channel.
        i = 0;
        for p in data.as_rgba_mut() {
            if alpha_channel[i] > 0.0 {
                let c = buf[i];
                let a = alpha_channel[i];
                p.as_mut_slice()[channel] = (c / a) as u8;
            } else {
                p.a = 0;
            }

            i += 1;
        }
    }

    fn set_alpha(
        alpha_channel: &[f64],
        data: &mut [u8],
    ) {
        let mut i = 0;
        for p in data.as_rgba_mut() {
            let a = alpha_channel[i];
            if a > 0.0 {
                p.a = a as u8;
            } else {
                *p = RGBA8::new(0, 0, 0, 0);
            }

            i += 1;
        }
    }

    /// IIR blur.
    ///
    /// Based on http://www.getreuer.info/home/gaussianiir
    ///
    /// Licensed under 'Simplified BSD License'.
    ///
    ///
    /// Implements the fast Gaussian convolution algorithm of Alvarez and Mazorra,
    /// where the Gaussian is approximated by a cascade of first-order infinite
    /// impulsive response (IIR) filters.  Boundaries are handled with half-sample
    /// symmetric extension.
    ///
    /// Gaussian convolution is approached as approximating the heat equation and
    /// each timestep is performed with an efficient recursive computation.  Using
    /// more steps yields a more accurate approximation of the Gaussian.  A
    /// reasonable default value for `numsteps` is 4.
    ///
    /// Reference:
    /// Alvarez, Mazorra, "Signal and Image Restoration using Shock Filters and
    /// Anisotropic Diffusion," SIAM J. on Numerical Analysis, vol. 31, no. 2,
    /// pp. 590-605, 1994.
    fn gaussianiir2d(
        d: &BlurData,
        buf: &mut Vec<f64>,
    ) {
        // Filter horizontally along each row.
        let (lambda_x, dnu_x) = if d.sigma_x > 0.0 {
            // let (lambda, dnu, boundary_scale) = gen_coefficients(d.sigma_x, d.steps);
            let (lambda, dnu) = gen_coefficients(d.sigma_x, d.steps);

            for y in 0..d.height {
                for _ in 0..d.steps {
                    let idx = d.width * y;
                    // TODO: Blurs right and bottom sides twice for some reasons.
                    // e-filter-002.svg
                    // buf[idx] *= boundary_scale;

                    // Filter rightwards.
                    for x in 1..d.width {
                        buf[idx + x] += dnu * buf[idx + x - 1];
                    }

                    let mut x = d.width - 1;
                    // buf[idx + x] *= boundary_scale;

                    // Filter leftwards.
                    while x > 0 {
                        buf[idx + x - 1] += dnu * buf[idx + x];
                        x -= 1;
                    }
                }
            }

            (lambda, dnu)
        } else {
            (1.0, 1.0)
        };

        // Filter vertically along each column.
        let (lambda_y, dnu_y) = if d.sigma_y > 0.0 {
            // let (lambda, dnu, boundary_scale) = gen_coefficients(d.sigma_y, d.steps);
            let (lambda, dnu) = gen_coefficients(d.sigma_y, d.steps);
            for x in 0..d.width {
                for _ in 0..d.steps {
                    let idx = x;
                    // buf[idx] *= boundary_scale;

                    // Filter downwards.
                    let mut y = d.width;
                    while y < buf.len() {
                        buf[idx + y] += dnu * buf[idx + y - d.width];
                        y += d.width;
                    }

                    y = buf.len() - d.width;
                    // buf[idx + y] *= boundary_scale;

                    // Filter upwards.
                    while y > 0 {
                        buf[idx + y - d.width] += dnu * buf[idx + y];
                        y -= d.width;
                    }
                }
            }

            (lambda, dnu)
        } else {
            (1.0, 1.0)
        };

        let post_scale =
            ((dnu_x * dnu_y).sqrt() / (lambda_x * lambda_y).sqrt()).powi(2 * d.steps as i32);

        buf.iter_mut().for_each(|v| *v *= post_scale);
    }

    fn gen_coefficients(sigma: f64, steps: usize) -> (f64, f64) {
        let lambda = (sigma * sigma) / (2.0 * steps as f64);
        let dnu = (1.0 + 2.0 * lambda - (1.0 + 4.0 * lambda).sqrt()) / (2.0 * lambda);
        // let boundary_scale = (1.0 / (1.0 - dnu)) / 2.0;

        // (lambda, dnu, boundary_scale)
        (lambda, dnu)
    }
}

fn calc_region(
    filter: &usvg::Filter,
    bbox: Option<Rect>,
    ts: &usvg::Transform,
    canvas_rect: ScreenRect,
) -> Result<ScreenRect, Error> {
    let path = utils::rect_to_path(filter.rect);

    let region_ts = if filter.units == usvg::Units::ObjectBoundingBox {
        let bbox = bbox.ok_or(Error::InvalidRegion)?;
        let bbox_ts = usvg::Transform::from_bbox(bbox);
        let mut ts2 = ts.clone();
        ts2.append(&bbox_ts);
        ts2
    } else {
        *ts
    };

    let region = utils::path_bbox(&path, None, Some(region_ts))
        .ok_or_else(|| Error::InvalidRegion)?
        .to_screen_rect()
        .fit_to_rect(canvas_rect);

    Ok(region)
}

/// Returns filter primitive region.
fn calc_subregion<T: ImageExt>(
    filter: &usvg::Filter,
    primitive: &usvg::FilterPrimitive,
    bbox: Option<Rect>,
    filter_region: ScreenRect,
    ts: &usvg::Transform,
    results: &[FilterResult<T>],
) -> Result<ScreenRect, Error> {
    // TODO: rewrite/simplify/explain/whatever

    let region = match primitive.kind {
        usvg::FilterKind::FeOffset(ref fe) => {
            // `feOffset` inherits it's region from the input.
            match fe.input {
                usvg::FilterInput::Reference(ref name) => {
                    match results.iter().rev().find(|v| v.name == *name) {
                        Some(ref res) => res.image.region,
                        None => filter_region,
                    }
                }
                _ => {
                    filter_region
                }
            }
        }
        usvg::FilterKind::FeImage(..) => {
            // `feImage` uses the object bbox.
            if filter.primitive_units == usvg::Units::ObjectBoundingBox {
                let bbox = bbox.ok_or(Error::InvalidRegion)?;

                // TODO: wrong
                let ts_bbox = Rect::new(ts.e, ts.f, ts.a, ts.d).unwrap();

                let r = Rect::new(
                    primitive.x.unwrap_or(0.0),
                    primitive.y.unwrap_or(0.0),
                    primitive.width.unwrap_or(1.0),
                    primitive.height.unwrap_or(1.0),
                ).ok_or_else(|| Error::InvalidRegion)?;

                let r = r
                    .bbox_transform(bbox)
                    .bbox_transform(ts_bbox)
                    .to_screen_rect();

                return Ok(r);
            } else {
                filter_region
            }
        }
        _ => filter_region,
    };

    // TODO: Wrong! Does not account rotate and skew.
    let subregion = if filter.primitive_units == usvg::Units::ObjectBoundingBox {
        let subregion_bbox = Rect::new(
            primitive.x.unwrap_or(0.0),
            primitive.y.unwrap_or(0.0),
            primitive.width.unwrap_or(1.0),
            primitive.height.unwrap_or(1.0),
        ).ok_or_else(|| Error::InvalidRegion)?;

        region.to_rect().bbox_transform(subregion_bbox)
    } else {
        let (dx, dy) = ts.get_translate();
        let (sx, sy) = ts.get_scale();
        Rect::new(
            primitive.x.map(|n| n * sx + dx).unwrap_or(region.x() as f64),
            primitive.y.map(|n| n * sy + dy).unwrap_or(region.y() as f64),
            primitive.width.map(|n| n * sx).unwrap_or(region.width() as f64),
            primitive.height.map(|n| n * sy).unwrap_or(region.height() as f64),
        ).ok_or_else(|| Error::InvalidRegion)?
    };

    Ok(subregion.to_screen_rect())
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
pub const SRGB_TO_LINEAR_RGB_TABLE: &[u8; 256] = &[
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
pub const LINEAR_RGB_TO_SRGB_TABLE: &[u8; 256] = &[
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
