// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::rc::Rc;

use log::warn;
use usvg::ColorInterpolation as ColorSpace;

use crate::prelude::*;


pub enum Error {
    #[allow(dead_code)] // Not used by raqote-backend.
    AllocFailed,
    InvalidRegion,
    NoResults,
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


pub struct FilterInputs<'a, T: ImageExt> {
    pub source: &'a T,
    pub background: Option<&'a T>,
    pub fill_paint: Option<&'a T>,
    pub stroke_paint: Option<&'a T>,
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
        tree: &usvg::Tree,
        background: Option<&T>,
        fill_paint: Option<&T>,
        stroke_paint: Option<&T>,
        canvas: &mut T,
    ) {
        let res = {
            let inputs = FilterInputs {
                source: canvas,
                background,
                fill_paint,
                stroke_paint,
            };

            Self::_apply(filter, &inputs, bbox, ts, opt, tree)
        };

        let res = res.and_then(|(image, region)| Self::apply_to_canvas(image, region, canvas));

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
            Err(Error::NoResults) => {}
        }
    }

    fn _apply(
        filter: &usvg::Filter,
        inputs: &FilterInputs<T>,
        bbox: Option<Rect>,
        ts: &usvg::Transform,
        opt: &Options,
        tree: &usvg::Tree,
    ) -> Result<(Image<T>, ScreenRect), Error> {
        let mut results = Vec::new();
        let region = calc_region(filter, bbox, ts, inputs.source)?;

        for primitive in &filter.children {
            let cs = primitive.color_interpolation;
            let subregion = calc_subregion(filter, primitive, bbox, region, ts, &results)?;

            let mut result = match primitive.kind {
                usvg::FilterKind::FeBlend(ref fe) => {
                    let input1 = Self::get_input(&fe.input1, region, inputs, &results)?;
                    let input2 = Self::get_input(&fe.input2, region, inputs, &results)?;
                    Self::apply_blend(fe, cs, region, input1, input2)
                }
                usvg::FilterKind::FeFlood(ref fe) => {
                    Self::apply_flood(fe, region)
                }
                usvg::FilterKind::FeGaussianBlur(ref fe) => {
                    let input = Self::get_input(&fe.input, region, inputs, &results)?;
                    Self::apply_blur(fe, filter.primitive_units, cs, bbox, ts, input)
                }
                usvg::FilterKind::FeOffset(ref fe) => {
                    let input = Self::get_input(&fe.input, region, inputs, &results)?;
                    Self::apply_offset(fe, filter.primitive_units, bbox, ts, input)
                }
                usvg::FilterKind::FeComposite(ref fe) => {
                    let input1 = Self::get_input(&fe.input1, region, inputs, &results)?;
                    let input2 = Self::get_input(&fe.input2, region, inputs, &results)?;
                    Self::apply_composite(fe, cs, region, input1, input2)
                }
                usvg::FilterKind::FeMerge(ref fe) => {
                    Self::apply_merge(fe, cs, region, inputs, &results)
                }
                usvg::FilterKind::FeTile(ref fe) => {
                    let input = Self::get_input(&fe.input, region, inputs, &results)?;
                    Self::apply_tile(input, region)
                }
                usvg::FilterKind::FeImage(ref fe) => {
                    Self::apply_image(fe, region, subregion, opt, tree, ts)
                }
                usvg::FilterKind::FeComponentTransfer(ref fe) => {
                    let input = Self::get_input(&fe.input, region, inputs, &results)?;
                    Self::apply_component_transfer(fe, cs, input)
                }
                usvg::FilterKind::FeColorMatrix(ref fe) => {
                    let input = Self::get_input(&fe.input, region, inputs, &results)?;
                    Self::apply_color_matrix(fe, cs, input)
                }
                usvg::FilterKind::FeConvolveMatrix(ref fe) => {
                    let input = Self::get_input(&fe.input, region, inputs, &results)?;
                    Self::apply_convolve_matrix(fe, cs, input)
                }
                usvg::FilterKind::FeMorphology(ref fe) => {
                    let input = Self::get_input(&fe.input, region, inputs, &results)?;
                    Self::apply_morphology(fe, filter.primitive_units, cs, bbox, ts, input)
                }
                usvg::FilterKind::FeDisplacementMap(ref fe) => {
                    let input1 = Self::get_input(&fe.input1, region, inputs, &results)?;
                    let input2 = Self::get_input(&fe.input2, region, inputs, &results)?;
                    Self::apply_displacement_map(fe, region, filter.primitive_units, cs, bbox, ts, input1, input2)
                }
                usvg::FilterKind::FeTurbulence(ref fe) => {
                    Self::apply_turbulence(fe, region, cs, ts)
                }
                usvg::FilterKind::FeDiffuseLighting(ref fe) => {
                    let input = Self::get_input(&fe.input, region, inputs, &results)?;
                    Self::apply_diffuse_lighting(fe, region, cs, ts, input)
                }
                usvg::FilterKind::FeSpecularLighting(ref fe) => {
                    let input = Self::get_input(&fe.input, region, inputs, &results)?;
                    Self::apply_specular_lighting(fe, region, cs, ts, input)
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
            Ok((res.image, region))
        } else {
            Err(Error::NoResults)
        }
    }

    fn get_input(
        kind: &usvg::FilterInput,
        region: ScreenRect,
        inputs: &FilterInputs<T>,
        results: &[FilterResult<T>],
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
        inputs: &FilterInputs<T>,
        results: &[FilterResult<T>],
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
        tree: &usvg::Tree,
        ts: &usvg::Transform,
    ) -> Result<Image<T>, Error>;

    fn apply_component_transfer(
        fe: &usvg::FeComponentTransfer,
        cs: ColorSpace,
        input: Image<T>,
    ) -> Result<Image<T>, Error>;

    fn apply_color_matrix(
        fe: &usvg::FeColorMatrix,
        cs: ColorSpace,
        input: Image<T>,
    ) -> Result<Image<T>, Error>;

    fn apply_convolve_matrix(
        fe: &usvg::FeConvolveMatrix,
        cs: ColorSpace,
        input: Image<T>,
    ) -> Result<Image<T>, Error>;

    fn apply_morphology(
        fe: &usvg::FeMorphology,
        units: usvg::Units,
        cs: ColorSpace,
        bbox: Option<Rect>,
        ts: &usvg::Transform,
        input: Image<T>,
    ) -> Result<Image<T>, Error>;

    fn apply_displacement_map(
        fe: &usvg::FeDisplacementMap,
        region: ScreenRect,
        units: usvg::Units,
        cs: ColorSpace,
        bbox: Option<Rect>,
        ts: &usvg::Transform,
        input1: Image<T>,
        input2: Image<T>,
    ) -> Result<Image<T>, Error>;

    fn apply_turbulence(
        fe: &usvg::FeTurbulence,
        region: ScreenRect,
        cs: ColorSpace,
        ts: &usvg::Transform,
    ) -> Result<Image<T>, Error>;

    fn apply_diffuse_lighting(
        fe: &usvg::FeDiffuseLighting,
        region: ScreenRect,
        cs: ColorSpace,
        ts: &usvg::Transform,
        input: Image<T>,
    ) -> Result<Image<T>, Error>;

    fn apply_specular_lighting(
        fe: &usvg::FeSpecularLighting,
        region: ScreenRect,
        cs: ColorSpace,
        ts: &usvg::Transform,
        input: Image<T>,
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
    ) -> Option<(f64, f64, bool)> {
        // 'A negative value or a value of zero disables the effect of the given filter primitive
        // (i.e., the result is the filter input image).'
        if fe.std_dev_x.is_zero() && fe.std_dev_y.is_zero() {
            return None;
        }

        let (std_dx, std_dy) = Self::scale_coordinates(
            fe.std_dev_x.value(), fe.std_dev_y.value(), units, bbox, ts,
        )?;
        if std_dx.is_fuzzy_zero() && std_dy.is_fuzzy_zero() {
            None
        } else {
            const BLUR_SIGMA_THRESHOLD: f64 = 2.0;
            // Check that the current feGaussianBlur filter can be applied using a box blur.
            let box_blur =    std_dx >= BLUR_SIGMA_THRESHOLD
                           && std_dy >= BLUR_SIGMA_THRESHOLD;

            Some((std_dx, std_dy, box_blur))
        }
    }

    fn scale_coordinates(
        x: f64,
        y: f64,
        units: usvg::Units,
        bbox: Option<Rect>,
        ts: &usvg::Transform,
    ) -> Option<(f64, f64)> {
        let (sx, sy) = ts.get_scale();
        if units == usvg::Units::ObjectBoundingBox {
            let bbox = bbox?;
            Some((x * sx * bbox.width(), y * sy * bbox.height()))
        } else {
            Some((x * sx, y * sy))
        }
    }
}

/// An IIR blur.
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
pub mod iir_blur {
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
    ) {
        let buf_size = (width * height) as usize;
        let mut buf = vec![0.0; buf_size];
        let buf = &mut buf;

        // We convert number types to prevent redundant types conversion.
        let d = BlurData {
            width: width as usize,
            height: height as usize,
            sigma_x,
            sigma_y,
            steps: 4,
        };

        gaussian_channel(data, &d, 0, buf);
        gaussian_channel(data, &d, 1, buf);
        gaussian_channel(data, &d, 2, buf);
        gaussian_channel(data, &d, 3, buf);
    }

    fn gaussian_channel(
        data: &mut [u8],
        d: &BlurData,
        channel: usize,
        buf: &mut Vec<f64>,
    ) {
        for i in 0..data.len() / 4 {
            buf[i] = data[i * 4 + channel] as f64 / 255.0;
        }

        gaussianiir2d(d, buf);

        for i in 0..data.len() / 4 {
            data[i * 4 + channel] = (buf[i] * 255.0) as u8;
        }
    }

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

/// A box blur.
///
/// Based on https://github.com/fschutt/fastblur
pub mod box_blur {
    use std::cmp;
    use rgb::{RGBA8, FromSlice};

    pub fn apply(
        data: &mut [u8],
        width: u32,
        height: u32,
        sigma_x: f64,
        sigma_y: f64,
    ) {
        let boxes_horz = create_box_gauss(sigma_x as f32, 5);
        let boxes_vert = create_box_gauss(sigma_y as f32, 5);
        let mut backbuf = data.to_vec();

        for (box_size_horz, box_size_vert) in boxes_horz.iter().zip(boxes_vert.iter()) {
            let radius_horz = ((box_size_horz - 1) / 2) as usize;
            let radius_vert = ((box_size_vert - 1) / 2) as usize;
            // We don't care if an input image is RGBA or BGRA
            // since all channels will be processed the same.
            box_blur(
                backbuf.as_rgba_mut(), data.as_rgba_mut(),
                width as usize, height as usize,
                radius_horz, radius_vert,
            );
        }
    }

    /// If there is no valid size (e.g. radius is negative), returns `vec![1; len]`
    /// which would translate to blur radius of 0
    #[inline]
    fn create_box_gauss(
        sigma: f32,
        n: usize,
    ) -> Vec<i32> {
        if sigma > 0.0 {
            let n_float = n as f32;

            // Ideal averaging filter width
            let w_ideal = (12.0 * sigma * sigma / n_float).sqrt() + 1.0;
            let mut wl = w_ideal.floor() as i32;
            if wl % 2 == 0 {
                wl -= 1;
            }

            let wu = wl + 2;

            let wl_float = wl as f32;
            let m_ideal =
                (  12.0 * sigma * sigma
                 - n_float * wl_float * wl_float
                 - 4.0 * n_float * wl_float
                 - 3.0 * n_float)
                / (-4.0 * wl_float - 4.0);
            let m = m_ideal.round() as usize;

            let mut sizes = Vec::new();
            for i in 0..n {
                if i < m {
                    sizes.push(wl);
                } else {
                    sizes.push(wu);
                }
            }

            sizes
        } else {
            vec![1; n]
        }
    }

    /// Needs 2x the same image
    #[inline]
    fn box_blur(
        backbuf: &mut [RGBA8],
        frontbuf: &mut [RGBA8],
        width: usize,
        height: usize,
        blur_radius_horz: usize,
        blur_radius_vert: usize,
    ) {
        box_blur_vert(frontbuf, backbuf, width, height, blur_radius_vert);
        box_blur_horz(backbuf, frontbuf, width, height, blur_radius_horz);
    }

    #[inline]
    fn box_blur_vert(
        backbuf: &[RGBA8],
        frontbuf: &mut [RGBA8],
        width: usize,
        height: usize,
        blur_radius: usize,
    ) {
        if blur_radius == 0 {
            frontbuf.copy_from_slice(backbuf);
            return;
        }

        let iarr = 1.0 / (blur_radius + blur_radius + 1) as f32;
        let blur_radius_prev = blur_radius as isize - height as isize;
        let blur_radius_next = blur_radius as isize + 1;

        for i in 0..width {
            let col_start = i; //inclusive
            let col_end = i + width * (height - 1); //inclusive
            let mut ti = i;
            let mut li = ti;
            let mut ri = ti + blur_radius * width;

            let fv: RGBA8 = [0,0,0,0].into();
            let lv: RGBA8 = [0,0,0,0].into();

            let mut val_r = blur_radius_next * (fv.r as isize);
            let mut val_g = blur_radius_next * (fv.g as isize);
            let mut val_b = blur_radius_next * (fv.b as isize);
            let mut val_a = blur_radius_next * (fv.a as isize);

            // Get the pixel at the specified index, or the first pixel of the column
            // if the index is beyond the top edge of the image
            let get_top = |i| {
                if i < col_start {
                    fv
                } else {
                    backbuf[i]
                }
            };

            // Get the pixel at the specified index, or the last pixel of the column
            // if the index is beyond the bottom edge of the image
            let get_bottom = |i| {
                if i > col_end {
                    lv
                } else {
                    backbuf[i]
                }
            };

            for j in 0..cmp::min(blur_radius, height) {
                let bb = backbuf[ti + j * width];
                val_r += bb.r as isize;
                val_g += bb.g as isize;
                val_b += bb.b as isize;
                val_a += bb.a as isize;
            }
            if blur_radius > height {
                val_r += blur_radius_prev * (lv.r as isize);
                val_g += blur_radius_prev * (lv.g as isize);
                val_b += blur_radius_prev * (lv.b as isize);
                val_a += blur_radius_prev * (lv.a as isize);
            }

            for _ in 0..cmp::min(height, blur_radius + 1) {
                let bb = get_bottom(ri);
                ri += width;
                val_r += sub(bb.r, fv.r);
                val_g += sub(bb.g, fv.g);
                val_b += sub(bb.b, fv.b);
                val_a += sub(bb.a, fv.a);

                frontbuf[ti] = [
                    round(val_r as f32 * iarr) as u8,
                    round(val_g as f32 * iarr) as u8,
                    round(val_b as f32 * iarr) as u8,
                    round(val_a as f32 * iarr) as u8,
                ].into();
                ti += width;
            }

            if height <= blur_radius {
                // otherwise `(height - blur_radius)` will underflow
                continue;
            }

            for _ in (blur_radius + 1)..(height - blur_radius) {
                let bb1 = backbuf[ri];
                ri += width;
                let bb2 = backbuf[li];
                li += width;

                val_r += sub(bb1.r, bb2.r);
                val_g += sub(bb1.g, bb2.g);
                val_b += sub(bb1.b, bb2.b);
                val_a += sub(bb1.a, bb2.a);

                frontbuf[ti] = [
                    round(val_r as f32 * iarr) as u8,
                    round(val_g as f32 * iarr) as u8,
                    round(val_b as f32 * iarr) as u8,
                    round(val_a as f32 * iarr) as u8,
                ].into();
                ti += width;
            }

            for _ in 0..cmp::min(height - blur_radius - 1, blur_radius) {
                let bb = get_top(li);
                li += width;

                val_r += sub(lv.r, bb.r);
                val_g += sub(lv.g, bb.g);
                val_b += sub(lv.b, bb.b);
                val_a += sub(lv.a, bb.a);

                frontbuf[ti] = [
                    round(val_r as f32 * iarr) as u8,
                    round(val_g as f32 * iarr) as u8,
                    round(val_b as f32 * iarr) as u8,
                    round(val_a as f32 * iarr) as u8,
                ].into();
                ti += width;
            }
        }
    }

    #[inline]
    fn box_blur_horz(
        backbuf: &[RGBA8],
        frontbuf: &mut [RGBA8],
        width: usize,
        height: usize,
        blur_radius: usize,
    ) {
        if blur_radius == 0 {
            frontbuf.copy_from_slice(backbuf);
            return;
        }

        let iarr = 1.0 / (blur_radius + blur_radius + 1) as f32;
        let blur_radius_prev = blur_radius as isize - width as isize;
        let blur_radius_next = blur_radius as isize + 1;

        for i in 0..height {
            let row_start = i * width; // inclusive
            let row_end = (i + 1) * width - 1; // inclusive
            let mut ti = i * width; // VERTICAL: $i;
            let mut li = ti;
            let mut ri = ti + blur_radius;

            let fv: RGBA8 = [0,0,0,0].into();
            let lv: RGBA8 = [0,0,0,0].into();

            let mut val_r = blur_radius_next * (fv.r as isize);
            let mut val_g = blur_radius_next * (fv.g as isize);
            let mut val_b = blur_radius_next * (fv.b as isize);
            let mut val_a = blur_radius_next * (fv.a as isize);

            // Get the pixel at the specified index, or the first pixel of the row
            // if the index is beyond the left edge of the image
            let get_left = |i| {
                if i < row_start {
                    fv
                } else {
                    backbuf[i]
                }
            };

            // Get the pixel at the specified index, or the last pixel of the row
            // if the index is beyond the right edge of the image
            let get_right = |i| {
                if i > row_end {
                    lv
                } else {
                    backbuf[i]
                }
            };

            for j in 0..cmp::min(blur_radius, width) {
                let bb = backbuf[ti + j]; // VERTICAL: ti + j * width
                val_r += bb.r as isize;
                val_g += bb.g as isize;
                val_b += bb.b as isize;
                val_a += bb.a as isize;
            }
            if blur_radius > width {
                val_r += blur_radius_prev * (lv.r as isize);
                val_g += blur_radius_prev * (lv.g as isize);
                val_b += blur_radius_prev * (lv.b as isize);
                val_a += blur_radius_prev * (lv.a as isize);
            }

            // Process the left side where we need pixels from beyond the left edge
            for _ in 0..cmp::min(width, blur_radius + 1) {
                let bb = get_right(ri);
                ri += 1;
                val_r += sub(bb.r, fv.r);
                val_g += sub(bb.g, fv.g);
                val_b += sub(bb.b, fv.b);
                val_a += sub(bb.a, fv.a);

                frontbuf[ti] = [
                    round(val_r as f32 * iarr) as u8,
                    round(val_g as f32 * iarr) as u8,
                    round(val_b as f32 * iarr) as u8,
                    round(val_a as f32 * iarr) as u8,
                ].into();
                ti += 1; // VERTICAL : ti += width, same with the other areas
            }

            if width <= blur_radius {
                // otherwise `(width - blur_radius)` will underflow
                continue;
            }

            // Process the middle where we know we won't bump into borders
            // without the extra indirection of get_left/get_right. This is faster.
            for _ in (blur_radius + 1)..(width - blur_radius) {
                let bb1 = backbuf[ri];
                ri += 1;
                let bb2 = backbuf[li];
                li += 1;

                val_r += sub(bb1.r, bb2.r);
                val_g += sub(bb1.g, bb2.g);
                val_b += sub(bb1.b, bb2.b);
                val_a += sub(bb1.a, bb2.a);

                frontbuf[ti] = [
                    round(val_r as f32 * iarr) as u8,
                    round(val_g as f32 * iarr) as u8,
                    round(val_b as f32 * iarr) as u8,
                    round(val_a as f32 * iarr) as u8,
                ].into();
                ti += 1;
            }

            // Process the right side where we need pixels from beyond the right edge
            for _ in 0..cmp::min(width - blur_radius - 1, blur_radius) {
                let bb = get_left(li);
                li += 1;

                val_r += sub(lv.r, bb.r);
                val_g += sub(lv.g, bb.g);
                val_b += sub(lv.b, bb.b);
                val_a += sub(lv.a, bb.a);

                frontbuf[ti] = [
                    round(val_r as f32 * iarr) as u8,
                    round(val_g as f32 * iarr) as u8,
                    round(val_b as f32 * iarr) as u8,
                    round(val_a as f32 * iarr) as u8,
                ].into();
                ti += 1;
            }
        }
    }

    /// Fast rounding for x <= 2^23.
    /// This is orders of magnitude faster than built-in rounding intrinsic.
    ///
    /// Source: https://stackoverflow.com/a/42386149/585725
    #[inline]
    fn round(mut x: f32) -> f32 {
        x += 12582912.0;
        x -= 12582912.0;
        x
    }

    #[inline]
    fn sub(c1: u8, c2: u8) -> isize {
        c1 as isize - c2 as isize
    }
}

pub mod color_matrix {
    use super::*;

    #[inline]
    fn to_normalized_components(pixel: rgb::alt::BGRA8) -> (f64, f64, f64, f64) {
        (pixel.r as f64 / 255.0,
         pixel.g as f64 / 255.0,
         pixel.b as f64 / 255.0,
         pixel.a as f64 / 255.0)
    }

    #[inline]
    fn from_normalized(c: f64) -> u8 {
        (f64_bound(0.0, c, 1.0) * 255.0) as u8
    }

    pub fn apply(
        kind: &usvg::FeColorMatrixKind,
        data: &mut [rgb::alt::BGRA8],
    ) {
        match kind {
            usvg::FeColorMatrixKind::Matrix(ref m) => {
                for pixel in data {
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
            usvg::FeColorMatrixKind::Saturate(v) => {
                let v = v.value();
                let m = [
                    0.213 + 0.787 * v, 0.715 - 0.715 * v, 0.072 - 0.072 * v,
                    0.213 - 0.213 * v, 0.715 + 0.285 * v, 0.072 - 0.072 * v,
                    0.213 - 0.213 * v, 0.715 - 0.715 * v, 0.072 + 0.928 * v,
                ];

                for pixel in data {
                    let (r, g, b, _) = to_normalized_components(*pixel);

                    let new_r = r * m[0] + g * m[1] + b * m[2];
                    let new_g = r * m[3] + g * m[4] + b * m[5];
                    let new_b = r * m[6] + g * m[7] + b * m[8];

                    pixel.r = from_normalized(new_r);
                    pixel.g = from_normalized(new_g);
                    pixel.b = from_normalized(new_b);
                }
            }
            usvg::FeColorMatrixKind::HueRotate(angle) => {
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

                for pixel in data {
                    let (r, g, b, _) = to_normalized_components(*pixel);

                    let new_r = r * m[0] + g * m[1] + b * m[2];
                    let new_g = r * m[3] + g * m[4] + b * m[5];
                    let new_b = r * m[6] + g * m[7] + b * m[8];

                    pixel.r = from_normalized(new_r);
                    pixel.g = from_normalized(new_g);
                    pixel.b = from_normalized(new_b);
                }
            }
            usvg::FeColorMatrixKind::LuminanceToAlpha => {
                for pixel in data {
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
}

pub mod convolve_matrix {
    use super::*;

    pub fn apply(
        fe: &usvg::FeConvolveMatrix,
        width: u32,
        height: u32,
        data: &mut [rgb::alt::BGRA8],
    ) {
        fn bound(min: i32, val: i32, max: i32) -> i32 {
            std::cmp::max(min, std::cmp::min(max, val))
        }

        let width_max = width as i32 - 1;
        let height_max = height as i32 - 1;
        let index_from_pos = |x: u32, y: u32| (y * width + x) as usize;

        let mut buf = vec![rgb::alt::BGRA8::default(); data.len()];
        let mut x = 0;
        let mut y = 0;
        for in_p in data.iter() {
            let mut new_r = 0.0;
            let mut new_g = 0.0;
            let mut new_b = 0.0;
            let mut new_a = 0.0;
            for oy in 0..fe.matrix.rows() {
                for ox in 0..fe.matrix.columns() {
                    let mut tx = x as i32 - fe.matrix.target_x() as i32 + ox as i32;
                    let mut ty = y as i32 - fe.matrix.target_y() as i32 + oy as i32;

                    match fe.edge_mode {
                        usvg::FeEdgeMode::None => {
                            if tx < 0 || tx > width_max || ty < 0 || ty > height_max {
                                continue;
                            }
                        }
                        usvg::FeEdgeMode::Duplicate => {
                            tx = bound(0, tx, width_max);
                            ty = bound(0, ty, height_max);
                        }
                        usvg::FeEdgeMode::Wrap => {
                            while tx < 0 {
                                tx += width as i32;
                            }
                            tx %= width as i32;

                            while ty < 0 {
                                ty += height as i32;
                            }
                            ty %= height as i32;
                        }
                    }

                    let k = fe.matrix.get(fe.matrix.columns() - ox - 1,
                                          fe.matrix.rows() - oy - 1);

                    let p = data[index_from_pos(tx as u32, ty as u32)];
                    new_r += (p.r as f64) / 255.0 * k;
                    new_g += (p.g as f64) / 255.0 * k;
                    new_b += (p.b as f64) / 255.0 * k;

                    if !fe.preserve_alpha {
                        new_a += (p.a as f64) / 255.0 * k;
                    }
                }
            }

            if fe.preserve_alpha {
                new_a = in_p.a as f64 / 255.0;
            } else {
                new_a = new_a / fe.divisor.value() + fe.bias;
            }

            let bounded_new_a = f64_bound(0.0, new_a, 1.0);

            let calc = |x| {
                let x = x / fe.divisor.value() + fe.bias * new_a;

                let x = if fe.preserve_alpha {
                    f64_bound(0.0, x, 1.0) * bounded_new_a
                } else {
                    f64_bound(0.0, x, bounded_new_a)
                };

                (x * 255.0 + 0.5) as u8
            };

            let out_p = &mut buf[index_from_pos(x, y)];
            out_p.r = calc(new_r);
            out_p.g = calc(new_g);
            out_p.b = calc(new_b);
            out_p.a = (bounded_new_a * 255.0 + 0.5) as u8;

            x += 1;
            if x == width {
                x = 0;
                y += 1;
            }
        }

        // Do not use `mem::swap` because `data` referenced via FFI.
        for i in 0..data.len() {
            data[i] = buf[i];
        }
    }
}

pub mod morphology {
    use super::*;

    pub fn apply(
        operator: usvg::FeMorphologyOperator,
        rx: f64,
        ry: f64,
        width: u32,
        height: u32,
        data: &mut [rgb::alt::BGRA8],
    ) {
        if !(rx > 0.0 && ry > 0.0) {
            // Reset to transparent black.
            for i in 0..data.len() {
                data[i] = rgb::alt::BGRA8::default();
            }

            return;
        }

        // No point in making matrix larger than image.
        let columns = std::cmp::min(rx as u32 * 2, width);
        let rows    = std::cmp::min(ry as u32 * 2, height);
        let target_x = (columns as f32 / 2.0).floor() as u32;
        let target_y = (rows as f32 / 2.0).floor() as u32;

        let width_max = width as i32 - 1;
        let height_max = height as i32 - 1;
        let index_from_pos = |x: u32, y: u32| (y * width + x) as usize;

        let mut buf = vec![rgb::alt::BGRA8::default(); data.len()];
        let mut x = 0;
        let mut y = 0;
        for _ in data.iter() {
            let mut new_p = rgb::alt::BGRA8::default();
            if operator == usvg::FeMorphologyOperator::Erode {
                new_p.r = 255;
                new_p.g = 255;
                new_p.b = 255;
                new_p.a = 255;
            }

            for oy in 0..rows {
                for ox in 0..columns {
                    let tx = x as i32 - target_x as i32 + ox as i32;
                    let ty = y as i32 - target_y as i32 + oy as i32;

                    if tx < 0 || tx > width_max || ty < 0 || ty > height_max {
                        continue;
                    }

                    let p = data[index_from_pos(tx as u32, ty as u32)];
                    if operator == usvg::FeMorphologyOperator::Erode {
                        new_p.r = std::cmp::min(p.r, new_p.r);
                        new_p.g = std::cmp::min(p.g, new_p.g);
                        new_p.b = std::cmp::min(p.b, new_p.b);
                        new_p.a = std::cmp::min(p.a, new_p.a);
                    } else {
                        new_p.r = std::cmp::max(p.r, new_p.r);
                        new_p.g = std::cmp::max(p.g, new_p.g);
                        new_p.b = std::cmp::max(p.b, new_p.b);
                        new_p.a = std::cmp::max(p.a, new_p.a);
                    }
                }
            }

            buf[index_from_pos(x, y)] = new_p;

            x += 1;
            if x == width {
                x = 0;
                y += 1;
            }
        }

        // Do not use `mem::swap` because `data` referenced via FFI.
        for i in 0..data.len() {
            data[i] = buf[i];
        }
    }
}

pub mod displacement_map {
    use super::*;

    pub fn apply(
        x_channel_selector: usvg::ColorChannel,
        y_channel_selector: usvg::ColorChannel,
        width: u32,
        height: u32,
        sx: f64,
        sy: f64,
        src: &[rgb::alt::BGRA8],
        displace: &[rgb::alt::BGRA8],
        dest: &mut [rgb::alt::BGRA8],
    ) {
        let rect = ScreenRect::new(0, 0, width, height).unwrap();

        let mut x = 0;
        let mut y = 0;
        for pixel in displace {
            let calc_offset = |channel| {
                let c = match channel {
                    usvg::ColorChannel::R => pixel.r,
                    usvg::ColorChannel::G => pixel.g,
                    usvg::ColorChannel::B => pixel.b,
                    usvg::ColorChannel::A => pixel.a,
                };

                c as f64 / 255.0 - 0.5
            };

            let dx = calc_offset(x_channel_selector);
            let dy = calc_offset(y_channel_selector);
            let ox = (x as f64 + dx * sx).round() as i32;
            let oy = (y as f64 + dy * sy).round() as i32;

            // TODO: we should use some kind of anti-aliasing when offset is on a pixel border

            if rect.contains(ox, oy) && rect.contains(ox, oy) {
                let idx = (oy * width as i32 + ox) as usize;
                let idx1 = (y * width + x) as usize;
                dest[idx1] = src[idx];
            }

            x += 1;
            if x == width {
                x = 0;
                y += 1;
            }
        }
    }
}

pub mod turbulence {
    use super::*;

    const RAND_M: i32 = 2147483647; // 2**31 - 1
    const RAND_A: i32 = 16807; // 7**5; primitive root of m
    const RAND_Q: i32 = 127773; // m / a
    const RAND_R: i32 = 2836; // m % a
    const B_SIZE: usize = 0x100;
    const B_SIZE_32: i32 = 0x100;
    const B_LEN: usize = B_SIZE + B_SIZE + 2;
    const BM: i32 = 0xff;
    const PERLIN_N: i32 = 0x1000;

    #[derive(Clone, Copy)]
    struct StitchInfo {
        width: i32, // How much to subtract to wrap for stitching.
        height: i32,
        wrap_x: i32, // Minimum value to wrap.
        wrap_y: i32,
    }

    pub fn apply(
        fe: &usvg::FeTurbulence,
        region: ScreenRect,
        ts: &usvg::Transform,
        buf: &mut [rgb::alt::BGRA8],
    ) {
        // TODO: this
        if ts.has_skew() || ts.has_rotate() {
            warn!("'feTurbulence' with complex transform is not supported.");
        }

        let mut ts = *ts;
        ts.a = 1.0 / ts.a;
        ts.d = 1.0 / ts.d;
        ts.e = -ts.e * ts.a;
        ts.f = -ts.f * ts.d;

        let (lattice_selector, gradient) = init(fe.seed);
        let mut x = 0;
        let mut y = 0;
        for pixel in buf {
            let turb = |channel| {
                let (tx, ty) = ts.apply((x as i32 + region.x()) as f64,
                                        (y as i32 + region.y()) as f64);
                let n = turbulence(
                    channel,
                    tx, ty,
                    x as f64, y as f64,
                    region.width() as f64, region.height() as f64,
                    fe.base_frequency.x.value(), fe.base_frequency.y.value(),
                    fe.num_octaves,
                    fe.kind == usvg::FeTurbulenceKind::FractalNoise,
                    fe.stitch_tiles,
                    &lattice_selector,
                    &gradient,
                );

                let n = match fe.kind {
                    usvg::FeTurbulenceKind::FractalNoise => (n * 255.0 + 255.0) / 2.0,
                    usvg::FeTurbulenceKind::Turbulence => n * 255.0,
                };

                (f64_bound(0.0, n, 255.0) + 0.5) as u8
            };

            pixel.r = turb(0);
            pixel.g = turb(1);
            pixel.b = turb(2);
            pixel.a = turb(3);

            x += 1;
            if x == region.width() {
                x = 0;
                y += 1;
            }
        }
    }

    fn init(mut seed: i32) -> (Vec<usize>, Vec<Vec<Vec<f64>>>) {
        let mut lattice_selector = vec![0; B_LEN];
        let mut gradient = vec![vec![vec![0.0; 2]; B_LEN]; 4];

        if seed <= 0 {
            seed = -seed % (RAND_M - 1) + 1;
        }

        if seed > RAND_M - 1 {
            seed = RAND_M - 1;
        }

        for k in 0..4 {
            for i in 0..B_SIZE {
                lattice_selector[i] = i;
                for j in 0..2 {
                    seed = random(seed);
                    gradient[k][i][j] =
                        ((seed % (B_SIZE_32 + B_SIZE_32)) - B_SIZE_32) as f64 / B_SIZE_32 as f64;
                }

                let s = (  gradient[k][i][0] * gradient[k][i][0]
                         + gradient[k][i][1] * gradient[k][i][1]).sqrt();

                gradient[k][i][0] /= s;
                gradient[k][i][1] /= s;
            }
        }

        for i in (1..B_SIZE).rev() {
            let k = lattice_selector[i];
            seed = random(seed);
            let j = (seed % B_SIZE_32) as usize;
            lattice_selector[i] = lattice_selector[j];
            lattice_selector[j] = k;
        }

        for i in 0..B_SIZE + 2 {
            lattice_selector[B_SIZE + i] = lattice_selector[i];
            for k in 0..4 {
                for j in 0..2 {
                    gradient[k][B_SIZE + i][j] = gradient[k][i][j];
                }
            }
        }

        (lattice_selector, gradient)
    }

    fn turbulence(
        color_channel: usize,
        mut x: f64,
        mut y: f64,
        tile_x: f64,
        tile_y: f64,
        tile_width: f64,
        tile_height: f64,
        mut base_freq_x: f64,
        mut base_freq_y: f64,
        num_octaves: u32,
        fractal_sum: bool,
        do_stitching: bool,
        lattice_selector: &[usize],
        gradient: &Vec<Vec<Vec<f64>>>,
    ) -> f64 {
        // Adjust the base frequencies if necessary for stitching.
        let mut stitch = if do_stitching {
            // When stitching tiled turbulence, the frequencies must be adjusted
            // so that the tile borders will be continuous.
            if !base_freq_x.is_fuzzy_zero() {
                let lo_freq = (tile_width * base_freq_x).floor() / tile_width;
                let hi_freq = (tile_width * base_freq_x).ceil() / tile_width;
                if base_freq_x / lo_freq < hi_freq / base_freq_x {
                    base_freq_x = lo_freq;
                } else {
                    base_freq_x = hi_freq;
                }
            }

            if !base_freq_y.is_fuzzy_zero() {
                let lo_freq = (tile_height * base_freq_y).floor() / tile_height;
                let hi_freq = (tile_height * base_freq_y).ceil() / tile_height;
                if base_freq_y / lo_freq < hi_freq / base_freq_y {
                    base_freq_y = lo_freq;
                } else {
                    base_freq_y = hi_freq;
                }
            }

            // Set up initial stitch values.
            let width = (tile_width * base_freq_x + 0.5) as i32;
            let height = (tile_height * base_freq_y + 0.5) as i32;
            let wrap_x = (tile_x * base_freq_x + PERLIN_N as f64 + width as f64) as i32;
            let wrap_y = (tile_y * base_freq_y + PERLIN_N as f64 + height as f64) as i32;
            Some(StitchInfo {
                width,
                height,
                wrap_x,
                wrap_y,
            })
        } else {
            None
        };

        let mut sum = 0.0;
        x *= base_freq_x;
        y *= base_freq_y;
        let mut ratio = 1.0;
        for _ in 0..num_octaves {
            if fractal_sum {
                sum += noise2(color_channel, x, y, lattice_selector, gradient, stitch) / ratio;
            } else {
                sum += noise2(color_channel, x, y, lattice_selector, gradient, stitch).abs() / ratio;
            }
            x *= 2.0;
            y *= 2.0;
            ratio *= 2.0;

            if let Some(ref mut stitch) = stitch {
                // Update stitch values. Subtracting PerlinN before the multiplication and
                // adding it afterward simplifies to subtracting it once.
                stitch.width *= 2;
                stitch.wrap_x = 2 * stitch.wrap_x - PERLIN_N;
                stitch.height *= 2;
                stitch.wrap_y = 2 * stitch.wrap_y - PERLIN_N;
            }
        }

        sum
    }

    fn noise2(
        color_channel: usize,
        x: f64,
        y: f64,
        lattice_selector: &[usize],
        gradient: &Vec<Vec<Vec<f64>>>,
        stitch_info: Option<StitchInfo>,
    ) -> f64 {
        let t = x + PERLIN_N as f64;
        let mut bx0 = t as i32;
        let mut bx1 = bx0 + 1;
        let rx0 = t.fract();
        let rx1 = rx0 - 1.0;
        let t = y + PERLIN_N as f64;
        let mut by0 = t as i32;
        let mut by1 = by0 + 1;
        let ry0 = t.fract();
        let ry1 = ry0 - 1.0;

        // If stitching, adjust lattice points accordingly.
        if let Some(info) = stitch_info {
            if bx0 >= info.wrap_x {
                bx0 -= info.width;
            }

            if bx1 >= info.wrap_x {
                bx1 -= info.width;
            }

            if by0 >= info.wrap_y {
                by0 -= info.height;
            }

            if by1 >= info.wrap_y {
                by1 -= info.height;
            }
        }

        bx0 &= BM;
        bx1 &= BM;
        by0 &= BM;
        by1 &= BM;
        let i = lattice_selector[bx0 as usize];
        let j = lattice_selector[bx1 as usize];
        let b00 = lattice_selector[i + by0 as usize];
        let b10 = lattice_selector[j + by0 as usize];
        let b01 = lattice_selector[i + by1 as usize];
        let b11 = lattice_selector[j + by1 as usize];
        let sx = s_curve(rx0);
        let sy = s_curve(ry0);
        let q = &gradient[color_channel][b00];
        let u = rx0 * q[0] + ry0 * q[1];
        let q = &gradient[color_channel][b10];
        let v = rx1 * q[0] + ry0 * q[1];
        let a = lerp(sx, u, v);
        let q = &gradient[color_channel][b01];
        let u = rx0 * q[0] + ry1 * q[1];
        let q = &gradient[color_channel][b11];
        let v = rx1 * q[0] + ry1 * q[1];
        let b = lerp(sx, u, v);
        lerp(sy, a, b)
    }

    fn random(seed: i32) -> i32 {
        let mut result = RAND_A * (seed % RAND_Q) - RAND_R * (seed / RAND_Q);
        if result <= 0 {
            result += RAND_M;
        }

        result
    }

    #[inline]
    fn s_curve(t: f64) -> f64 {
        t * t * (3.0 - 2.0 * t)
    }

    #[inline]
    fn lerp(t: f64, a: f64, b: f64) -> f64 {
        a + t * (b - a)
    }
}

pub mod lighting {
    use super::*;
    use rgb::alt::BGRA8 as Pixel;


    const FACTOR_1_2: f64 = 1.0 / 2.0;
    const FACTOR_1_3: f64 = 1.0 / 3.0;
    const FACTOR_1_4: f64 = 1.0 / 4.0;
    const FACTOR_2_3: f64 = 2.0 / 3.0;


    #[derive(Clone, Copy)]
    struct ImageRef<'a> {
        data: &'a [Pixel],
        width: u32,
    }

    impl<'a> ImageRef<'a> {
        #[inline]
        fn new(data: &'a [Pixel], width: u32) -> Self {
            ImageRef { data, width }
        }

        #[inline]
        fn alpha_at(&self, x: u32, y: u32) -> i16 {
            self.data[(self.width * y + x) as usize].a as i16
        }
    }


    #[derive(Clone, Copy, Debug)]
    struct Vector2 {
        x: f64,
        y: f64,
    }

    impl Vector2 {
        #[inline]
        fn new(x: f64, y: f64) -> Self {
            Vector2 { x, y }
        }
    }

    impl usvg::FuzzyEq for Vector2 {
        #[inline]
        fn fuzzy_eq(&self, other: &Self) -> bool {
               self.x.fuzzy_eq(&other.x)
            && self.y.fuzzy_eq(&other.y)
        }
    }

    impl usvg::FuzzyZero for Vector2 {
        #[inline]
        fn is_fuzzy_zero(&self) -> bool {
               self.x.is_fuzzy_zero()
            && self.y.is_fuzzy_zero()
        }
    }

    impl std::ops::Mul<f64> for Vector2 {
        type Output = Self;

        #[inline]
        fn mul(self, c: f64) -> Self::Output {
            Vector2 {
                x: self.x * c,
                y: self.y * c,
            }
        }
    }


    #[derive(Clone, Copy, Debug)]
    struct Vector3 {
        x: f64,
        y: f64,
        z: f64,
    }

    impl Vector3 {
        #[inline]
        fn new(x: f64, y: f64, z: f64) -> Self {
            Vector3 { x, y, z }
        }

        #[inline]
        fn dot(&self, other: &Self) -> f64 {
            self.x * other.x + self.y * other.y + self.z * other.z
        }

        #[inline]
        fn length(&self) -> f64 {
            (self.x*self.x + self.y*self.y + self.z*self.z).sqrt()
        }

        #[inline]
        fn normalized(&self) -> Option<Self> {
            let length = self.length();
            if !length.is_fuzzy_zero() {
                Some(Vector3 {
                    x: self.x / length,
                    y: self.y / length,
                    z: self.z / length,
                })
            } else {
                None
            }
        }
    }

    impl std::ops::Add<Vector3> for Vector3 {
        type Output = Self;

        #[inline]
        fn add(self, rhs: Vector3) -> Self::Output {
            Vector3 {
                x: self.x + rhs.x,
                y: self.y + rhs.y,
                z: self.z + rhs.z,
            }
        }
    }

    impl std::ops::Sub<Vector3> for Vector3 {
        type Output = Self;

        #[inline]
        fn sub(self, rhs: Vector3) -> Self::Output {
            Vector3 {
                x: self.x - rhs.x,
                y: self.y - rhs.y,
                z: self.z - rhs.z,
            }
        }
    }


    #[derive(Clone, Copy, Debug)]
    struct Normal {
        factor: Vector2,
        normal: Vector2,
    }

    impl Normal {
        #[inline]
        fn new(factor_x: f64, factor_y: f64, nx: i16, ny: i16) -> Self {
            Normal {
                factor: Vector2::new(factor_x, factor_y),
                normal: Vector2::new(-nx as f64, -ny as f64),
            }
        }
    }


    pub fn apply_diffuse(
        fe: &usvg::FeDiffuseLighting,
        region: ScreenRect,
        ts: &usvg::Transform,
        buf_in: &[Pixel],
        buf_out: &mut [Pixel],
    ) {
        let light_factor = |normal: Normal, light_vector: Vector3| {
            let k = if normal.normal.is_fuzzy_zero() {
                light_vector.z
            } else {
                let mut n = normal.normal * (fe.surface_scale / 255.0);
                n.x *= normal.factor.x;
                n.y *= normal.factor.y;

                let normal = Vector3::new(n.x, n.y, 1.0);

                normal.dot(&light_vector) / normal.length()
            };

            fe.diffuse_constant * k
        };

        apply(fe.light_source, fe.surface_scale, fe.lighting_color, &light_factor,
              calc_diffuse_alpha, region, ts, buf_in, buf_out);
    }

    pub fn apply_specular(
        fe: &usvg::FeSpecularLighting,
        region: ScreenRect,
        ts: &usvg::Transform,
        buf_in: &[Pixel],
        buf_out: &mut [Pixel],
    ) {
        let light_factor = |normal: Normal, light_vector: Vector3| {
            let h = light_vector + Vector3::new(0.0, 0.0, 1.0);
            let h_length = h.length();

            if h_length.is_fuzzy_zero() {
                return 0.0;
            }

            let k = if normal.normal.is_fuzzy_zero() {
                let n_dot_h = h.z / h_length;
                if fe.specular_exponent.fuzzy_eq(&1.0) {
                    n_dot_h
                } else {
                    n_dot_h.powf(fe.specular_exponent)
                }
            } else {
                let mut n = normal.normal * (fe.surface_scale / 255.0);
                n.x *= normal.factor.x;
                n.y *= normal.factor.y;

                let normal = Vector3::new(n.x, n.y, 1.0);

                let n_dot_h = normal.dot(&h) / normal.length() / h_length;
                if fe.specular_exponent.fuzzy_eq(&1.0) {
                    n_dot_h
                } else {
                    n_dot_h.powf(fe.specular_exponent)
                }
            };

            fe.specular_constant * k
        };

        apply(fe.light_source, fe.surface_scale, fe.lighting_color, &light_factor,
              calc_specular_alpha, region, ts, buf_in, buf_out);
    }

    fn apply(
        mut light_source: usvg::FeLightSource,
        surface_scale: f64,
        lighting_color: usvg::Color,
        light_factor: &dyn Fn(Normal, Vector3) -> f64,
        calc_alpha: fn(u8, u8, u8) -> u8,
        region: ScreenRect,
        ts: &usvg::Transform,
        buf_in: &[Pixel],
        buf_out: &mut [Pixel],
    ) {
        use std::f64::consts::SQRT_2;

        let width = region.width();
        let height = region.height();

        if width < 3 || height < 3 {
            return;
        }

        // Transform light source.
        match light_source {
            usvg::FeLightSource::FeDistantLight(..) => {}
            usvg::FeLightSource::FePointLight(ref mut light) => {
                let (x, y) = ts.apply(light.x, light.y);
                light.x = x - region.x() as f64;
                light.y = y - region.y() as f64;
                light.z = light.z * (ts.a*ts.a + ts.d*ts.d).sqrt() / SQRT_2;
            }
            usvg::FeLightSource::FeSpotLight(ref mut light) => {
                let sz = (ts.a*ts.a + ts.d*ts.d).sqrt() / SQRT_2;

                let (x, y) = ts.apply(light.x, light.y);
                light.x = x - region.x() as f64;
                light.y = y - region.x() as f64;
                light.z = light.z * sz;

                let (x, y) = ts.apply(light.points_at_x, light.points_at_y);
                light.points_at_x = x - region.x() as f64;
                light.points_at_y = y - region.x() as f64;
                light.points_at_z = light.points_at_z * sz;
            }
        };

        let in_img = ImageRef::new(buf_in, width);

        // `feDistantLight` has a fixed vector, so calculate it beforehand.
        let mut light_vector = match light_source {
            usvg::FeLightSource::FeDistantLight(ref light) => {
                let azimuth = light.azimuth.to_radians();
                let elevation = light.elevation.to_radians();
                Vector3::new(
                    azimuth.cos() * elevation.cos(),
                    azimuth.sin() * elevation.cos(),
                    elevation.sin(),
                )
            }
            _ => Vector3::new(1.0, 1.0, 1.0),
        };

        let mut calc = |x, y, normal: Normal| {
            let z = in_img.alpha_at(x, y) as f64 / 255.0 * surface_scale;

            match light_source {
                usvg::FeLightSource::FeDistantLight(..) => {}
                usvg::FeLightSource::FePointLight(ref light) => {
                    let origin = Vector3::new(light.x, light.y, light.z);
                    let v = origin - Vector3::new(x as f64, y as f64, z);
                    light_vector = v.normalized().unwrap_or(v);
                }
                usvg::FeLightSource::FeSpotLight(ref light) => {
                    let origin = Vector3::new(light.x, light.y, light.z);
                    let v = origin - Vector3::new(x as f64, y as f64, z);
                    light_vector = v.normalized().unwrap_or(v);
                }
            }

            let light_color = light_color(&light_source, lighting_color, light_vector);
            let factor = light_factor(normal, light_vector);

            let compute = |x| (f64_bound(0.0, x as f64 * factor, 255.0) + 0.5) as u8;

            let r = compute(light_color.red);
            let g = compute(light_color.green);
            let b = compute(light_color.blue);
            let a = calc_alpha(r, g, b);

            buf_out[(width * y + x) as usize] = Pixel { b, g, r, a };
        };

        calc(0,         0,          top_left_normal(in_img));
        calc(width - 1, 0,          top_right_normal(in_img, width));
        calc(0,         height - 1, bottom_left_normal(in_img, height));
        calc(width - 1, height - 1, bottom_right_normal(in_img, width, height));

        for x in 1..width-1 {
            calc(x, 0,          top_row_normal(in_img, x));
            calc(x, height - 1, bottom_row_normal(in_img, x, height));
        }

        for y in 1..height-1 {
            calc(0,         y, left_column_normal(in_img, y));
            calc(width - 1, y, right_column_normal(in_img, y, width));
        }

        for y in 1..height-1 {
            for x in 1..width-1 {
                calc(x, y, interior_normal(in_img, x, y));
            }
        }
    }

    fn light_color(
        light: &usvg::FeLightSource,
        lighting_color: usvg::Color,
        light_vector: Vector3,
    ) -> usvg::Color {
        match light {
            usvg::FeLightSource::FeDistantLight(..) | usvg::FeLightSource::FePointLight(..) => {
                lighting_color
            }
            usvg::FeLightSource::FeSpotLight(ref light) => {
                let origin = Vector3::new(light.x, light.y, light.z);
                let direction = Vector3::new(light.points_at_x, light.points_at_y, light.points_at_z);
                let direction = direction - origin;
                let direction = direction.normalized().unwrap_or(direction);
                let minus_l_dot_s = -light_vector.dot(&direction);
                if minus_l_dot_s <= 0.0 {
                    return usvg::Color::black();
                }

                if let Some(limiting_cone_angle) = light.limiting_cone_angle {
                    if minus_l_dot_s < limiting_cone_angle.to_radians().cos() {
                        return usvg::Color::black();
                    }
                }

                let factor = minus_l_dot_s.powf(light.specular_exponent.value());
                let compute = |x| (f64_bound(0.0, x as f64 * factor, 255.0) + 0.5) as u8;

                usvg::Color::new(
                    compute(lighting_color.red),
                    compute(lighting_color.green),
                    compute(lighting_color.blue),
                )
            }
        }
    }

    fn top_left_normal(img: ImageRef) -> Normal {
        let center       = img.alpha_at(0, 0);
        let right        = img.alpha_at(1, 0);
        let bottom       = img.alpha_at(0, 1);
        let bottom_right = img.alpha_at(1, 1);

        Normal::new(
            FACTOR_2_3,
            FACTOR_2_3,
            -2 * center + 2 * right - bottom + bottom_right,
            -2 * center - right + 2 * bottom + bottom_right,
        )
    }

    fn top_right_normal(img: ImageRef, width: u32) -> Normal {
        let left         = img.alpha_at(width - 2, 0);
        let center       = img.alpha_at(width - 1, 0);
        let bottom_left  = img.alpha_at(width - 2, 1);
        let bottom       = img.alpha_at(width - 1, 1);

        Normal::new(
            FACTOR_2_3,
            FACTOR_2_3,
            -2 * left + 2 * center - bottom_left + bottom,
            -left - 2 * center + bottom_left + 2 * bottom,
        )
    }

    fn bottom_left_normal(img: ImageRef, height: u32) -> Normal {
        let top          = img.alpha_at(0, height - 2);
        let top_right    = img.alpha_at(1, height - 2);
        let center       = img.alpha_at(0, height - 1);
        let right        = img.alpha_at(1, height - 1);

        Normal::new(
            FACTOR_2_3,
            FACTOR_2_3,
            -top + top_right - 2 * center + 2 * right,
            -2 * top - top_right + 2 * center + right,
        )
    }

    fn bottom_right_normal(img: ImageRef, width: u32, height: u32) -> Normal {
        let top_left     = img.alpha_at(width - 2, height - 2);
        let top          = img.alpha_at(width - 1, height - 2);
        let left         = img.alpha_at(width - 2, height - 1);
        let center       = img.alpha_at(width - 1, height - 1);

        Normal::new(
            FACTOR_2_3,
            FACTOR_2_3,
            -top_left + top - 2 * left + 2 * center,
            -top_left - 2 * top + left + 2 * center,
        )
    }

    fn top_row_normal(img: ImageRef, x: u32) -> Normal {
        let left         = img.alpha_at(x - 1, 0);
        let center       = img.alpha_at(x,     0);
        let right        = img.alpha_at(x + 1, 0);
        let bottom_left  = img.alpha_at(x - 1, 1);
        let bottom       = img.alpha_at(x,     1);
        let bottom_right = img.alpha_at(x + 1, 1);

        Normal::new(
            FACTOR_1_3,
            FACTOR_1_2,
            -2 * left + 2 * right - bottom_left + bottom_right,
            -left - 2 * center - right + bottom_left + 2 * bottom + bottom_right,
        )
    }

    fn bottom_row_normal(img: ImageRef, x: u32, height: u32) -> Normal {
        let top_left     = img.alpha_at(x - 1, height - 2);
        let top          = img.alpha_at(x,     height - 2);
        let top_right    = img.alpha_at(x + 1, height - 2);
        let left         = img.alpha_at(x - 1, height - 1);
        let center       = img.alpha_at(x,     height - 1);
        let right        = img.alpha_at(x + 1, height - 1);

        Normal::new(
            FACTOR_1_3,
            FACTOR_1_2,
            -top_left + top_right - 2 * left + 2 * right,
            -top_left - 2 * top - top_right + left + 2 * center + right,
        )
    }

    fn left_column_normal(img: ImageRef, y: u32) -> Normal {
        let top          = img.alpha_at(0, y - 1);
        let top_right    = img.alpha_at(1, y - 1);
        let center       = img.alpha_at(0, y);
        let right        = img.alpha_at(1, y);
        let bottom       = img.alpha_at(0, y + 1);
        let bottom_right = img.alpha_at(1, y + 1);

        Normal::new(
            FACTOR_1_2,
            FACTOR_1_3,
            -top + top_right - 2 * center + 2 * right - bottom + bottom_right,
            -2 * top - top_right + 2 * bottom + bottom_right,
        )
    }

    fn right_column_normal(img: ImageRef, y: u32, width: u32) -> Normal {
        let top_left     = img.alpha_at(width - 2, y - 1);
        let top          = img.alpha_at(width - 1, y - 1);
        let left         = img.alpha_at(width - 2, y);
        let center       = img.alpha_at(width - 1, y);
        let bottom_left  = img.alpha_at(width - 2, y + 1);
        let bottom       = img.alpha_at(width - 1, y + 1);

        Normal::new(
            FACTOR_1_2,
            FACTOR_1_3,
            -top_left + top - 2 * left + 2 * center - bottom_left + bottom,
            -top_left - 2 * top + bottom_left + 2 * bottom,
        )
    }

    fn interior_normal(img: ImageRef, x: u32, y: u32) -> Normal {
        let top_left     = img.alpha_at(x - 1, y - 1);
        let top          = img.alpha_at(x,     y - 1);
        let top_right    = img.alpha_at(x + 1, y - 1);
        let left         = img.alpha_at(x - 1, y);
        let right        = img.alpha_at(x + 1, y);
        let bottom_left  = img.alpha_at(x - 1, y + 1);
        let bottom       = img.alpha_at(x,     y + 1);
        let bottom_right = img.alpha_at(x + 1, y + 1);

        Normal::new(
            FACTOR_1_4,
            FACTOR_1_4,
            -top_left + top_right - 2 * left + 2 * right - bottom_left + bottom_right,
            -top_left - 2 * top - top_right + bottom_left + 2 * bottom + bottom_right,
        )
    }

    fn calc_diffuse_alpha(_: u8, _: u8, _: u8) -> u8 {
        255
    }

    fn calc_specular_alpha(r: u8, g: u8, b: u8) -> u8 {
        use std::cmp::max;
        max(max(r, g), b)
    }
}


pub trait TransferFunctionExt {
    /// Applies a transfer function to a provided color component.
    ///
    /// Requires a non-premultiplied color component.
    fn apply(&self, c: u8) -> u8;
}

impl TransferFunctionExt for usvg::TransferFunction {
    /// Applies a transfer function to a provided color component.
    ///
    /// Requires a non-premultiplied color component.
    fn apply(&self, c: u8) -> u8 {
        fn apply_impl(kind: &usvg::TransferFunction, c: f64) -> f64 {
            use std::cmp;

            match kind {
                usvg::TransferFunction::Identity => {
                    c
                }
                usvg::TransferFunction::Table(ref values) => {
                    if values.is_empty() {
                        return c;
                    }

                    let n = values.len() - 1;
                    let k = (c * (n as f64)).floor() as usize;
                    let k = cmp::min(k, n);
                    if k == n {
                        return values[k];
                    }

                    let vk = values[k];
                    let vk1 = values[k + 1];
                    let k = k as f64;
                    let n = n as f64;

                    vk + (c - k / n) * n * (vk1 - vk)
                }
                usvg::TransferFunction::Discrete(ref values) => {
                    if values.is_empty() {
                        return c;
                    }

                    let n = values.len();
                    let k = (c * (n as f64)).floor() as usize;

                    values[cmp::min(k, n - 1)]
                }
                usvg::TransferFunction::Linear { slope, intercept } => {
                    slope * c + intercept
                }
                usvg::TransferFunction::Gamma { amplitude, exponent, offset } => {
                    amplitude * c.powf(*exponent) + offset
                }
            }
        }

        (f64_bound(0.0, apply_impl(self, c as f64 / 255.0), 1.0) * 255.0) as u8
    }
}


pub fn calc_region<T: ImageExt>(
    filter: &usvg::Filter,
    bbox: Option<Rect>,
    ts: &usvg::Transform,
    canvas: &T,
) -> Result<ScreenRect, Error> {
    let path = usvg::PathData::from_rect(filter.rect);

    let region_ts = if filter.units == usvg::Units::ObjectBoundingBox {
        let bbox = bbox.ok_or(Error::InvalidRegion)?;
        let bbox_ts = usvg::Transform::from_bbox(bbox);
        let mut ts2 = ts.clone();
        ts2.append(&bbox_ts);
        ts2
    } else {
        *ts
    };

    let canvas_rect = ScreenRect::new(0, 0, canvas.width(), canvas.height()).unwrap();
    let region = path.bbox_with_transform(region_ts, None)
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


pub fn from_premultiplied(data: &mut [rgb::alt::BGRA8]) {
    for p in data {
        let a = p.a as f64 / 255.0;
        p.b = (p.b as f64 / a + 0.5) as u8;
        p.g = (p.g as f64 / a + 0.5) as u8;
        p.r = (p.r as f64 / a + 0.5) as u8;
    }
}

pub fn into_premultiplied(data: &mut [rgb::alt::BGRA8]) {
    for p in data {
        let a = p.a as f64 / 255.0;
        p.b = (p.b as f64 * a + 0.5) as u8;
        p.g = (p.g as f64 * a + 0.5) as u8;
        p.r = (p.r as f64 * a + 0.5) as u8;
    }
}
