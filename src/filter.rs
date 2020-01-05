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
                           || std_dy >= BLUR_SIGMA_THRESHOLD;

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
        usvg::FilterKind::FeFlood(..) |
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

pub trait IntoSvgFilters<T>: Sized {
    fn into_svgf(self) -> T;
}

impl IntoSvgFilters<svgfilters::BGR8> for usvg::Color {
    fn into_svgf(self) -> svgfilters::BGR8 {
        svgfilters::BGR8 { b: self.blue, g: self.green, r: self.red }
    }
}

impl IntoSvgFilters<svgfilters::LightSource> for usvg::FeLightSource {
    fn into_svgf(self) -> svgfilters::LightSource {
        match self {
            usvg::FeLightSource::FeDistantLight(ref light) => {
                svgfilters::LightSource::DistantLight {
                    azimuth: light.azimuth,
                    elevation: light.elevation,
                }
            }
            usvg::FeLightSource::FePointLight(ref light) => {
                svgfilters::LightSource::PointLight {
                    x: light.x,
                    y: light.y,
                    z: light.z,
                }
            }
            usvg::FeLightSource::FeSpotLight(ref light) => {
                svgfilters::LightSource::SpotLight {
                    x: light.x,
                    y: light.y,
                    z: light.z,
                    points_at_x: light.points_at_x,
                    points_at_y: light.points_at_y,
                    points_at_z: light.points_at_z,
                    specular_exponent: light.specular_exponent.value().into(),
                    limiting_cone_angle: light.limiting_cone_angle,
                }
            }
        }
    }
}

impl<'a> IntoSvgFilters<svgfilters::TransferFunction<'a>> for &'a usvg::TransferFunction {
    fn into_svgf(self) -> svgfilters::TransferFunction<'a> {
        match *self {
            usvg::TransferFunction::Identity =>
                svgfilters::TransferFunction::Identity,
            usvg::TransferFunction::Table(ref data) =>
                svgfilters::TransferFunction::Table(data),
            usvg::TransferFunction::Discrete(ref data) =>
                svgfilters::TransferFunction::Discrete(data),
            usvg::TransferFunction::Linear { slope, intercept } =>
                svgfilters::TransferFunction::Linear { slope, intercept },
            usvg::TransferFunction::Gamma { amplitude, exponent, offset } =>
                svgfilters::TransferFunction::Gamma { amplitude, exponent, offset },
        }
    }
}

impl<'a> IntoSvgFilters<svgfilters::ColorMatrix<'a>> for &'a usvg::FeColorMatrixKind {
    fn into_svgf(self) -> svgfilters::ColorMatrix<'a> {
        use std::convert::TryInto;

        match *self {
            usvg::FeColorMatrixKind::Matrix(ref data) =>
                svgfilters::ColorMatrix::Matrix(data.as_slice().try_into().unwrap()),
            usvg::FeColorMatrixKind::Saturate(n) =>
                svgfilters::ColorMatrix::Saturate(svgfilters::NormalizedValue::new(n.value())),
            usvg::FeColorMatrixKind::HueRotate(n) =>
                svgfilters::ColorMatrix::HueRotate(n),
            usvg::FeColorMatrixKind::LuminanceToAlpha =>
                svgfilters::ColorMatrix::LuminanceToAlpha,
        }
    }
}

impl IntoSvgFilters<svgfilters::MorphologyOperator> for usvg::FeMorphologyOperator {
    fn into_svgf(self) -> svgfilters::MorphologyOperator {
        match self {
            usvg::FeMorphologyOperator::Erode => svgfilters::MorphologyOperator::Erode,
            usvg::FeMorphologyOperator::Dilate => svgfilters::MorphologyOperator::Dilate,
        }
    }
}

impl IntoSvgFilters<svgfilters::ColorChannel> for usvg::ColorChannel {
    fn into_svgf(self) -> svgfilters::ColorChannel {
        match self {
            usvg::ColorChannel::R => svgfilters::ColorChannel::R,
            usvg::ColorChannel::G => svgfilters::ColorChannel::G,
            usvg::ColorChannel::B => svgfilters::ColorChannel::B,
            usvg::ColorChannel::A => svgfilters::ColorChannel::A,
        }
    }
}

impl IntoSvgFilters<svgfilters::EdgeMode> for usvg::FeEdgeMode {
    fn into_svgf(self) -> svgfilters::EdgeMode {
        match self {
            usvg::FeEdgeMode::None => svgfilters::EdgeMode::None,
            usvg::FeEdgeMode::Duplicate => svgfilters::EdgeMode::Duplicate,
            usvg::FeEdgeMode::Wrap => svgfilters::EdgeMode::Wrap,
        }
    }
}

impl<'a> IntoSvgFilters<svgfilters::ConvolveMatrix<'a>> for &'a usvg::ConvolveMatrix {
    fn into_svgf(self) -> svgfilters::ConvolveMatrix<'a> {
        svgfilters::ConvolveMatrix::new(
            self.target_x(), self.target_y(),
            self.columns(), self.rows(),
            self.data(),
        ).unwrap()
    }
}

pub fn transform_light_source(
    region: ScreenRect,
    ts: &usvg::Transform,
    mut light_source: usvg::FeLightSource,
) -> usvg::FeLightSource {
    use std::f64::consts::SQRT_2;

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
    }

    light_source
}
