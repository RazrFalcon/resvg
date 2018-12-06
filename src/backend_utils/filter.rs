// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::rc::Rc;

// external
use usvg::ColorInterpolation as ColorSpace;

// self
use super::super::prelude::*;


pub enum Error {
    AllocFailed,
    InvalidRegion,
    ZeroSizedObject,
}


pub trait ImageExt
    where Self: Sized
{
    fn width(&self) -> u32;
    fn height(&self) -> u32;

    fn clone_image(&self) -> Result<Self, Error>;
    fn clip_image(&mut self, region: ScreenRect);

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
            region: ScreenRect::new(0, 0, w, h),
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
            Err(v) => v.clone_image(),
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
        bbox: Rect,
        ts: &usvg::Transform,
        canvas: &mut T,
    ) {
        match Self::_apply(filter, bbox, ts, canvas) {
            Ok(_) => {}
            Err(Error::AllocFailed) =>
                warn!("Memory allocation failed while processing the '{}' filter. Skipped.", filter.id),
            Err(Error::InvalidRegion) =>
                warn!("Filter '{}' has an invalid region.", filter.id),
            Err(Error::ZeroSizedObject) =>
                warn!("Filter '{}' cannot be used on a zero-sized object.", filter.id),
        }
    }

    fn _apply(
        filter: &usvg::Filter,
        bbox: Rect,
        ts: &usvg::Transform,
        canvas: &mut T,
    ) -> Result<(), Error> {
        let mut results = Vec::new();

        let canvas_rect = ScreenRect::new(0, 0, canvas.width(), canvas.height());
        let region = calc_region(filter, bbox, ts, canvas_rect)?;

        for primitive in &filter.children {
            let input = &primitive.filter_input;
            let cs = primitive.color_interpolation;

            let mut result = match primitive.kind {
                usvg::FilterKind::FeBlend(ref fe) => {
                    let input1 = Self::get_input(input, region, &results, canvas)?;
                    let input2 = Self::get_input(&fe.filter_input2, region, &results, canvas)?;
                    Self::apply_blend(fe, cs, region, input1, input2)
                }
                usvg::FilterKind::FeFlood(ref fe) => {
                    Self::apply_flood(fe, region)
                }
                usvg::FilterKind::FeGaussianBlur(ref fe) => {
                    let input = Self::get_input(input, region, &results, canvas)?;
                    Self::apply_blur(fe, filter.primitive_units, cs, bbox, ts, input)
                }
                usvg::FilterKind::FeOffset(ref fe) => {
                    let input = Self::get_input(input, region, &results, canvas)?;
                    Self::apply_offset(filter, fe, bbox, ts, input)
                }
                usvg::FilterKind::FeComposite(ref fe) => {
                    let input1 = Self::get_input(input, region, &results, canvas)?;
                    let input2 = Self::get_input(&fe.filter_input2, region, &results, canvas)?;
                    Self::apply_composite(fe, cs, region, input1, input2)
                }
                usvg::FilterKind::FeMerge(ref fe) => {
                    Self::apply_merge(fe, cs, region, &results, canvas)
                }
                usvg::FilterKind::FeTile => {
                    let input = Self::get_input(input, region, &results, canvas)?;
                    Self::apply_tile(input, region)
                }
            }?;

            let subregion = calc_subregion(filter, primitive, region, ts, &results);
            if region != subregion {
                // Clip result.

                let mut subregion2 = subregion;
                subregion2.x -= region.x;
                subregion2.y -= region.y;

                let color_space = result.color_space;
                let mut buffer = result.take()?;
                buffer.clip_image(subregion2);

                result = Image {
                    image: Rc::new(buffer),
                    region: subregion,
                    color_space,
                };
            }

            results.push(FilterResult {
                name: primitive.filter_result.as_ref().unwrap_or(&String::new()).clone(),
                image: result,
            });
        }

        if let Some(res) = results.pop() {
            Self::apply_to_canvas(res.image, region, canvas)?;
        }

        Ok(())
    }

    fn get_input(
        input: &Option<usvg::FilterInput>,
        region: ScreenRect,
        results: &[FilterResult<T>],
        canvas: &T,
    ) -> Result<Image<T>, Error>;

    fn apply_blur(
        fe: &usvg::FeGaussianBlur,
        units: usvg::Units,
        cs: ColorSpace,
        bbox: Rect,
        ts: &usvg::Transform,
        input: Image<T>,
    ) -> Result<Image<T>, Error>;

    fn apply_offset(
        filter: &usvg::Filter,
        fe: &usvg::FeOffset,
        bbox: Rect,
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

    fn apply_to_canvas(
        input: Image<T>,
        region: ScreenRect,
        canvas: &mut T,
    ) -> Result<(), Error>;

    fn resolve_std_dev(
        fe: &usvg::FeGaussianBlur,
        units: usvg::Units,
        bbox: Rect,
        ts: &usvg::Transform,
    ) -> Option<(f64, f64)> {
        // 'A negative value or a value of zero disables the effect of the given filter primitive
        // (i.e., the result is the filter input image).'
        if fe.std_dev_x.is_zero() && fe.std_dev_y.is_zero() {
            return None;
        }

        let (sx, sy) = ts.get_scale();

        Some(if units == usvg::Units::ObjectBoundingBox {
            (*fe.std_dev_x * sx * bbox.width, *fe.std_dev_y * sy * bbox.height)
        } else {
            (*fe.std_dev_x * sx, *fe.std_dev_y * sy)
        })
    }
}


pub mod blur {
    // external
    use rgb::{
        RGBA8,
        FromSlice,
        ComponentSlice,
    };

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

        let post_scale = ((dnu_x * dnu_y).sqrt() / (lambda_x * lambda_y).sqrt())
                            .powi(2 * d.steps as i32);

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
    bbox: Rect,
    ts: &usvg::Transform,
    canvas_rect: ScreenRect,
) -> Result<ScreenRect, Error> {
    let region = if filter.units == usvg::Units::ObjectBoundingBox {
        // Not 0.0, because bbox min size is 1x1.
        // See utils::path_bbox().
        if bbox.width.fuzzy_eq(&1.0) || bbox.height.fuzzy_eq(&1.0) {
            return Err(Error::ZeroSizedObject);
        }

        Rect::new(
            bbox.x + bbox.width * filter.rect.x,
            bbox.y + bbox.height * filter.rect.y,
            bbox.width * filter.rect.width,
            bbox.height * filter.rect.height,
        )
    } else {
        filter.rect
    };

    let region = region.transform(*ts).to_screen_rect().fit_to_rect(canvas_rect);

    if region.width == 0 || region.height == 0 {
        return Err(Error::InvalidRegion);
    }

    Ok(region)
}

/// Returns filter primitive region.
fn calc_subregion<T: ImageExt>(
    filter: &usvg::Filter,
    primitive: &usvg::FilterPrimitive,
    filter_region: ScreenRect,
    ts: &usvg::Transform,
    results: &[FilterResult<T>],
) -> ScreenRect {
    let region = if let usvg::FilterKind::FeOffset(..) = primitive.kind {
        // `feOffset` inherits it's region from the input.
        match primitive.filter_input {
            Some(usvg::FilterInput::Reference(ref name)) => {
                match results.iter().rev().find(|v| v.name == *name) {
                    Some(ref res) => res.image.region,
                    None => filter_region,
                }
            }
            None => {
                match results.last() {
                    Some(ref res) => res.image.region,
                    None => filter_region,
                }
            }
            _ => {
                filter_region
            }
        }
    } else {
        filter_region
    };

    let subregion = if filter.primitive_units == usvg::Units::ObjectBoundingBox {
        let subregion_bbox = Rect::new(
            primitive.x.unwrap_or(0.0),
            primitive.y.unwrap_or(0.0),
            primitive.width.unwrap_or(1.0),
            primitive.height.unwrap_or(1.0),
        );
        let ts = usvg::Transform::from_bbox(subregion_bbox);

        region.to_rect().transform(ts)
    } else {
        let (sx, sy) = ts.get_scale();
        Rect::new(
            primitive.x.map(|n| n * sx).unwrap_or(region.x as f64),
            primitive.y.map(|n| n * sy).unwrap_or(region.y as f64),
            primitive.width.map(|n| n * sx).unwrap_or(region.width as f64),
            primitive.height.map(|n| n * sy).unwrap_or(region.height as f64),
        )
    };

    subregion.to_screen_rect()
}
