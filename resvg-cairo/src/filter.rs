// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::cmp;
use std::rc::Rc;

use usvg::ColorInterpolation as ColorSpace;
use rgb::FromSlice;
use log::warn;

use crate::render::prelude::*;


/// A helper trait to convert `usvg` types into `svgfilters` one.
trait IntoSvgFilters<T>: Sized {
    /// Converts an `usvg` type into `svgfilters` one.
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


pub(crate) enum Error {
    AllocFailed,
    InvalidRegion,
    NoResults,
}


trait ImageSurfaceExt: Sized {
    fn try_create(width: u32, height: u32) -> Result<cairo::ImageSurface, Error>;
    fn copy_region(&self, region: ScreenRect) -> Result<cairo::ImageSurface, Error>;
    fn clip_region(&mut self, region: ScreenRect);
    fn clear(&mut self);
    fn into_srgb(&mut self);
    fn into_linear_rgb(&mut self);
}

impl ImageSurfaceExt for cairo::ImageSurface {
    fn try_create(width: u32, height: u32) -> Result<cairo::ImageSurface, Error> {
        cairo::ImageSurface::create(cairo::Format::ARgb32, width as i32, height as i32)
            .map_err(|_| Error::AllocFailed)
    }

    fn copy_region(&self, region: ScreenRect) -> Result<cairo::ImageSurface, Error> {
        let x = cmp::max(0, region.x()) as f64;
        let y = cmp::max(0, region.y()) as f64;

        let new_image = cairo::ImageSurface::try_create(region.width(), region.height())?;

        let cr = cairo::Context::new(&new_image);
        cr.set_source_surface(&*self, -x, -y);
        cr.paint();

        Ok(new_image)
    }

    fn clip_region(&mut self, region: ScreenRect) {
        let cr = cairo::Context::new(self);
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        cr.set_operator(cairo::Operator::Clear);

        cr.rectangle(0.0, 0.0, self.get_width() as f64, region.y() as f64);
        cr.rectangle(0.0, 0.0, region.x() as f64, self.get_height() as f64);
        cr.rectangle(region.right() as f64, 0.0, self.get_width() as f64, self.get_height() as f64);
        cr.rectangle(0.0, region.bottom() as f64, self.get_width() as f64, self.get_height() as f64);

        cr.fill();
    }

    fn clear(&mut self) {
        let cr = cairo::Context::new(self);
        cr.set_operator(cairo::Operator::Clear);
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        cr.paint();
    }

    fn into_srgb(&mut self) {
        if let Ok(ref mut data) = self.get_data() {
            svgfilters::demultiply_alpha(data.as_bgra_mut());
            svgfilters::from_linear_rgb(data.as_bgra_mut());
            svgfilters::multiply_alpha(data.as_bgra_mut());
        } else {
            warn!("Cairo surface is already borrowed.");
        }
    }

    fn into_linear_rgb(&mut self) {
        if let Ok(ref mut data) = self.get_data() {
            svgfilters::demultiply_alpha(data.as_bgra_mut());
            svgfilters::into_linear_rgb(data.as_bgra_mut());
            svgfilters::multiply_alpha(data.as_bgra_mut());
        } else {
            warn!("Cairo surface is already borrowed.");
        }
    }
}


#[derive(Clone)]
struct Image {
    /// Filter primitive result.
    ///
    /// All images have the same size which is equal to the current filter region.
    image: Rc<cairo::ImageSurface>,

    /// Image's region that has actual data.
    ///
    /// Region is in global coordinates and not in `image` one.
    ///
    /// Image's content outside this region will be transparent/cleared.
    ///
    /// Currently used only for `feTile`.
    region: ScreenRect,

    /// The current color space.
    color_space: ColorSpace,
}

impl Image {
    fn from_image(image: cairo::ImageSurface, color_space: ColorSpace) -> Self {
        let (w, h) = (image.get_width() as u32, image.get_height() as u32);
        Image {
            image: Rc::new(image),
            region: ScreenRect::new(0, 0, w, h).unwrap(),
            color_space,
        }
    }

    fn into_color_space(self, color_space: ColorSpace) -> Result<Self, Error> {
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

    fn take(self) -> Result<cairo::ImageSurface, Error> {
        match Rc::try_unwrap(self.image) {
            Ok(v) => Ok(v),
            Err(v) => {
                let new_image =
                    cairo::ImageSurface::create(
                        cairo::Format::ARgb32,
                        v.get_width(),
                        v.get_height(),
                    )
                    .map_err(|_| Error::AllocFailed)?;

                let cr = cairo::Context::new(&new_image);
                cr.set_source_surface(&v, 0.0, 0.0);
                cr.paint();

                Ok(new_image)
            }
        }
    }
}

impl std::ops::Deref for Image {
    type Target = cairo::ImageSurface;

    fn deref(&self) -> &Self::Target {
        &self.image
    }
}


struct FilterInputs<'a> {
    source: &'a cairo::ImageSurface,
    background: Option<&'a cairo::ImageSurface>,
    fill_paint: Option<&'a cairo::ImageSurface>,
    stroke_paint: Option<&'a cairo::ImageSurface>,
}


struct FilterResult {
    name: String,
    image: Image,
}

pub fn apply(
    filter: &usvg::Filter,
    bbox: Option<Rect>,
    ts: &usvg::Transform,
    opt: &Options,
    tree: &usvg::Tree,
    background: Option<&cairo::ImageSurface>,
    fill_paint: Option<&cairo::ImageSurface>,
    stroke_paint: Option<&cairo::ImageSurface>,
    canvas: &mut cairo::ImageSurface,
) {
    let res = {
        let inputs = FilterInputs {
            source: canvas,
            background,
            fill_paint,
            stroke_paint,
        };

        _apply(filter, &inputs, bbox, ts, opt, tree)
    };

    let res = res.and_then(|(image, region)| apply_to_canvas(image, region, canvas));

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
    inputs: &FilterInputs,
    bbox: Option<Rect>,
    ts: &usvg::Transform,
    opt: &Options,
    tree: &usvg::Tree,
) -> Result<(Image, ScreenRect), Error> {
    let mut results = Vec::new();
    let region = calc_region(filter, bbox, ts, inputs.source)?;

    for primitive in &filter.children {
        let cs = primitive.color_interpolation;
        let subregion = calc_subregion(filter, primitive, bbox, region, ts, &results)?;

        let mut result = match primitive.kind {
            usvg::FilterKind::FeBlend(ref fe) => {
                let input1 = get_input(&fe.input1, region, inputs, &results)?;
                let input2 = get_input(&fe.input2, region, inputs, &results)?;
                apply_blend(fe, cs, region, input1, input2)
            }
            usvg::FilterKind::FeFlood(ref fe) => {
                apply_flood(fe, region)
            }
            usvg::FilterKind::FeGaussianBlur(ref fe) => {
                let input = get_input(&fe.input, region, inputs, &results)?;
                apply_blur(fe, filter.primitive_units, cs, bbox, ts, input)
            }
            usvg::FilterKind::FeOffset(ref fe) => {
                let input = get_input(&fe.input, region, inputs, &results)?;
                apply_offset(fe, filter.primitive_units, bbox, ts, input)
            }
            usvg::FilterKind::FeComposite(ref fe) => {
                let input1 = get_input(&fe.input1, region, inputs, &results)?;
                let input2 = get_input(&fe.input2, region, inputs, &results)?;
                apply_composite(fe, cs, region, input1, input2)
            }
            usvg::FilterKind::FeMerge(ref fe) => {
                apply_merge(fe, cs, region, inputs, &results)
            }
            usvg::FilterKind::FeTile(ref fe) => {
                let input = get_input(&fe.input, region, inputs, &results)?;
                apply_tile(input, region)
            }
            usvg::FilterKind::FeImage(ref fe) => {
                apply_image(fe, region, subregion, opt, tree, ts)
            }
            usvg::FilterKind::FeComponentTransfer(ref fe) => {
                let input = get_input(&fe.input, region, inputs, &results)?;
                apply_component_transfer(fe, cs, input)
            }
            usvg::FilterKind::FeColorMatrix(ref fe) => {
                let input = get_input(&fe.input, region, inputs, &results)?;
                apply_color_matrix(fe, cs, input)
            }
            usvg::FilterKind::FeConvolveMatrix(ref fe) => {
                let input = get_input(&fe.input, region, inputs, &results)?;
                apply_convolve_matrix(fe, cs, input)
            }
            usvg::FilterKind::FeMorphology(ref fe) => {
                let input = get_input(&fe.input, region, inputs, &results)?;
                apply_morphology(fe, filter.primitive_units, cs, bbox, ts, input)
            }
            usvg::FilterKind::FeDisplacementMap(ref fe) => {
                let input1 = get_input(&fe.input1, region, inputs, &results)?;
                let input2 = get_input(&fe.input2, region, inputs, &results)?;
                apply_displacement_map(fe, region, filter.primitive_units, cs, bbox, ts, input1, input2)
            }
            usvg::FilterKind::FeTurbulence(ref fe) => {
                apply_turbulence(fe, region, cs, ts)
            }
            usvg::FilterKind::FeDiffuseLighting(ref fe) => {
                let input = get_input(&fe.input, region, inputs, &results)?;
                apply_diffuse_lighting(fe, region, cs, ts, input)
            }
            usvg::FilterKind::FeSpecularLighting(ref fe) => {
                let input = get_input(&fe.input, region, inputs, &results)?;
                apply_specular_lighting(fe, region, cs, ts, input)
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
            buffer.clip_region(subregion2);

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

pub(crate) fn calc_region(
    filter: &usvg::Filter,
    bbox: Option<Rect>,
    ts: &usvg::Transform,
    canvas: &cairo::ImageSurface,
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

    let canvas_rect = ScreenRect::new(0, 0, canvas.get_width() as u32, canvas.get_height() as u32).unwrap();
    let region = path.bbox_with_transform(region_ts, None)
        .ok_or_else(|| Error::InvalidRegion)?
        .to_screen_rect()
        .fit_to_rect(canvas_rect);

    Ok(region)
}

/// Returns filter primitive region.
fn calc_subregion(
    filter: &usvg::Filter,
    primitive: &usvg::FilterPrimitive,
    bbox: Option<Rect>,
    filter_region: ScreenRect,
    ts: &usvg::Transform,
    results: &[FilterResult],
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

fn get_input(
    input: &usvg::FilterInput,
    region: ScreenRect,
    inputs: &FilterInputs,
    results: &[FilterResult],
) -> Result<Image, Error> {
    let convert = |in_image: Option<&cairo::ImageSurface>, region| {
        let image = if let Some(image) = in_image {
            image.copy_region(region)?
        } else {
            cairo::ImageSurface::try_create(region.width(), region.height())?
        };

        Ok(Image {
            image: Rc::new(image),
            region: region.translate_to(0, 0),
            color_space: ColorSpace::SRGB,
        })
    };

    let convert_alpha = |mut image: cairo::ImageSurface| {
        // Set RGB to black. Keep alpha as is.
        if let Ok(ref mut data) = image.get_data() {
            for p in data.chunks_mut(4) {
                p[0] = 0;
                p[1] = 0;
                p[2] = 0;
            }
        } else {
            warn!("Cairo surface is already borrowed.");
        }

        Ok(Image {
            image: Rc::new(image),
            region: region.translate_to(0, 0),
            color_space: ColorSpace::SRGB,
        })
    };

    match input {
        usvg::FilterInput::SourceGraphic => {
            let image = inputs.source.copy_region(region)?;

            Ok(Image {
                image: Rc::new(image),
                region: region.translate_to(0, 0),
                color_space: ColorSpace::SRGB,
            })
        }
        usvg::FilterInput::SourceAlpha => {
            let image = inputs.source.copy_region(region)?;
            convert_alpha(image)
        }
        usvg::FilterInput::BackgroundImage => {
            convert(inputs.background, region)
        }
        usvg::FilterInput::BackgroundAlpha => {
            let image = get_input(
                &usvg::FilterInput::BackgroundImage, region, inputs, results,
            )?;
            let image = image.take()?;
            convert_alpha(image)
        }
        usvg::FilterInput::FillPaint => {
            convert(inputs.fill_paint, region.translate_to(0, 0))
        }
        usvg::FilterInput::StrokePaint => {
            convert(inputs.stroke_paint, region.translate_to(0, 0))
        }
        usvg::FilterInput::Reference(ref name) => {
            if let Some(ref v) = results.iter().rev().find(|v| v.name == *name) {
                Ok(v.image.clone())
            } else {
                // Technically unreachable.
                warn!("Unknown filter primitive reference '{}'.", name);
                get_input(&usvg::FilterInput::SourceGraphic, region, inputs, results)
            }
        }
    }
}

fn apply_blur(
    fe: &usvg::FeGaussianBlur,
    units: usvg::Units,
    cs: ColorSpace,
    bbox: Option<Rect>,
    ts: &usvg::Transform,
    input: Image,
) -> Result<Image, Error> {
    // 'A negative value or a value of zero disables the effect of the given filter primitive
    // (i.e., the result is the filter input image).'
    if fe.std_dev_x.is_zero() && fe.std_dev_y.is_zero() {
        return Ok(input);
    }

    let (std_dx, std_dy) = try_opt_or!(scale_coordinates(
        fe.std_dev_x.value(), fe.std_dev_y.value(), units, bbox, ts,
    ), Ok(input));

    if std_dx.is_fuzzy_zero() && std_dy.is_fuzzy_zero() {
        return Ok(input);
    }

    const BLUR_SIGMA_THRESHOLD: f64 = 2.0;
    // Check that the current feGaussianBlur filter can be applied using a box blur.
    let use_box_blur = std_dx >= BLUR_SIGMA_THRESHOLD || std_dy >= BLUR_SIGMA_THRESHOLD;

    let mut buffer = input.into_color_space(cs)?.take()?;
    let (w, h) = (buffer.get_width() as u32, buffer.get_height() as u32);
    if let Ok(ref mut data) = buffer.get_data() {
        let img = svgfilters::ImageRefMut::new(data.as_bgra_mut(), w, h);
        if use_box_blur {
            svgfilters::box_blur(std_dx, std_dy, img);
        } else {
            svgfilters::iir_blur(std_dx, std_dy, img);
        }
    }

    Ok(Image::from_image(buffer, cs))
}

fn apply_offset(
    fe: &usvg::FeOffset,
    units: usvg::Units,
    bbox: Option<Rect>,
    ts: &usvg::Transform,
    input: Image,
) -> Result<Image, Error> {
    let (dx, dy) = try_opt_or!(scale_coordinates(fe.dx, fe.dy, units, bbox, ts), Ok(input));
    if dx.is_fuzzy_zero() && dy.is_fuzzy_zero() {
        return Ok(input);
    }

    // TODO: do not use an additional buffer
    let buffer =
        cairo::ImageSurface::create(
            cairo::Format::ARgb32,
            input.image.get_width(),
            input.image.get_height(),
        )
        .map_err(|_| Error::AllocFailed)?;

    let cr = cairo::Context::new(&buffer);
    cr.set_source_surface(&input, dx, dy);
    cr.paint();

    Ok(Image::from_image(buffer, input.color_space))
}

fn apply_blend(
    fe: &usvg::FeBlend,
    cs: ColorSpace,
    region: ScreenRect,
    input1: Image,
    input2: Image,
) -> Result<Image, Error> {
    let input1 = input1.into_color_space(cs)?;
    let input2 = input2.into_color_space(cs)?;

    let buffer = cairo::ImageSurface::try_create(region.width(), region.height())?;
    let cr = cairo::Context::new(&buffer);

    cr.set_source_surface(&input2, 0.0, 0.0);
    cr.paint();

    let operator = match fe.mode {
        usvg::FeBlendMode::Normal => cairo::Operator::Over,
        usvg::FeBlendMode::Multiply => cairo::Operator::Multiply,
        usvg::FeBlendMode::Screen => cairo::Operator::Screen,
        usvg::FeBlendMode::Darken => cairo::Operator::Darken,
        usvg::FeBlendMode::Lighten => cairo::Operator::Lighten,
    };

    cr.set_operator(operator);
    cr.set_source_surface(&input1, 0.0, 0.0);
    cr.paint();

    Ok(Image::from_image(buffer, cs))
}

fn apply_composite(
    fe: &usvg::FeComposite,
    cs: ColorSpace,
    region: ScreenRect,
    input1: Image,
    input2: Image,
) -> Result<Image, Error> {
    let mut buffer1 = input1.into_color_space(cs)?.take()?;
    let mut buffer2 = input2.into_color_space(cs)?.take()?;

    let mut buffer = cairo::ImageSurface::try_create(region.width(), region.height())?;

    if let Operator::Arithmetic { k1, k2, k3, k4 } = fe.operator {
        let (w, h) = (region.width(), region.height());
        svgfilters::arithmetic_composite(
            k1, k2, k3, k4,
            svgfilters::ImageRef::new(buffer1.get_data().unwrap().as_bgra(), w, h),
            svgfilters::ImageRef::new(buffer2.get_data().unwrap().as_bgra(), w, h),
            svgfilters::ImageRefMut::new(buffer.get_data().unwrap().as_bgra_mut(), w, h),
        );

        return Ok(Image::from_image(buffer, cs));
    }

    let cr = cairo::Context::new(&buffer);

    cr.set_source_surface(&buffer2, 0.0, 0.0);
    cr.paint();

    use usvg::FeCompositeOperator as Operator;
    let operator = match fe.operator {
        Operator::Over => cairo::Operator::Over,
        Operator::In => cairo::Operator::In,
        Operator::Out => cairo::Operator::Out,
        Operator::Atop => cairo::Operator::Atop,
        Operator::Xor => cairo::Operator::Xor,
        Operator::Arithmetic { .. } => cairo::Operator::Over,
    };

    cr.set_operator(operator);
    cr.set_source_surface(&buffer1, 0.0, 0.0);
    cr.paint();

    Ok(Image::from_image(buffer, cs))
}

fn apply_merge(
    fe: &usvg::FeMerge,
    cs: ColorSpace,
    region: ScreenRect,
    inputs: &FilterInputs,
    results: &[FilterResult],
) -> Result<Image, Error> {
    let buffer = cairo::ImageSurface::try_create(region.width(), region.height())?;
    let cr = cairo::Context::new(&buffer);

    for input in &fe.inputs {
        let input = get_input(input, region, inputs, results)?;
        let input = input.into_color_space(cs)?;

        cr.set_source_surface(&input, 0.0, 0.0);
        cr.paint();
    }

    Ok(Image::from_image(buffer, cs))
}

fn apply_flood(
    fe: &usvg::FeFlood,
    region: ScreenRect,
) -> Result<Image, Error> {
    let buffer = cairo::ImageSurface::try_create(region.width(), region.height())?;

    let cr = cairo::Context::new(&buffer);
    cr.set_source_color(fe.color, fe.opacity);
    cr.paint();

    Ok(Image::from_image(buffer, ColorSpace::SRGB))
}

fn apply_tile(
    input: Image,
    region: ScreenRect,
) -> Result<Image, Error> {
    let buffer = cairo::ImageSurface::try_create(region.width(), region.height())?;

    let subregion = input.region.translate(-region.x(), -region.y());

    let tile = input.image.copy_region(subregion)?;
    let brush_ts = usvg::Transform::new_translate(subregion.x() as f64, subregion.y() as f64);

    let patt = cairo::SurfacePattern::create(&tile);
    patt.set_extend(cairo::Extend::Repeat);
    patt.set_filter(cairo::Filter::Best);

    let cr = cairo::Context::new(&buffer);
    let mut m: cairo::Matrix = brush_ts.to_native();
    m.invert();
    patt.set_matrix(m);

    cr.set_source(&patt);
    cr.rectangle(0.0, 0.0, region.width() as f64, region.height() as f64);
    cr.fill();

    Ok(Image::from_image(buffer, ColorSpace::SRGB))
}

fn apply_image(
    fe: &usvg::FeImage,
    region: ScreenRect,
    subregion: ScreenRect,
    opt: &Options,
    tree: &usvg::Tree,
    ts: &usvg::Transform,
) -> Result<Image, Error> {
    let buffer = cairo::ImageSurface::try_create(region.width(), region.height())?;

    match fe.data {
        usvg::FeImageKind::Image(ref data, format) => {
            let cr = cairo::Context::new(&buffer);

            let dx = (subregion.x() - region.x()) as f64;
            let dy = (subregion.y() - region.y()) as f64;
            cr.translate(dx, dy);

            let view_box = usvg::ViewBox {
                rect: subregion.translate_to(0, 0).to_rect(),
                aspect: fe.aspect,
            };

            if format == usvg::ImageFormat::SVG {
                super::image::draw_svg(data, view_box, opt, &cr);
            } else {
                super::image::draw_raster(format, data, view_box, fe.rendering_mode, opt, &cr);
            }
        }
        usvg::FeImageKind::Use(ref id) => {
            if let Some(ref node) = tree.defs_by_id(id).or(tree.node_by_id(id)) {
                let mut layers = Layers::new(region.size());
                let cr = cairo::Context::new(&buffer);

                let (sx, sy) = ts.get_scale();
                cr.scale(sx, sy);
                cr.transform(node.transform().to_native());

                crate::render::render_node(node, opt, &mut RenderState::Ok, &mut layers, &cr);
            }
        }
    }

    Ok(Image::from_image(buffer, ColorSpace::SRGB))
}

fn apply_component_transfer(
    fe: &usvg::FeComponentTransfer,
    cs: ColorSpace,
    input: Image,
) -> Result<Image, Error> {
    let mut buffer = input.into_color_space(cs)?.take()?;
    let (w, h) = (buffer.get_width() as u32, buffer.get_height() as u32);
    if let Ok(ref mut data) = buffer.get_data() {
        svgfilters::demultiply_alpha(data.as_bgra_mut());

        svgfilters::component_transfer(
            fe.func_b.into_svgf(),
            fe.func_g.into_svgf(),
            fe.func_r.into_svgf(),
            fe.func_a.into_svgf(),
            svgfilters::ImageRefMut::new(data.as_bgra_mut(), w, h),
        );

        svgfilters::multiply_alpha(data.as_bgra_mut());
    }

    Ok(Image::from_image(buffer, cs))
}

fn apply_color_matrix(
    fe: &usvg::FeColorMatrix,
    cs: ColorSpace,
    input: Image,
) -> Result<Image, Error> {
    let mut buffer = input.into_color_space(cs)?.take()?;
    let (w, h) = (buffer.get_width() as u32, buffer.get_height() as u32);
    if let Ok(ref mut data) = buffer.get_data() {
        use std::convert::TryInto;

        let kind = match fe.kind {
            usvg::FeColorMatrixKind::Matrix(ref data) =>
                svgfilters::ColorMatrix::Matrix(data.as_slice().try_into().unwrap()),
            usvg::FeColorMatrixKind::Saturate(n) =>
                svgfilters::ColorMatrix::Saturate(svgfilters::NormalizedValue::new(n.value())),
            usvg::FeColorMatrixKind::HueRotate(n) =>
                svgfilters::ColorMatrix::HueRotate(n),
            usvg::FeColorMatrixKind::LuminanceToAlpha =>
                svgfilters::ColorMatrix::LuminanceToAlpha,
        };

        svgfilters::demultiply_alpha(data.as_bgra_mut());
        svgfilters::color_matrix(
            kind, svgfilters::ImageRefMut::new(data.as_bgra_mut(), w, h),
        );
        svgfilters::multiply_alpha(data.as_bgra_mut());
    }

    Ok(Image::from_image(buffer, cs))
}

fn apply_convolve_matrix(
    fe: &usvg::FeConvolveMatrix,
    cs: ColorSpace,
    input: Image,
) -> Result<Image, Error> {
    let mut buffer = input.into_color_space(cs)?.take()?;

    if fe.preserve_alpha {
        if let Ok(ref mut data) = buffer.get_data() {
            svgfilters::demultiply_alpha(data.as_bgra_mut());
        }
    }

    let (w, h) = (buffer.get_width() as u32, buffer.get_height() as u32);
    if let Ok(ref mut data) = buffer.get_data() {
        let matrix = svgfilters::ConvolveMatrix::new(
            fe.matrix.target_x(), fe.matrix.target_y(),
            fe.matrix.columns(), fe.matrix.rows(),
            fe.matrix.data(),
        ).unwrap();

        let edge_mode = match fe.edge_mode {
            usvg::FeEdgeMode::None => svgfilters::EdgeMode::None,
            usvg::FeEdgeMode::Duplicate => svgfilters::EdgeMode::Duplicate,
            usvg::FeEdgeMode::Wrap => svgfilters::EdgeMode::Wrap,
        };

        svgfilters::convolve_matrix(
            matrix, fe.divisor.value(), fe.bias,
            edge_mode, fe.preserve_alpha,
            svgfilters::ImageRefMut::new(data.as_bgra_mut(), w, h),
        );
    }

    Ok(Image::from_image(buffer, cs))
}

fn apply_morphology(
    fe: &usvg::FeMorphology,
    units: usvg::Units,
    cs: ColorSpace,
    bbox: Option<Rect>,
    ts: &usvg::Transform,
    input: Image,
) -> Result<Image, Error> {
    let mut buffer = input.into_color_space(cs)?.take()?;
    let (rx, ry) = try_opt_or!(
        scale_coordinates(fe.radius_x.value(), fe.radius_y.value(), units, bbox, ts),
        Ok(Image::from_image(buffer, cs))
    );

    if !(rx > 0.0 && ry > 0.0) {
        buffer.clear();
        return Ok(Image::from_image(buffer, cs));
    }

    let (w, h) = (buffer.get_width() as u32, buffer.get_height() as u32);
    if let Ok(ref mut data) = buffer.get_data() {
        let operator = match fe.operator {
            usvg::FeMorphologyOperator::Erode => svgfilters::MorphologyOperator::Erode,
            usvg::FeMorphologyOperator::Dilate => svgfilters::MorphologyOperator::Dilate,
        };

        svgfilters::morphology(
            operator, rx, ry,
            svgfilters::ImageRefMut::new(data.as_bgra_mut(), w, h),
        );
    }

    Ok(Image::from_image(buffer, cs))
}

fn apply_displacement_map(
    fe: &usvg::FeDisplacementMap,
    region: ScreenRect,
    units: usvg::Units,
    cs: ColorSpace,
    bbox: Option<Rect>,
    ts: &usvg::Transform,
    input1: Image,
    input2: Image,
) -> Result<Image, Error> {
    let mut buffer1 = input1.into_color_space(cs)?.take()?;
    let mut buffer2 = input2.into_color_space(cs)?.take()?;
    let (sx, sy) = try_opt_or!(
        scale_coordinates(fe.scale, fe.scale, units, bbox, ts),
        Ok(Image::from_image(buffer1, cs))
    );

    let mut buffer = cairo::ImageSurface::try_create(region.width(), region.height())?;

    let (w, h) = (buffer.get_width() as u32, buffer.get_height() as u32);
    if let (Ok(buffer1), Ok(buffer2), Ok(mut buffer))
        = (buffer1.get_data(), buffer2.get_data(), buffer.get_data())
    {
        svgfilters::displacement_map(
            fe.x_channel_selector.into_svgf(),
            fe.y_channel_selector.into_svgf(),
            sx, sy,
            svgfilters::ImageRef::new(buffer1.as_bgra(), w, h),
            svgfilters::ImageRef::new(buffer2.as_bgra(), w, h),
            svgfilters::ImageRefMut::new(buffer.as_bgra_mut(), w, h),
        );
    }

    Ok(Image::from_image(buffer, cs))
}

fn apply_turbulence(
    fe: &usvg::FeTurbulence,
    region: ScreenRect,
    cs: ColorSpace,
    ts: &usvg::Transform,
) -> Result<Image, Error> {
    let mut buffer = cairo::ImageSurface::try_create(region.width(), region.height())?;
    let (sx, sy) = ts.get_scale();
    if sx.is_fuzzy_zero() || sy.is_fuzzy_zero() {
        return Ok(Image::from_image(buffer, cs));
    }

    let (w, h) = (buffer.get_width() as u32, buffer.get_height() as u32);
    if let Ok(ref mut data) = buffer.get_data() {
        svgfilters::turbulence(
            region.x() as f64, region.y() as f64,
            sx, sy,
            fe.base_frequency.x.value().into(), fe.base_frequency.y.value().into(),
            fe.num_octaves,
            fe.seed,
            fe.stitch_tiles,
            fe.kind == usvg::FeTurbulenceKind::FractalNoise,
            svgfilters::ImageRefMut::new(data.as_bgra_mut(), w, h),
        );

        svgfilters::multiply_alpha(data.as_bgra_mut());
    }

    Ok(Image::from_image(buffer, cs))
}

fn apply_diffuse_lighting(
    fe: &usvg::FeDiffuseLighting,
    region: ScreenRect,
    cs: ColorSpace,
    ts: &usvg::Transform,
    input: Image,
) -> Result<Image, Error> {
    let mut input = input.take()?;
    let mut buffer = cairo::ImageSurface::try_create(region.width(), region.height())?;

    let light_source = fe.light_source.transform(region, ts);

    let (w, h) = (buffer.get_width() as u32, buffer.get_height() as u32);
    if let (Ok(ref buf_in), Ok(ref mut buf_out)) = (input.get_data(), buffer.get_data()) {
        svgfilters::diffuse_lighting(
            fe.surface_scale,
            fe.diffuse_constant,
            fe.lighting_color.into_svgf(),
            light_source.into_svgf(),
            svgfilters::ImageRef::new(buf_in.as_bgra(), w, h),
            svgfilters::ImageRefMut::new(buf_out.as_bgra_mut(), w, h),
        );
    }

    Ok(Image::from_image(buffer, cs))
}

fn apply_specular_lighting(
    fe: &usvg::FeSpecularLighting,
    region: ScreenRect,
    cs: ColorSpace,
    ts: &usvg::Transform,
    input: Image,
) -> Result<Image, Error> {
    let mut input = input.take()?;
    let mut buffer = cairo::ImageSurface::try_create(region.width(), region.height())?;

    let light_source = fe.light_source.transform(region, ts);

    let (w, h) = (buffer.get_width() as u32, buffer.get_height() as u32);
    if let (Ok(ref buf_in), Ok(ref mut buf_out)) = (input.get_data(), buffer.get_data()) {
        svgfilters::specular_lighting(
            fe.surface_scale,
            fe.specular_constant,
            fe.specular_exponent,
            fe.lighting_color.into_svgf(),
            light_source.into_svgf(),
            svgfilters::ImageRef::new(buf_in.as_bgra(), w, h),
            svgfilters::ImageRefMut::new(buf_out.as_bgra_mut(), w, h),
        );
    }

    Ok(Image::from_image(buffer, cs))
}

fn apply_to_canvas(
    input: Image,
    region: ScreenRect,
    canvas: &mut cairo::ImageSurface,
) -> Result<(), Error> {
    let input = input.into_color_space(ColorSpace::SRGB)?;

    let cr = cairo::Context::new(canvas);

    cr.set_operator(cairo::Operator::Clear);
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
    cr.paint();

    cr.set_operator(cairo::Operator::Over);
    cr.set_source_surface(&input, region.x() as f64, region.y() as f64);
    cr.paint();

    Ok(())
}

/// Converts coordinates from `objectBoundingBox` to the `userSpaceOnUse`.
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
