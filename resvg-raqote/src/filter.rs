// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::cmp;
use std::rc::Rc;

use usvg::ColorInterpolation as ColorSpace;
use rgb::FromSlice;
use log::warn;

use crate::render::prelude::*;

macro_rules! into_svgfilters_image {
    ($img:expr) => { svgfilters::ImageRef::new($img.get_data_u8().as_bgra(), $img.width() as u32, $img.height() as u32) };
}

macro_rules! into_svgfilters_image_mut {
    ($img:expr) => { into_svgfilters_image_mut($img.width() as u32, $img.height() as u32, &mut $img) };
}

// We need a macro and a function to resolve lifetimes.
fn into_svgfilters_image_mut(width: u32, height: u32, data: &mut raqote::DrawTarget)
    -> svgfilters::ImageRefMut
{
    svgfilters::ImageRefMut::new(data.get_data_u8_mut().as_bgra_mut(), width, height)
}


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
    InvalidRegion,
    NoResults,
}

#[derive(Clone)]
struct Image {
    /// Filter primitive result.
    ///
    /// All images have the same size which is equal to the current filter region.
    image: Rc<raqote::DrawTarget>,

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
    fn from_image(image: raqote::DrawTarget, color_space: ColorSpace) -> Self {
        let (w, h) = (image.width() as u32, image.height() as u32);
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

    fn take(self) -> Result<raqote::DrawTarget, Error> {
        match Rc::try_unwrap(self.image) {
            Ok(v) => Ok(v),
            Err(v) => {
                let mut dt = raqote::DrawTarget::new(v.width(), v.height());
                dt.draw_image_at(0.0, 0.0, &v.as_image(), &raqote::DrawOptions {
                    blend_mode: raqote::BlendMode::Src,
                    ..raqote::DrawOptions::default()
                });

                Ok(dt)
            }
        }
    }

    fn width(&self) -> u32 {
        self.image.width() as u32
    }

    fn height(&self) -> u32 {
        self.image.height() as u32
    }

    fn as_ref(&self) -> &raqote::DrawTarget {
        &self.image
    }
}


struct FilterInputs<'a> {
    source: &'a raqote::DrawTarget,
    background: Option<&'a raqote::DrawTarget>,
    fill_paint: Option<&'a raqote::DrawTarget>,
    stroke_paint: Option<&'a raqote::DrawTarget>,
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
    background: Option<&raqote::DrawTarget>,
    fill_paint: Option<&raqote::DrawTarget>,
    stroke_paint: Option<&raqote::DrawTarget>,
    canvas: &mut raqote::DrawTarget,
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
        canvas.make_transparent();
    }

    match res {
        Ok(_) => {}
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
        // let cs = primitive.color_interpolation;
        let subregion = calc_subregion(filter, primitive, bbox, region, ts, &results)?;
        let mut result = apply2(filter, primitive, inputs, &results, bbox, region, subregion, ts, opt, tree)?;

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

fn apply2(
    filter: &usvg::Filter,
    primitive: &usvg::FilterPrimitive,
    inputs: &FilterInputs,
    results: &[FilterResult],
    bbox: Option<Rect>,
    region: ScreenRect,
    subregion: ScreenRect,
    ts: &usvg::Transform,
    opt: &Options,
    tree: &usvg::Tree,
) -> Result<Image, Error> {
    let cs = primitive.color_interpolation;
    match primitive.kind {
        usvg::FilterKind::FeBlend(ref fe) => {
            let input1 = get_input(&fe.input1, region, inputs, &results)?;
            let input2 = get_input(&fe.input2, region, inputs, &results)?;

            let input1 = input1.into_color_space(cs)?;
            let input2 = input2.into_color_space(cs)?;

            let mut dt = create_image(region.width(), region.height())?;
            let draw_opt = raqote::DrawOptions {
                blend_mode: raqote::BlendMode::Src,
                ..raqote::DrawOptions::default()
            };
            dt.draw_image_at(0.0, 0.0, &input2.as_ref().as_image(), &draw_opt);

            let blend_mode = match fe.mode {
                usvg::FeBlendMode::Normal => raqote::BlendMode::SrcOver,
                usvg::FeBlendMode::Multiply => raqote::BlendMode::Multiply,
                usvg::FeBlendMode::Screen => raqote::BlendMode::Screen,
                usvg::FeBlendMode::Darken => raqote::BlendMode::Darken,
                usvg::FeBlendMode::Lighten => raqote::BlendMode::Lighten,
            };

            let draw_opt = raqote::DrawOptions { blend_mode, ..raqote::DrawOptions::default() };
            dt.draw_image_at(0.0, 0.0, &input1.as_ref().as_image(), &draw_opt);

            Ok(Image::from_image(dt, cs))
        }
        usvg::FilterKind::FeFlood(ref fe) => {
            let mut dt = create_image(region.width(), region.height())?;

            let alpha = (fe.opacity.value() * 255.0) as u8;
            dt.clear(fe.color.to_solid(alpha));

            Ok(Image::from_image(dt, ColorSpace::SRGB))
        }
        usvg::FilterKind::FeGaussianBlur(ref fe) => {
            let input = get_input(&fe.input, region, inputs, &results)?;
            let (std_dx, std_dy, box_blur)
                = try_opt_or!(resolve_std_dev(fe, filter.primitive_units, bbox, ts), Ok(input));

            let mut buffer = input.into_color_space(cs)?.take()?;
            if box_blur {
                svgfilters::box_blur(std_dx, std_dy, into_svgfilters_image_mut!(buffer));
            } else {
                svgfilters::iir_blur(std_dx, std_dy, into_svgfilters_image_mut!(buffer));
            }

            Ok(Image::from_image(buffer, cs))
        }
        usvg::FilterKind::FeOffset(ref fe) => {
            let input = get_input(&fe.input, region, inputs, &results)?;
            let (dx, dy) = try_opt_or!(scale_coordinates(fe.dx, fe.dy, filter.primitive_units, bbox, ts), Ok(input));
            if dx.is_fuzzy_zero() && dy.is_fuzzy_zero() {
                return Ok(input);
            }

            // TODO: do not use an additional buffer
            let mut dt = create_image(input.width(), input.height())?;
            dt.draw_image_at(
                dx as f32, dy as f32, &input.as_ref().as_image(), &raqote::DrawOptions::default(),
            );

            Ok(Image::from_image(dt, input.color_space))
        }
        usvg::FilterKind::FeComposite(ref fe) => {
            let input1 = get_input(&fe.input1, region, inputs, &results)?;
            let input2 = get_input(&fe.input2, region, inputs, &results)?;
            apply_composite(fe, cs, region, input1, input2)
        }
        usvg::FilterKind::FeMerge(ref fe) => {
            let mut dt = create_image(region.width(), region.height())?;

            for input in &fe.inputs {
                let input = get_input(input, region, inputs, results)?;
                let input = input.into_color_space(cs)?;
                dt.draw_image_at(0.0, 0.0, &input.as_ref().as_image(), &raqote::DrawOptions::default());
            }

            Ok(Image::from_image(dt, cs))
        }
        usvg::FilterKind::FeTile(ref fe) => {
            let input = get_input(&fe.input, region, inputs, &results)?;
            let mut dt = create_image(region.width(), region.height())?;

            let subregion = input.region.translate(-region.x(), -region.y());

            let tile = copy_image(&input.image, subregion)?;
            let brush_ts = usvg::Transform::new_translate(subregion.x() as f64, subregion.y() as f64);

            let ts: raqote::Transform = brush_ts.to_native();
            let ts = ts.inverse().unwrap();
            let patt = raqote::Source::Image(
                tile.as_image(),
                raqote::ExtendMode::Repeat,
                raqote::FilterMode::Bilinear,
                ts,
            );

            let mut pb = raqote::PathBuilder::new();
            pb.rect(0.0, 0.0, region.width() as f32, region.height() as f32);
            dt.fill(&pb.finish(), &patt, &raqote::DrawOptions::default());

            dt.set_transform(&raqote::Transform::default());
            Ok(Image::from_image(dt, ColorSpace::SRGB))
        }
        usvg::FilterKind::FeImage(ref fe) => {
            apply_image(fe, region, subregion, opt, tree, ts)
        }
        usvg::FilterKind::FeComponentTransfer(ref fe) => {
            let input = get_input(&fe.input, region, inputs, &results)?;
            let mut buffer = input.into_color_space(cs)?.take()?;

            svgfilters::demultiply_alpha(buffer.get_data_u8_mut().as_bgra_mut());

            svgfilters::component_transfer(
                fe.func_b.into_svgf(),
                fe.func_g.into_svgf(),
                fe.func_r.into_svgf(),
                fe.func_a.into_svgf(),
                into_svgfilters_image_mut!(buffer),
            );

            svgfilters::multiply_alpha(buffer.get_data_u8_mut().as_bgra_mut());

            Ok(Image::from_image(buffer, cs))
        }
        usvg::FilterKind::FeColorMatrix(ref fe) => {
            use std::convert::TryInto;

            let input = get_input(&fe.input, region, inputs, &results)?;
            let mut buffer = input.into_color_space(cs)?.take()?;

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

            svgfilters::demultiply_alpha(buffer.get_data_u8_mut().as_bgra_mut());
            svgfilters::color_matrix(kind, into_svgfilters_image_mut!(buffer));
            svgfilters::multiply_alpha(buffer.get_data_u8_mut().as_bgra_mut());

            Ok(Image::from_image(buffer, cs))
        }
        usvg::FilterKind::FeConvolveMatrix(ref fe) => {
            let input = get_input(&fe.input, region, inputs, &results)?;
            let mut buffer = input.into_color_space(cs)?.take()?;

            if fe.preserve_alpha {
                svgfilters::demultiply_alpha(buffer.get_data_u8_mut().as_bgra_mut());
            }

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
                matrix, fe.divisor.value(), fe.bias, edge_mode, fe.preserve_alpha,
                into_svgfilters_image_mut!(buffer),
            );

            Ok(Image::from_image(buffer, cs))
        }
        usvg::FilterKind::FeMorphology(ref fe) => {
            let input = get_input(&fe.input, region, inputs, &results)?;
            let mut buffer = input.into_color_space(cs)?.take()?;
            let (rx, ry) = try_opt_or!(
                scale_coordinates(fe.radius_x.value(), fe.radius_y.value(), filter.primitive_units, bbox, ts),
                Ok(Image::from_image(buffer, cs))
            );

            if !(rx > 0.0 && ry > 0.0) {
                buffer.make_transparent();
                return Ok(Image::from_image(buffer, cs));
            }

            let operator = match fe.operator {
                usvg::FeMorphologyOperator::Erode => svgfilters::MorphologyOperator::Erode,
                usvg::FeMorphologyOperator::Dilate => svgfilters::MorphologyOperator::Dilate,
            };

            svgfilters::morphology(operator, rx, ry, into_svgfilters_image_mut!(buffer));

            Ok(Image::from_image(buffer, cs))
        }
        usvg::FilterKind::FeDisplacementMap(ref fe) => {
            let input1 = get_input(&fe.input1, region, inputs, &results)?;
            let input2 = get_input(&fe.input2, region, inputs, &results)?;
            let buffer1 = input1.into_color_space(cs)?.take()?;
            let buffer2 = input2.into_color_space(cs)?.take()?;

            let mut buffer = create_image(region.width(), region.height())?;
            let (sx, sy) = try_opt_or!(
                scale_coordinates(fe.scale, fe.scale, filter.primitive_units, bbox, ts),
                Ok(Image::from_image(buffer, cs))
            );

            svgfilters::displacement_map(
                fe.x_channel_selector.into_svgf(),
                fe.y_channel_selector.into_svgf(),
                sx, sy,
                into_svgfilters_image!(buffer1),
                into_svgfilters_image!(buffer2),
                into_svgfilters_image_mut!(buffer),
            );

            Ok(Image::from_image(buffer, cs))
        }
        usvg::FilterKind::FeTurbulence(ref fe) => {
            let mut buffer = create_image(region.width(), region.height())?;

            let (sx, sy) = ts.get_scale();
            if sx.is_fuzzy_zero() || sy.is_fuzzy_zero() {
                return Ok(Image::from_image(buffer, cs));
            }

            svgfilters::turbulence(
                region.x() as f64, region.y() as f64,
                sx, sy,
                fe.base_frequency.x.value().into(), fe.base_frequency.y.value().into(),
                fe.num_octaves,
                fe.seed,
                fe.stitch_tiles,
                fe.kind == usvg::FeTurbulenceKind::FractalNoise,
                into_svgfilters_image_mut!(buffer),
            );

            svgfilters::multiply_alpha(buffer.get_data_u8_mut().as_bgra_mut());

            Ok(Image::from_image(buffer, cs))
        }
        usvg::FilterKind::FeDiffuseLighting(ref fe) => {
            let input = get_input(&fe.input, region, inputs, &results)?;
            let mut buffer = create_image(region.width(), region.height())?;

            let light_source = fe.light_source.transform(region, ts);

            svgfilters::diffuse_lighting(
                fe.surface_scale,
                fe.diffuse_constant,
                fe.lighting_color.into_svgf(),
                light_source.into_svgf(),
                into_svgfilters_image!(input.as_ref()),
                into_svgfilters_image_mut!(buffer),
            );

            Ok(Image::from_image(buffer, cs))
        }
        usvg::FilterKind::FeSpecularLighting(ref fe) => {
            let input = get_input(&fe.input, region, inputs, &results)?;
            let mut buffer = create_image(region.width(), region.height())?;

            let light_source = fe.light_source.transform(region, ts);

            svgfilters::specular_lighting(
                fe.surface_scale,
                fe.specular_constant,
                fe.specular_exponent,
                fe.lighting_color.into_svgf(),
                light_source.into_svgf(),
                into_svgfilters_image!(input.as_ref()),
                into_svgfilters_image_mut!(buffer),
            );

            Ok(Image::from_image(buffer, cs))
        }
    }
}

fn create_image(width: u32, height: u32) -> Result<raqote::DrawTarget, Error> {
    Ok(raqote::DrawTarget::new(width as i32, height as i32))
}

fn copy_image(
    image: &raqote::DrawTarget,
    region: ScreenRect,
) -> Result<raqote::DrawTarget, Error> {
    let x = cmp::max(0, region.x()) as f32;
    let y = cmp::max(0, region.y()) as f32;

    let mut new_image = create_image(region.width(), region.height())?;

    new_image.draw_image_at(-x, -y, &image.as_image(), &raqote::DrawOptions {
        blend_mode: raqote::BlendMode::Src,
        ..raqote::DrawOptions::default()
    });

    Ok(new_image)
}

pub(crate) fn calc_region(
    filter: &usvg::Filter,
    bbox: Option<Rect>,
    ts: &usvg::Transform,
    canvas: &raqote::DrawTarget,
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

    let canvas_rect = ScreenRect::new(0, 0, canvas.width() as u32, canvas.height() as u32).unwrap();
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
    let convert = |in_image, region| {
        let image = if let Some(image) = in_image {
            copy_image(image, region)?
        } else {
            create_image(region.width(), region.height())?
        };

        Ok(Image {
            image: Rc::new(image),
            region: region.translate_to(0, 0),
            color_space: ColorSpace::SRGB,
        })
    };

    let convert_alpha = |mut image: raqote::DrawTarget| {
        // Set RGB to black. Keep alpha as is.
        let data = image.get_data_u8_mut();
        for p in data.chunks_mut(4) {
            p[0] = 0;
            p[1] = 0;
            p[2] = 0;
        }

        Ok(Image {
            image: Rc::new(image),
            region: region.translate_to(0, 0),
            color_space: ColorSpace::SRGB,
        })
    };

    match input {
        usvg::FilterInput::SourceGraphic => {
            let image = copy_image(inputs.source, region)?;

            Ok(Image {
                image: Rc::new(image),
                region: region.translate_to(0, 0),
                color_space: ColorSpace::SRGB,
            })
        }
        usvg::FilterInput::SourceAlpha => {
            let image = copy_image(inputs.source, region)?;
            convert_alpha(image)
        }
        usvg::FilterInput::BackgroundImage => {
            convert(inputs.background, region)
        }
        usvg::FilterInput::BackgroundAlpha => {
            let image = get_input(
                &usvg::FilterInput::BackgroundImage, region, inputs, results,
            )?;
            convert_alpha(image.take()?)
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
                get_input(
                    &usvg::FilterInput::SourceGraphic, region, inputs, results,
                )
            }
        }
    }
}

fn apply_composite(
    fe: &usvg::FeComposite,
    cs: ColorSpace,
    region: ScreenRect,
    input1: Image,
    input2: Image,
) -> Result<Image, Error> {
    let buffer1 = input1.into_color_space(cs)?.take()?;
    let buffer2 = input2.into_color_space(cs)?.take()?;

    let mut dt = create_image(region.width(), region.height())?;

    if let Operator::Arithmetic { k1, k2, k3, k4 } = fe.operator {
        let (w, h) = (region.width(), region.height());
        svgfilters::arithmetic_composite(
            k1, k2, k3, k4,
            svgfilters::ImageRef::new(buffer1.get_data_u8().as_bgra(), w, h),
            svgfilters::ImageRef::new(buffer2.get_data_u8().as_bgra(), w, h),
            svgfilters::ImageRefMut::new(dt.get_data_u8_mut().as_bgra_mut(), w, h),
        );

        return Ok(Image::from_image(dt, cs));
    }

    let draw_opt = raqote::DrawOptions {
        blend_mode: raqote::BlendMode::Src,
        ..raqote::DrawOptions::default()
    };
    dt.draw_image_at(0.0, 0.0, &buffer2.as_image(), &draw_opt);

    use usvg::FeCompositeOperator as Operator;
    let blend_mode = match fe.operator {
        Operator::Over => raqote::BlendMode::SrcOver,
        Operator::In => raqote::BlendMode::SrcIn,
        Operator::Out => raqote::BlendMode::SrcOut,
        Operator::Atop => raqote::BlendMode::SrcAtop,
        Operator::Xor => raqote::BlendMode::Xor,
        Operator::Arithmetic { .. } => raqote::BlendMode::SrcOver,
    };

    let draw_opt = raqote::DrawOptions { blend_mode, ..raqote::DrawOptions::default() };
    dt.draw_image_at(0.0, 0.0, &buffer1.as_image(), &draw_opt);

    Ok(Image::from_image(dt, cs))
}

fn apply_image(
    fe: &usvg::FeImage,
    region: ScreenRect,
    subregion: ScreenRect,
    opt: &Options,
    tree: &usvg::Tree,
    ts: &usvg::Transform,
) -> Result<Image, Error> {
    let mut dt = create_image(region.width(), region.height())?;

    match fe.data {
        usvg::FeImageKind::Image(ref data, format) => {
            let dx = (subregion.x() - region.x()) as f64;
            let dy = (subregion.y() - region.y()) as f64;
            let ctm = dt.get_transform().pre_translate(raqote::Vector::new(dx as f32, dy as f32));
            dt.set_transform(&ctm);

            let view_box = usvg::ViewBox {
                rect: subregion.translate_to(0, 0).to_rect(),
                aspect: fe.aspect,
            };

            if format == usvg::ImageFormat::SVG {
                super::image::draw_svg(data, view_box, opt, &mut dt);
            } else {
                super::image::draw_raster(
                    format, data, view_box, fe.rendering_mode, opt, &mut dt
                );
            }
        }
        usvg::FeImageKind::Use(ref id) => {
            if let Some(ref node) = tree.defs_by_id(id).or(tree.node_by_id(id)) {
                let mut layers = Layers::new(region.size());

                let (sx, sy) = ts.get_scale();
                dt.transform(&usvg::Transform::new_scale(sx, sy).to_native());
                dt.transform(&node.transform().to_native());

                render_node(node, opt, &mut RenderState::Ok, &mut layers, &mut dt);
            }
        }
    }

    dt.set_transform(&raqote::Transform::default());
    Ok(Image::from_image(dt, ColorSpace::SRGB))
}

fn apply_to_canvas(
    input: Image,
    region: ScreenRect,
    canvas: &mut raqote::DrawTarget,
) -> Result<(), Error> {
    let input = input.into_color_space(ColorSpace::SRGB)?;

    canvas.set_transform(&raqote::Transform::identity());
    canvas.make_transparent();

    let image = input.as_ref();

    canvas.copy_surface(
        image,
        raqote::IntRect::new(
            raqote::IntPoint::new(0, 0),
            raqote::IntPoint::new(image.width(), image.height())
        ),
        raqote::IntPoint::new(region.x(), region.y())
    );

    Ok(())
}

/// Calculates Gaussian blur sigmas for the current world transform.
///
/// If the last flag is set, then a box blur should be used. Or IIR otherwise.
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

    let (std_dx, std_dy) = scale_coordinates(
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
