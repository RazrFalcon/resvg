// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::rc::Rc;

use rgb::FromSlice;
use usvg::{FuzzyZero, NodeExt, TransformFromBBox};

use crate::{ConvTransform, render::{Canvas, RenderState}};

macro_rules! into_svgfilters_image {
    ($img:expr) => { svgfilters::ImageRef::new($img.data().as_rgba(), $img.width(), $img.height()) };
}

macro_rules! into_svgfilters_image_mut {
    ($img:expr) => { into_svgfilters_image_mut($img.width(), $img.height(), &mut $img.data_mut()) };
}

// We need a macro and a function to resolve lifetimes.
fn into_svgfilters_image_mut(width: u32, height: u32, data: &mut [u8])
    -> svgfilters::ImageRefMut
{
    svgfilters::ImageRefMut::new(data.as_rgba_mut(), width, height)
}


/// A helper trait to convert `usvg` types into `svgfilters` one.
trait IntoSvgFilters<T>: Sized {
    /// Converts an `usvg` type into `svgfilters` one.
    fn into_svgf(self) -> T;
}

impl IntoSvgFilters<svgfilters::RGB8> for usvg::Color {
    fn into_svgf(self) -> svgfilters::RGB8 {
        svgfilters::RGB8 { r: self.red, g: self.green, b: self.blue  }
    }
}

impl IntoSvgFilters<svgfilters::LightSource> for usvg::filter::LightSource {
    fn into_svgf(self) -> svgfilters::LightSource {
        match self {
            usvg::filter::LightSource::DistantLight(ref light) => {
                svgfilters::LightSource::DistantLight {
                    azimuth: light.azimuth,
                    elevation: light.elevation,
                }
            }
            usvg::filter::LightSource::PointLight(ref light) => {
                svgfilters::LightSource::PointLight {
                    x: light.x,
                    y: light.y,
                    z: light.z,
                }
            }
            usvg::filter::LightSource::SpotLight(ref light) => {
                svgfilters::LightSource::SpotLight {
                    x: light.x,
                    y: light.y,
                    z: light.z,
                    points_at_x: light.points_at_x,
                    points_at_y: light.points_at_y,
                    points_at_z: light.points_at_z,
                    specular_exponent: light.specular_exponent.value(),
                    limiting_cone_angle: light.limiting_cone_angle,
                }
            }
        }
    }
}

impl<'a> IntoSvgFilters<svgfilters::TransferFunction<'a>> for &'a usvg::filter::TransferFunction {
    fn into_svgf(self) -> svgfilters::TransferFunction<'a> {
        match *self {
            usvg::filter::TransferFunction::Identity =>
                svgfilters::TransferFunction::Identity,
            usvg::filter::TransferFunction::Table(ref data) =>
                svgfilters::TransferFunction::Table(data),
            usvg::filter::TransferFunction::Discrete(ref data) =>
                svgfilters::TransferFunction::Discrete(data),
            usvg::filter::TransferFunction::Linear { slope, intercept } =>
                svgfilters::TransferFunction::Linear { slope, intercept },
            usvg::filter::TransferFunction::Gamma { amplitude, exponent, offset } =>
                svgfilters::TransferFunction::Gamma { amplitude, exponent, offset },
        }
    }
}

impl IntoSvgFilters<svgfilters::ColorChannel> for usvg::filter::ColorChannel {
    fn into_svgf(self) -> svgfilters::ColorChannel {
        match self {
            usvg::filter::ColorChannel::R => svgfilters::ColorChannel::R,
            usvg::filter::ColorChannel::G => svgfilters::ColorChannel::G,
            usvg::filter::ColorChannel::B => svgfilters::ColorChannel::B,
            usvg::filter::ColorChannel::A => svgfilters::ColorChannel::A,
        }
    }
}


pub(crate) enum Error {
    InvalidRegion,
    NoResults,
}


trait PixmapExt: Sized {
    fn try_create(width: u32, height: u32) -> Result<tiny_skia::Pixmap, Error>;
    fn copy_region(&self, region: usvg::ScreenRect) -> Result<tiny_skia::Pixmap, Error>;
    fn clear(&mut self);
    fn into_srgb(&mut self);
    fn into_linear_rgb(&mut self);
}

impl PixmapExt for tiny_skia::Pixmap {
    fn try_create(width: u32, height: u32) -> Result<tiny_skia::Pixmap, Error> {
        tiny_skia::Pixmap::new(width, height).ok_or(Error::InvalidRegion)
    }

    fn copy_region(&self, region: usvg::ScreenRect) -> Result<tiny_skia::Pixmap, Error> {
        let rect = tiny_skia::IntRect::from_xywh(
            region.x(), region.y(), region.width(), region.height()
        ).ok_or(Error::InvalidRegion)?;
        self.clone_rect(rect).ok_or(Error::InvalidRegion)
    }

    fn clear(&mut self) {
        self.fill(tiny_skia::Color::TRANSPARENT);
    }

    fn into_srgb(&mut self) {
        svgfilters::demultiply_alpha(self.data_mut().as_rgba_mut());
        svgfilters::from_linear_rgb(self.data_mut().as_rgba_mut());
        svgfilters::multiply_alpha(self.data_mut().as_rgba_mut());
    }

    fn into_linear_rgb(&mut self) {
        svgfilters::demultiply_alpha(self.data_mut().as_rgba_mut());
        svgfilters::into_linear_rgb(self.data_mut().as_rgba_mut());
        svgfilters::multiply_alpha(self.data_mut().as_rgba_mut());
    }
}


#[derive(Clone)]
struct Image {
    /// Filter primitive result.
    ///
    /// All images have the same size which is equal to the current filter region.
    image: Rc<tiny_skia::Pixmap>,

    /// Image's region that has actual data.
    ///
    /// Region is in global coordinates and not in `image` one.
    ///
    /// Image's content outside this region will be transparent/cleared.
    ///
    /// Currently used only for `feTile`.
    region: usvg::ScreenRect,

    /// The current color space.
    color_space: usvg::filter::ColorInterpolation,
}

impl Image {
    fn from_image(image: tiny_skia::Pixmap, color_space: usvg::filter::ColorInterpolation) -> Self {
        let (w, h) = (image.width(), image.height());
        Image {
            image: Rc::new(image),
            region: usvg::ScreenRect::new(0, 0, w, h).unwrap(),
            color_space,
        }
    }

    fn into_color_space(self, color_space: usvg::filter::ColorInterpolation) -> Result<Self, Error> {
        if color_space != self.color_space {
            let region = self.region;

            let mut image = self.take()?;

            match color_space {
                usvg::filter::ColorInterpolation::SRGB => image.into_srgb(),
                usvg::filter::ColorInterpolation::LinearRGB => image.into_linear_rgb(),
            }

            Ok(Image {
                image: Rc::new(image),
                region,
                color_space,
            })
        } else {
            Ok(self)
        }
    }

    fn take(self) -> Result<tiny_skia::Pixmap, Error> {
        match Rc::try_unwrap(self.image) {
            Ok(v) => Ok(v),
            Err(v) => Ok((*v).clone()),
        }
    }

    fn width(&self) -> u32 {
        self.image.width()
    }

    fn height(&self) -> u32 {
        self.image.height()
    }

    fn as_ref(&self) -> &tiny_skia::Pixmap {
        &self.image
    }
}


struct FilterInputs<'a> {
    source: &'a mut tiny_skia::Pixmap,
    background: Option<&'a tiny_skia::Pixmap>,
    fill_paint: Option<&'a tiny_skia::Pixmap>,
    stroke_paint: Option<&'a tiny_skia::Pixmap>,
}


struct FilterResult {
    name: String,
    image: Image,
}


pub fn apply(
    filter: &usvg::filter::Filter,
    bbox: Option<usvg::Rect>,
    ts: &usvg::Transform,
    tree: &usvg::Tree,
    background: Option<&tiny_skia::Pixmap>,
    fill_paint: Option<&tiny_skia::Pixmap>,
    stroke_paint: Option<&tiny_skia::Pixmap>,
    source: &mut tiny_skia::Pixmap,
) {
    let res = {
        let inputs = FilterInputs {
            source,
            background,
            fill_paint,
            stroke_paint,
        };

        _apply(filter, &inputs, bbox, ts, tree)
    };

    let res = res.and_then(|(image, region)| apply_to_canvas(image, region, source));

    // Clear on error.
    if res.is_err() {
        source.fill(tiny_skia::Color::TRANSPARENT);
    }

    match res {
        Ok(_) => {}
        Err(Error::InvalidRegion) => {
            log::warn!("Filter '{}' has an invalid region.", filter.id);
        }
        Err(Error::NoResults) => {}
    }
}

fn _apply(
    filter: &usvg::filter::Filter,
    inputs: &FilterInputs,
    bbox: Option<usvg::Rect>,
    ts: &usvg::Transform,
    tree: &usvg::Tree,
) -> Result<(Image, usvg::ScreenRect), Error> {
    let mut results = Vec::new();
    let region = calc_region(filter, bbox, ts, inputs.source)?;

    for primitive in &filter.primitives {
        let cs = primitive.color_interpolation;
        let subregion = calc_subregion(filter, primitive, bbox, region, ts, &results)?;

        let mut result = match primitive.kind {
            usvg::filter::Kind::Blend(ref fe) => {
                let input1 = get_input(&fe.input1, region, inputs, &results)?;
                let input2 = get_input(&fe.input2, region, inputs, &results)?;
                apply_blend(fe, cs, region, input1, input2)
            }
            usvg::filter::Kind::DropShadow(ref fe) => {
                let input = get_input(&fe.input, region, inputs, &results)?;
                apply_drop_shadow(fe, filter.primitive_units, cs, bbox, ts, input)
            }
            usvg::filter::Kind::Flood(ref fe) => {
                apply_flood(fe, region)
            }
            usvg::filter::Kind::GaussianBlur(ref fe) => {
                let input = get_input(&fe.input, region, inputs, &results)?;
                apply_blur(fe, filter.primitive_units, cs, bbox, ts, input)
            }
            usvg::filter::Kind::Offset(ref fe) => {
                let input = get_input(&fe.input, region, inputs, &results)?;
                apply_offset(fe, filter.primitive_units, bbox, ts, input)
            }
            usvg::filter::Kind::Composite(ref fe) => {
                let input1 = get_input(&fe.input1, region, inputs, &results)?;
                let input2 = get_input(&fe.input2, region, inputs, &results)?;
                apply_composite(fe, cs, region, input1, input2)
            }
            usvg::filter::Kind::Merge(ref fe) => {
                apply_merge(fe, cs, region, inputs, &results)
            }
            usvg::filter::Kind::Tile(ref fe) => {
                let input = get_input(&fe.input, region, inputs, &results)?;
                apply_tile(input, region)
            }
            usvg::filter::Kind::Image(ref fe) => {
                apply_image(fe, region, subregion, tree, ts)
            }
            usvg::filter::Kind::ComponentTransfer(ref fe) => {
                let input = get_input(&fe.input, region, inputs, &results)?;
                apply_component_transfer(fe, cs, input)
            }
            usvg::filter::Kind::ColorMatrix(ref fe) => {
                let input = get_input(&fe.input, region, inputs, &results)?;
                apply_color_matrix(fe, cs, input)
            }
            usvg::filter::Kind::ConvolveMatrix(ref fe) => {
                let input = get_input(&fe.input, region, inputs, &results)?;
                apply_convolve_matrix(fe, cs, input)
            }
            usvg::filter::Kind::Morphology(ref fe) => {
                let input = get_input(&fe.input, region, inputs, &results)?;
                apply_morphology(fe, filter.primitive_units, cs, bbox, ts, input)
            }
            usvg::filter::Kind::DisplacementMap(ref fe) => {
                let input1 = get_input(&fe.input1, region, inputs, &results)?;
                let input2 = get_input(&fe.input2, region, inputs, &results)?;
                apply_displacement_map(fe, region, filter.primitive_units, cs, bbox, ts, input1, input2)
            }
            usvg::filter::Kind::Turbulence(ref fe) => {
                apply_turbulence(fe, region, cs, ts)
            }
            usvg::filter::Kind::DiffuseLighting(ref fe) => {
                let input = get_input(&fe.input, region, inputs, &results)?;
                apply_diffuse_lighting(fe, region, cs, ts, input)
            }
            usvg::filter::Kind::SpecularLighting(ref fe) => {
                let input = get_input(&fe.input, region, inputs, &results)?;
                apply_specular_lighting(fe, region, cs, ts, input)
            }
        }?;

        if region != subregion {
            // Clip result.

            // TODO: explain
            let subregion2 = if let usvg::filter::Kind::Offset(..) = primitive.kind {
                // We do not support clipping on feOffset.
                region.translate_to(0, 0)
            } else {
                subregion.translate(-region.x(), -region.y())
            };

            let color_space = result.color_space;

            let pixmap = {
                // This is cropping by clearing the pixels outside the region.
                let mut paint = tiny_skia::Paint::default();
                paint.set_color(tiny_skia::Color::BLACK);
                paint.blend_mode = tiny_skia::BlendMode::Clear;

                let mut pixmap = result.take()?;
                let w = pixmap.width() as f32;
                let h = pixmap.height() as f32;

                if let Some(rect) = tiny_skia::Rect::from_xywh(0.0, 0.0, w, subregion2.y() as f32) {
                    pixmap.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
                }

                if let Some(rect) = tiny_skia::Rect::from_xywh(0.0, 0.0, subregion2.x() as f32, h) {
                    pixmap.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
                }

                if let Some(rect) = tiny_skia::Rect::from_xywh(subregion2.right() as f32, 0.0, w, h) {
                    pixmap.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
                }

                if let Some(rect) = tiny_skia::Rect::from_xywh(0.0, subregion2.bottom() as f32, w, h) {
                    pixmap.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
                }

                pixmap
            };

            result = Image {
                image: Rc::new(pixmap),
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
    filter: &usvg::filter::Filter,
    bbox: Option<usvg::Rect>,
    ts: &usvg::Transform,
    pixmap: &tiny_skia::Pixmap,
) -> Result<usvg::ScreenRect, Error> {
    let path = usvg::PathData::from_rect(filter.rect);

    let region_ts = if filter.units == usvg::Units::ObjectBoundingBox {
        let bbox = bbox.ok_or(Error::InvalidRegion)?;
        let bbox_ts = usvg::Transform::from_bbox(bbox);
        let mut ts2 = *ts;
        ts2.append(&bbox_ts);
        ts2
    } else {
        *ts
    };

    let canvas_rect = usvg::ScreenRect::new(0, 0, pixmap.width(), pixmap.height()).unwrap();
    let region = path.bbox_with_transform(region_ts, None).ok_or(Error::InvalidRegion)?
        .to_rect().ok_or(Error::InvalidRegion)?
        .to_screen_rect()
        .fit_to_rect(canvas_rect);

    Ok(region)
}

/// Returns filter primitive region.
fn calc_subregion(
    filter: &usvg::filter::Filter,
    primitive: &usvg::filter::Primitive,
    bbox: Option<usvg::Rect>,
    filter_region: usvg::ScreenRect,
    ts: &usvg::Transform,
    results: &[FilterResult],
) -> Result<usvg::ScreenRect, Error> {
    // TODO: rewrite/simplify/explain/whatever

    let region = match primitive.kind {
        usvg::filter::Kind::Offset(ref fe) => {
            // `feOffset` inherits it's region from the input.
            match fe.input {
                usvg::filter::Input::Reference(ref name) => {
                    match results.iter().rev().find(|v| v.name == *name) {
                        Some(res) => res.image.region,
                        None => filter_region,
                    }
                }
                _ => {
                    filter_region
                }
            }
        }
        usvg::filter::Kind::Flood(..) |
        usvg::filter::Kind::Image(..) => {
            // `feImage` uses the object bbox.
            if filter.primitive_units == usvg::Units::ObjectBoundingBox {
                let bbox = bbox.ok_or(Error::InvalidRegion)?;

                // TODO: wrong
                let ts_bbox = usvg::Rect::new(ts.e, ts.f, ts.a, ts.d).unwrap();

                let r = usvg::Rect::new(
                    primitive.x.unwrap_or(0.0),
                    primitive.y.unwrap_or(0.0),
                    primitive.width.unwrap_or(1.0),
                    primitive.height.unwrap_or(1.0),
                ).ok_or(Error::InvalidRegion)?;

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
        let subregion_bbox = usvg::Rect::new(
            primitive.x.unwrap_or(0.0),
            primitive.y.unwrap_or(0.0),
            primitive.width.unwrap_or(1.0),
            primitive.height.unwrap_or(1.0),
        ).ok_or(Error::InvalidRegion)?;

        region.to_rect().bbox_transform(subregion_bbox)
    } else {
        let (dx, dy) = ts.get_translate();
        let (sx, sy) = ts.get_scale();
        usvg::Rect::new(
            primitive.x.map(|n| n * sx + dx).unwrap_or(region.x() as f64),
            primitive.y.map(|n| n * sy + dy).unwrap_or(region.y() as f64),
            primitive.width.map(|n| n * sx).unwrap_or(region.width() as f64),
            primitive.height.map(|n| n * sy).unwrap_or(region.height() as f64),
        ).ok_or(Error::InvalidRegion)?
    };

    Ok(subregion.to_screen_rect())
}

fn get_input(
    input: &usvg::filter::Input,
    region: usvg::ScreenRect,
    inputs: &FilterInputs,
    results: &[FilterResult],
) -> Result<Image, Error> {
    let convert = |in_image: Option<&tiny_skia::Pixmap>, region| {
        let image = if let Some(image) = in_image {
            image.copy_region(region)?
        } else {
            tiny_skia::Pixmap::try_create(region.width(), region.height())?
        };

        Ok(Image {
            image: Rc::new(image),
            region: region.translate_to(0, 0),
            color_space: usvg::filter::ColorInterpolation::SRGB,
        })
    };

    let convert_alpha = |mut image: tiny_skia::Pixmap| {
        // Set RGB to black. Keep alpha as is.
        for p in image.data_mut().chunks_mut(4) {
            p[0] = 0;
            p[1] = 0;
            p[2] = 0;
        }

        Ok(Image {
            image: Rc::new(image),
            region: region.translate_to(0, 0),
            color_space: usvg::filter::ColorInterpolation::SRGB,
        })
    };

    match input {
        usvg::filter::Input::SourceGraphic => {
            let image = inputs.source.copy_region(region)?;

            Ok(Image {
                image: Rc::new(image),
                region: region.translate_to(0, 0),
                color_space: usvg::filter::ColorInterpolation::SRGB,
            })
        }
        usvg::filter::Input::SourceAlpha => {
            let image = inputs.source.copy_region(region)?;
            convert_alpha(image)
        }
        usvg::filter::Input::BackgroundImage => {
            convert(inputs.background, region)
        }
        usvg::filter::Input::BackgroundAlpha => {
            let image = get_input(
                &usvg::filter::Input::BackgroundImage, region, inputs, results,
            )?;
            convert_alpha(image.take()?)
        }
        usvg::filter::Input::FillPaint => {
            convert(inputs.fill_paint, region.translate_to(0, 0))
        }
        usvg::filter::Input::StrokePaint => {
            convert(inputs.stroke_paint, region.translate_to(0, 0))
        }
        usvg::filter::Input::Reference(ref name) => {
            if let Some(v) = results.iter().rev().find(|v| v.name == *name) {
                Ok(v.image.clone())
            } else {
                // Technically unreachable.
                log::warn!("Unknown filter primitive reference '{}'.", name);
                get_input(
                    &usvg::filter::Input::SourceGraphic, region, inputs, results,
                )
            }
        }
    }
}

fn apply_drop_shadow(
    fe: &usvg::filter::DropShadow,
    units: usvg::Units,
    cs: usvg::filter::ColorInterpolation,
    bbox: Option<usvg::Rect>,
    ts: &usvg::Transform,
    input: Image,
) -> Result<Image, Error> {
    let (dx, dy) = match scale_coordinates(fe.dx, fe.dy, units, bbox, ts) {
        Some(v) => v,
        None => return Ok(input),
    };

    let mut pixmap = tiny_skia::Pixmap::try_create(input.width(), input.height())?;
    let input_pixmap = input.into_color_space(cs)?.take()?;
    let mut shadow_pixmap = input_pixmap.clone();

    if let Some((std_dx, std_dy, box_blur)) = resolve_std_dev(fe.std_dev_x, fe.std_dev_y, units, bbox, ts) {
        if box_blur {
            svgfilters::box_blur(std_dx, std_dy, into_svgfilters_image_mut!(shadow_pixmap));
        } else {
            svgfilters::iir_blur(std_dx, std_dy, into_svgfilters_image_mut!(shadow_pixmap));
        }
    }

    // flood
    let alpha = crate::paint_server::multiply_a8(fe.opacity.to_u8(), fe.color.alpha);
    let color = tiny_skia::Color::from_rgba8(fe.color.red, fe.color.green, fe.color.blue, alpha);
    for p in shadow_pixmap.pixels_mut() {
        let mut color = color.clone();
        color.apply_opacity(p.alpha() as f32 / 255.0);
        *p = color.premultiply().to_color_u8();
    }

    match cs {
        usvg::filter::ColorInterpolation::SRGB => shadow_pixmap.into_srgb(),
        usvg::filter::ColorInterpolation::LinearRGB => shadow_pixmap.into_linear_rgb(),
    }

    pixmap.draw_pixmap(
        dx as i32,
        dy as i32,
        shadow_pixmap.as_ref(),
        &tiny_skia::PixmapPaint::default(),
        tiny_skia::Transform::identity(),
        None,
    );

    pixmap.draw_pixmap(
        0,
        0,
        input_pixmap.as_ref(),
        &tiny_skia::PixmapPaint::default(),
        tiny_skia::Transform::identity(),
        None,
    );

    Ok(Image::from_image(pixmap, cs))
}

fn apply_blur(
    fe: &usvg::filter::GaussianBlur,
    units: usvg::Units,
    cs: usvg::filter::ColorInterpolation,
    bbox: Option<usvg::Rect>,
    ts: &usvg::Transform,
    input: Image,
) -> Result<Image, Error> {
    let (std_dx, std_dy, box_blur) = match resolve_std_dev(fe.std_dev_x, fe.std_dev_y, units, bbox, ts) {
        Some(v) => v,
        None => return Ok(input),
    };

    let mut pixmap = input.into_color_space(cs)?.take()?;

    if box_blur {
        svgfilters::box_blur(std_dx, std_dy, into_svgfilters_image_mut!(pixmap));
    } else {
        svgfilters::iir_blur(std_dx, std_dy, into_svgfilters_image_mut!(pixmap));
    }

    Ok(Image::from_image(pixmap, cs))
}

fn apply_offset(
    fe: &usvg::filter::Offset,
    units: usvg::Units,
    bbox: Option<usvg::Rect>,
    ts: &usvg::Transform,
    input: Image,
) -> Result<Image, Error> {
    let (dx, dy) = match scale_coordinates(fe.dx, fe.dy, units, bbox, ts) {
        Some(v) => v,
        None => return Ok(input),
    };

    if dx.is_fuzzy_zero() && dy.is_fuzzy_zero() {
        return Ok(input);
    }

    let mut pixmap = tiny_skia::Pixmap::try_create(input.width(), input.height())?;
    pixmap.draw_pixmap(
        dx as i32,
        dy as i32,
        input.as_ref().as_ref(),
        &tiny_skia::PixmapPaint::default(),
        tiny_skia::Transform::identity(),
        None,
    );

    Ok(Image::from_image(pixmap, input.color_space))
}

fn apply_blend(
    fe: &usvg::filter::Blend,
    cs: usvg::filter::ColorInterpolation,
    region: usvg::ScreenRect,
    input1: Image,
    input2: Image,
) -> Result<Image, Error> {
    let input1 = input1.into_color_space(cs)?;
    let input2 = input2.into_color_space(cs)?;

    let mut pixmap = tiny_skia::Pixmap::try_create(region.width(), region.height())?;

    pixmap.draw_pixmap(
        0,
        0,
        input2.as_ref().as_ref(),
        &tiny_skia::PixmapPaint::default(),
        tiny_skia::Transform::identity(),
        None,
    );

    let blend_mode = match fe.mode {
        usvg::filter::BlendMode::Normal => tiny_skia::BlendMode::SourceOver,
        usvg::filter::BlendMode::Multiply => tiny_skia::BlendMode::Multiply,
        usvg::filter::BlendMode::Screen => tiny_skia::BlendMode::Screen,
        usvg::filter::BlendMode::Overlay => tiny_skia::BlendMode::Overlay,
        usvg::filter::BlendMode::Darken => tiny_skia::BlendMode::Darken,
        usvg::filter::BlendMode::Lighten => tiny_skia::BlendMode::Lighten,
        usvg::filter::BlendMode::ColorDodge => tiny_skia::BlendMode::ColorDodge,
        usvg::filter::BlendMode::ColorBurn => tiny_skia::BlendMode::ColorBurn,
        usvg::filter::BlendMode::HardLight => tiny_skia::BlendMode::HardLight,
        usvg::filter::BlendMode::SoftLight => tiny_skia::BlendMode::SoftLight,
        usvg::filter::BlendMode::Difference => tiny_skia::BlendMode::Difference,
        usvg::filter::BlendMode::Exclusion => tiny_skia::BlendMode::Exclusion,
        usvg::filter::BlendMode::Hue => tiny_skia::BlendMode::Hue,
        usvg::filter::BlendMode::Saturation => tiny_skia::BlendMode::Saturation,
        usvg::filter::BlendMode::Color => tiny_skia::BlendMode::Color,
        usvg::filter::BlendMode::Luminosity => tiny_skia::BlendMode::Luminosity,
    };

    pixmap.draw_pixmap(
        0,
        0,
        input1.as_ref().as_ref(),
        &tiny_skia::PixmapPaint {
            blend_mode,
            ..tiny_skia::PixmapPaint::default()
        },
        tiny_skia::Transform::identity(),
        None,
    );

    Ok(Image::from_image(pixmap, cs))
}

fn apply_composite(
    fe: &usvg::filter::Composite,
    cs: usvg::filter::ColorInterpolation,
    region: usvg::ScreenRect,
    input1: Image,
    input2: Image,
) -> Result<Image, Error> {
    use usvg::filter::CompositeOperator as Operator;

    let input1 = input1.into_color_space(cs)?;
    let input2 = input2.into_color_space(cs)?;

    let mut pixmap = tiny_skia::Pixmap::try_create(region.width(), region.height())?;

    if let Operator::Arithmetic { k1, k2, k3, k4 } = fe.operator {
        let pixmap1 = input1.take()?;
        let pixmap2 = input2.take()?;

        svgfilters::arithmetic_composite(
            k1, k2, k3, k4,
            into_svgfilters_image!(pixmap1),
            into_svgfilters_image!(pixmap2),
            into_svgfilters_image_mut!(pixmap),
        );

        return Ok(Image::from_image(pixmap, cs));
    }

    pixmap.draw_pixmap(
        0,
        0,
        input2.as_ref().as_ref(),
        &tiny_skia::PixmapPaint::default(),
        tiny_skia::Transform::identity(),
        None,
    );

    let blend_mode = match fe.operator {
        Operator::Over => tiny_skia::BlendMode::SourceOver,
        Operator::In => tiny_skia::BlendMode::SourceIn,
        Operator::Out => tiny_skia::BlendMode::SourceOut,
        Operator::Atop => tiny_skia::BlendMode::SourceAtop,
        Operator::Xor => tiny_skia::BlendMode::Xor,
        Operator::Arithmetic { .. } => tiny_skia::BlendMode::SourceOver,
    };

    pixmap.draw_pixmap(
        0,
        0,
        input1.as_ref().as_ref(),
        &tiny_skia::PixmapPaint {
            blend_mode,
            ..tiny_skia::PixmapPaint::default()
        },
        tiny_skia::Transform::identity(),
        None,
    );

    Ok(Image::from_image(pixmap, cs))
}

fn apply_merge(
    fe: &usvg::filter::Merge,
    cs: usvg::filter::ColorInterpolation,
    region: usvg::ScreenRect,
    inputs: &FilterInputs,
    results: &[FilterResult],
) -> Result<Image, Error> {
    let mut pixmap = tiny_skia::Pixmap::try_create(region.width(), region.height())?;

    for input in &fe.inputs {
        let input = get_input(input, region, inputs, results)?;
        let input = input.into_color_space(cs)?;
        pixmap.draw_pixmap(
            0,
            0,
            input.as_ref().as_ref(),
            &tiny_skia::PixmapPaint::default(),
            tiny_skia::Transform::identity(),
            None,
        );
    }

    Ok(Image::from_image(pixmap, cs))
}

fn apply_flood(
    fe: &usvg::filter::Flood,
    region: usvg::ScreenRect,
) -> Result<Image, Error> {
    let c = fe.color;

    let mut pixmap = tiny_skia::Pixmap::try_create(region.width(), region.height())?;
    let alpha = crate::paint_server::multiply_a8(fe.opacity.to_u8(), c.alpha);
    pixmap.fill(tiny_skia::Color::from_rgba8(c.red, c.green, c.blue, alpha));

    Ok(Image::from_image(pixmap, usvg::filter::ColorInterpolation::SRGB))
}

fn apply_tile(
    input: Image,
    region: usvg::ScreenRect,
) -> Result<Image, Error> {
    let subregion = input.region.translate(-region.x(), -region.y());

    let tile_pixmap = input.image.copy_region(subregion)?;
    let mut paint = tiny_skia::Paint::default();
    paint.shader = tiny_skia::Pattern::new(
        tile_pixmap.as_ref(),
        tiny_skia::SpreadMode::Repeat,
        tiny_skia::FilterQuality::Bicubic,
        1.0,
        tiny_skia::Transform::from_translate(subregion.x() as f32, subregion.y() as f32),
    );

    let mut pixmap = tiny_skia::Pixmap::try_create(region.width(), region.height())?;
    let rect = tiny_skia::Rect::from_xywh(0.0, 0.0, region.width() as f32, region.height() as f32).unwrap();
    pixmap.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);

    Ok(Image::from_image(pixmap, usvg::filter::ColorInterpolation::SRGB))
}

fn apply_image(
    fe: &usvg::filter::Image,
    region: usvg::ScreenRect,
    subregion: usvg::ScreenRect,
    tree: &usvg::Tree,
    ts: &usvg::Transform,
) -> Result<Image, Error> {
    let mut pixmap = tiny_skia::Pixmap::try_create(region.width(), region.height())?;
    let mut canvas = Canvas::from(pixmap.as_mut());

    match fe.data {
        usvg::filter::ImageKind::Image(ref kind) => {
            let dx = (subregion.x() - region.x()) as f32;
            let dy = (subregion.y() - region.y()) as f32;
            canvas.translate(dx, dy);

            let view_box = usvg::ViewBox {
                rect: subregion.translate_to(0, 0).to_rect(),
                aspect: fe.aspect,
            };

            crate::image::draw_kind(kind, view_box, fe.rendering_mode, &mut canvas);
        }
        usvg::filter::ImageKind::Use(ref id) => {
            if let Some(ref node) = tree.defs_by_id(id).or_else(|| tree.node_by_id(id)) {
                let (sx, sy) = ts.get_scale();
                canvas.scale(sx as f32, sy as f32);
                canvas.apply_transform(node.transform().to_native());

                crate::render::render_node(tree, node, &mut RenderState::Ok, &mut canvas);
            }
        }
    }

    Ok(Image::from_image(pixmap, usvg::filter::ColorInterpolation::SRGB))
}

fn apply_component_transfer(
    fe: &usvg::filter::ComponentTransfer,
    cs: usvg::filter::ColorInterpolation,
    input: Image,
) -> Result<Image, Error> {
    let mut pixmap = input.into_color_space(cs)?.take()?;

    svgfilters::demultiply_alpha(pixmap.data_mut().as_rgba_mut());

    svgfilters::component_transfer(
        fe.func_b.into_svgf(),
        fe.func_g.into_svgf(),
        fe.func_r.into_svgf(),
        fe.func_a.into_svgf(),
        into_svgfilters_image_mut!(pixmap),
    );

    svgfilters::multiply_alpha(pixmap.data_mut().as_rgba_mut());

    Ok(Image::from_image(pixmap, cs))
}

fn apply_color_matrix(
    fe: &usvg::filter::ColorMatrix,
    cs: usvg::filter::ColorInterpolation,
    input: Image,
) -> Result<Image, Error> {
    use std::convert::TryInto;

    let mut pixmap = input.into_color_space(cs)?.take()?;

    svgfilters::demultiply_alpha(pixmap.data_mut().as_rgba_mut());

    let kind = match fe.kind {
        usvg::filter::ColorMatrixKind::Matrix(ref data) =>
            svgfilters::ColorMatrix::Matrix(data.as_slice().try_into().unwrap()),
        usvg::filter::ColorMatrixKind::Saturate(n) =>
            svgfilters::ColorMatrix::Saturate(n.value()),
        usvg::filter::ColorMatrixKind::HueRotate(n) =>
            svgfilters::ColorMatrix::HueRotate(n),
        usvg::filter::ColorMatrixKind::LuminanceToAlpha =>
            svgfilters::ColorMatrix::LuminanceToAlpha,
    };

    svgfilters::color_matrix(kind, into_svgfilters_image_mut!(pixmap));

    svgfilters::multiply_alpha(pixmap.data_mut().as_rgba_mut());

    Ok(Image::from_image(pixmap, cs))
}

fn apply_convolve_matrix(
    fe: &usvg::filter::ConvolveMatrix,
    cs: usvg::filter::ColorInterpolation,
    input: Image,
) -> Result<Image, Error> {
    let mut pixmap = input.into_color_space(cs)?.take()?;

    if fe.preserve_alpha {
        svgfilters::demultiply_alpha(pixmap.data_mut().as_rgba_mut());
    }

    let matrix = svgfilters::ConvolveMatrix::new(
        fe.matrix.target_x(), fe.matrix.target_y(),
        fe.matrix.columns(), fe.matrix.rows(),
        fe.matrix.data(),
    ).unwrap();

    let edge_mode = match fe.edge_mode {
        usvg::filter::EdgeMode::None => svgfilters::EdgeMode::None,
        usvg::filter::EdgeMode::Duplicate => svgfilters::EdgeMode::Duplicate,
        usvg::filter::EdgeMode::Wrap => svgfilters::EdgeMode::Wrap,
    };

    svgfilters::convolve_matrix(
        matrix, fe.divisor.value(), fe.bias, edge_mode, fe.preserve_alpha,
        into_svgfilters_image_mut!(pixmap),
    );

    Ok(Image::from_image(pixmap, cs))
}

fn apply_morphology(
    fe: &usvg::filter::Morphology,
    units: usvg::Units,
    cs: usvg::filter::ColorInterpolation,
    bbox: Option<usvg::Rect>,
    ts: &usvg::Transform,
    input: Image,
) -> Result<Image, Error> {
    let mut pixmap = input.into_color_space(cs)?.take()?;
    let (rx, ry) = match scale_coordinates(fe.radius_x.value(), fe.radius_y.value(), units, bbox, ts) {
        Some(v) => v,
        None => return Ok(Image::from_image(pixmap, cs)),
    };

    if !(rx > 0.0 && ry > 0.0) {
        pixmap.clear();
        return Ok(Image::from_image(pixmap, cs));
    }

    let operator = match fe.operator {
        usvg::filter::MorphologyOperator::Erode => svgfilters::MorphologyOperator::Erode,
        usvg::filter::MorphologyOperator::Dilate => svgfilters::MorphologyOperator::Dilate,
    };

    svgfilters::morphology(operator, rx, ry, into_svgfilters_image_mut!(pixmap));

    Ok(Image::from_image(pixmap, cs))
}

fn apply_displacement_map(
    fe: &usvg::filter::DisplacementMap,
    region: usvg::ScreenRect,
    units: usvg::Units,
    cs: usvg::filter::ColorInterpolation,
    bbox: Option<usvg::Rect>,
    ts: &usvg::Transform,
    input1: Image,
    input2: Image,
) -> Result<Image, Error> {
    let pixmap1 = input1.into_color_space(cs)?.take()?;
    let pixmap2 = input2.into_color_space(cs)?.take()?;
    let (sx, sy) = match scale_coordinates(fe.scale, fe.scale, units, bbox, ts) {
        Some(v) => v,
        None => return Ok(Image::from_image(pixmap1, cs))
    };

    let mut pixmap = tiny_skia::Pixmap::try_create(region.width(), region.height())?;

    svgfilters::displacement_map(
        fe.x_channel_selector.into_svgf(),
        fe.y_channel_selector.into_svgf(),
        sx, sy,
        into_svgfilters_image!(&pixmap1),
        into_svgfilters_image!(&pixmap2),
        into_svgfilters_image_mut!(pixmap),
    );

    Ok(Image::from_image(pixmap, cs))
}

fn apply_turbulence(
    fe: &usvg::filter::Turbulence,
    region: usvg::ScreenRect,
    cs: usvg::filter::ColorInterpolation,
    ts: &usvg::Transform,
) -> Result<Image, Error> {
    let mut pixmap = tiny_skia::Pixmap::try_create(region.width(), region.height())?;

    let (sx, sy) = ts.get_scale();
    if sx.is_fuzzy_zero() || sy.is_fuzzy_zero() {
        return Ok(Image::from_image(pixmap, cs));
    }

    svgfilters::turbulence(
        region.x() as f64, region.y() as f64,
        sx, sy,
        fe.base_frequency.x.value(), fe.base_frequency.y.value(),
        fe.num_octaves,
        fe.seed,
        fe.stitch_tiles,
        fe.kind == usvg::filter::TurbulenceKind::FractalNoise,
        into_svgfilters_image_mut!(pixmap),
    );

    svgfilters::multiply_alpha(pixmap.data_mut().as_rgba_mut());

    Ok(Image::from_image(pixmap, cs))
}

fn apply_diffuse_lighting(
    fe: &usvg::filter::DiffuseLighting,
    region: usvg::ScreenRect,
    cs: usvg::filter::ColorInterpolation,
    ts: &usvg::Transform,
    input: Image,
) -> Result<Image, Error> {
    let mut pixmap = tiny_skia::Pixmap::try_create(region.width(), region.height())?;

    let light_source = fe.light_source.transform(region, ts);

    svgfilters::diffuse_lighting(
        fe.surface_scale,
        fe.diffuse_constant,
        fe.lighting_color.into_svgf(),
        light_source.into_svgf(),
        into_svgfilters_image!(input.as_ref()),
        into_svgfilters_image_mut!(pixmap),
    );

    Ok(Image::from_image(pixmap, cs))
}

fn apply_specular_lighting(
    fe: &usvg::filter::SpecularLighting,
    region: usvg::ScreenRect,
    cs: usvg::filter::ColorInterpolation,
    ts: &usvg::Transform,
    input: Image,
) -> Result<Image, Error> {
    let mut pixmap = tiny_skia::Pixmap::try_create(region.width(), region.height())?;

    let light_source = fe.light_source.transform(region, ts);

    svgfilters::specular_lighting(
        fe.surface_scale,
        fe.specular_constant,
        fe.specular_exponent,
        fe.lighting_color.into_svgf(),
        light_source.into_svgf(),
        into_svgfilters_image!(input.as_ref()),
        into_svgfilters_image_mut!(pixmap),
    );

    Ok(Image::from_image(pixmap, cs))
}

fn apply_to_canvas(
    input: Image,
    region: usvg::ScreenRect,
    pixmap: &mut tiny_skia::Pixmap,
) -> Result<(), Error> {
    let input = input.into_color_space(usvg::filter::ColorInterpolation::SRGB)?;

    pixmap.fill(tiny_skia::Color::TRANSPARENT);
    pixmap.draw_pixmap(
        region.x() as i32,
        region.y() as i32,
        input.as_ref().as_ref(),
        &tiny_skia::PixmapPaint::default(),
        tiny_skia::Transform::identity(),
        None,
    );

    Ok(())
}

/// Calculates Gaussian blur sigmas for the current world transform.
///
/// If the last flag is set, then a box blur should be used. Or IIR otherwise.
fn resolve_std_dev(
    std_dev_x: usvg::PositiveNumber,
    std_dev_y: usvg::PositiveNumber,
    units: usvg::Units,
    bbox: Option<usvg::Rect>,
    ts: &usvg::Transform,
) -> Option<(f64, f64, bool)> {
    // 'A negative value or a value of zero disables the effect of the given filter primitive
    // (i.e., the result is the filter input image).'
    if std_dev_x.is_zero() && std_dev_y.is_zero() {
        return None;
    }

    let (mut std_dx, mut std_dy) = scale_coordinates(
        std_dev_x.value(), std_dev_y.value(), units, bbox, ts,
    )?;
    if std_dx.is_fuzzy_zero() && std_dy.is_fuzzy_zero() {
        None
    } else {
        // Ignore tiny sigmas. In case of IIR blur it can lead to a transparent image.
        if std_dx < 0.05 {
            std_dx = 0.0;
        }

        if std_dy < 0.05 {
            std_dy = 0.0;
        }

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
    bbox: Option<usvg::Rect>,
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
