// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::rc::Rc;

use rgb::FromSlice;
use usvg::{FuzzyEq, FuzzyZero, Transform};

use crate::geom::{IntRect, UsvgRectExt};
use crate::tree::{ConvTransform, Node};

// TODO: apply single primitive filters in-place

macro_rules! into_svgfilters_image {
    ($img:expr) => {
        svgfilters::ImageRef::new($img.data().as_rgba(), $img.width(), $img.height())
    };
}

macro_rules! into_svgfilters_image_mut {
    ($img:expr) => {
        into_svgfilters_image_mut($img.width(), $img.height(), &mut $img.data_mut())
    };
}

// We need a macro and a function to resolve lifetimes.
fn into_svgfilters_image_mut(width: u32, height: u32, data: &mut [u8]) -> svgfilters::ImageRefMut {
    svgfilters::ImageRefMut::new(data.as_rgba_mut(), width, height)
}

/// A helper trait to convert `usvg` types into `svgfilters` one.
trait IntoSvgFilters<T>: Sized {
    /// Converts an `usvg` type into `svgfilters` one.
    fn into_svgf(self) -> T;
}

impl IntoSvgFilters<svgfilters::RGB8> for usvg::Color {
    fn into_svgf(self) -> svgfilters::RGB8 {
        svgfilters::RGB8 {
            r: self.red,
            g: self.green,
            b: self.blue,
        }
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
            usvg::filter::LightSource::SpotLight(ref light) => svgfilters::LightSource::SpotLight {
                x: light.x,
                y: light.y,
                z: light.z,
                points_at_x: light.points_at_x,
                points_at_y: light.points_at_y,
                points_at_z: light.points_at_z,
                specular_exponent: light.specular_exponent.get(),
                limiting_cone_angle: light.limiting_cone_angle,
            },
        }
    }
}

impl<'a> IntoSvgFilters<svgfilters::TransferFunction<'a>> for &'a usvg::filter::TransferFunction {
    fn into_svgf(self) -> svgfilters::TransferFunction<'a> {
        match *self {
            usvg::filter::TransferFunction::Identity => svgfilters::TransferFunction::Identity,
            usvg::filter::TransferFunction::Table(ref data) => {
                svgfilters::TransferFunction::Table(data)
            }
            usvg::filter::TransferFunction::Discrete(ref data) => {
                svgfilters::TransferFunction::Discrete(data)
            }
            usvg::filter::TransferFunction::Linear { slope, intercept } => {
                svgfilters::TransferFunction::Linear { slope, intercept }
            }
            usvg::filter::TransferFunction::Gamma {
                amplitude,
                exponent,
                offset,
            } => svgfilters::TransferFunction::Gamma {
                amplitude,
                exponent,
                offset,
            },
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

pub struct Primitive {
    pub region: usvg::Rect,
    pub color_interpolation: usvg::filter::ColorInterpolation,
    pub result: String,
    pub kind: usvg::filter::Kind,
}

pub struct Filter {
    pub region: usvg::Rect,
    pub primitives: Vec<Primitive>,
}

pub fn convert(
    ufilters: &[Rc<usvg::filter::Filter>],
    object_bbox: Option<usvg::PathBbox>,
) -> (Vec<Filter>, Option<usvg::PathBbox>) {
    let object_bbox = object_bbox.and_then(|bbox| bbox.to_rect());

    let region = match calc_filters_region(ufilters, object_bbox) {
        Some(v) => v,
        None => return (Vec::new(), None),
    };

    let mut filters = Vec::new();
    for ufilter in ufilters {
        let filter = match convert_filter(&ufilter, object_bbox, region) {
            Some(v) => v,
            None => return (Vec::new(), None),
        };
        filters.push(filter);
    }

    (filters, Some(region.to_path_bbox()))
}

fn convert_filter(
    ufilter: &usvg::filter::Filter,
    object_bbox: Option<usvg::Rect>,
    region: usvg::Rect,
) -> Option<Filter> {
    let mut primitives = Vec::with_capacity(ufilter.primitives.len());
    for uprimitive in &ufilter.primitives {
        let subregion = match calc_subregion(ufilter, uprimitive, object_bbox, region) {
            Some(v) => v,
            None => {
                log::warn!("Invalid filter primitive region.");
                continue;
            }
        };

        if let Some(kind) = convert_primitive(&uprimitive, ufilter.primitive_units, object_bbox) {
            primitives.push(Primitive {
                region: subregion,
                color_interpolation: uprimitive.color_interpolation,
                result: uprimitive.result.clone(),
                kind,
            });
        }
    }

    Some(Filter { region, primitives })
}

fn convert_primitive(
    uprimitive: &usvg::filter::Primitive,
    units: usvg::Units,
    object_bbox: Option<usvg::Rect>,
) -> Option<usvg::filter::Kind> {
    match uprimitive.kind {
        usvg::filter::Kind::DisplacementMap(ref fe) => {
            let (sx, _) = scale_coordinates(fe.scale, fe.scale, units, object_bbox)?;
            Some(usvg::filter::Kind::DisplacementMap(
                usvg::filter::DisplacementMap {
                    input1: fe.input1.clone(),
                    input2: fe.input2.clone(),
                    scale: sx,
                    x_channel_selector: fe.x_channel_selector,
                    y_channel_selector: fe.y_channel_selector,
                },
            ))
        }
        usvg::filter::Kind::DropShadow(ref fe) => {
            let (dx, dy) = scale_coordinates(fe.dx, fe.dy, units, object_bbox)?;
            let (std_dev_x, std_dev_y) =
                scale_coordinates(fe.std_dev_x.get(), fe.std_dev_y.get(), units, object_bbox)?;
            Some(usvg::filter::Kind::DropShadow(usvg::filter::DropShadow {
                input: fe.input.clone(),
                dx,
                dy,
                std_dev_x: usvg::PositiveF64::new(std_dev_x).unwrap_or_default(),
                std_dev_y: usvg::PositiveF64::new(std_dev_y).unwrap_or_default(),
                color: fe.color,
                opacity: fe.opacity,
            }))
        }
        usvg::filter::Kind::GaussianBlur(ref fe) => {
            let (std_dev_x, std_dev_y) =
                scale_coordinates(fe.std_dev_x.get(), fe.std_dev_y.get(), units, object_bbox)?;
            Some(usvg::filter::Kind::GaussianBlur(
                usvg::filter::GaussianBlur {
                    input: fe.input.clone(),
                    std_dev_x: usvg::PositiveF64::new(std_dev_x).unwrap_or_default(),
                    std_dev_y: usvg::PositiveF64::new(std_dev_y).unwrap_or_default(),
                },
            ))
        }
        usvg::filter::Kind::Morphology(ref fe) => {
            let (radius_x, radius_y) =
                scale_coordinates(fe.radius_x.get(), fe.radius_y.get(), units, object_bbox)?;
            Some(usvg::filter::Kind::Morphology(usvg::filter::Morphology {
                input: fe.input.clone(),
                operator: fe.operator,
                radius_x: usvg::PositiveF64::new(radius_x).unwrap_or_default(),
                radius_y: usvg::PositiveF64::new(radius_y).unwrap_or_default(),
            }))
        }
        usvg::filter::Kind::Offset(ref fe) => {
            let (dx, dy) = scale_coordinates(fe.dx, fe.dy, units, object_bbox)?;
            Some(usvg::filter::Kind::Offset(usvg::filter::Offset {
                input: fe.input.clone(),
                dx,
                dy,
            }))
        }
        _ => Some(uprimitive.kind.clone()),
    }
}

#[derive(Debug)]
pub(crate) enum Error {
    InvalidRegion,
    NoResults,
}

trait PixmapExt: Sized {
    fn try_create(width: u32, height: u32) -> Result<tiny_skia::Pixmap, Error>;
    fn copy_region(&self, region: IntRect) -> Result<tiny_skia::Pixmap, Error>;
    fn clear(&mut self);
    fn into_srgb(&mut self);
    fn into_linear_rgb(&mut self);
}

impl PixmapExt for tiny_skia::Pixmap {
    fn try_create(width: u32, height: u32) -> Result<tiny_skia::Pixmap, Error> {
        tiny_skia::Pixmap::new(width, height).ok_or(Error::InvalidRegion)
    }

    fn copy_region(&self, region: IntRect) -> Result<tiny_skia::Pixmap, Error> {
        let rect =
            tiny_skia::IntRect::from_xywh(region.x(), region.y(), region.width(), region.height())
                .ok_or(Error::InvalidRegion)?;
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
    region: IntRect,

    /// The current color space.
    color_space: usvg::filter::ColorInterpolation,
}

impl Image {
    fn from_image(image: tiny_skia::Pixmap, color_space: usvg::filter::ColorInterpolation) -> Self {
        let (w, h) = (image.width(), image.height());
        Image {
            image: Rc::new(image),
            region: IntRect::new(0, 0, w, h).unwrap(),
            color_space,
        }
    }

    fn into_color_space(
        self,
        color_space: usvg::filter::ColorInterpolation,
    ) -> Result<Self, Error> {
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
    fill_paint: Option<&'a tiny_skia::Pixmap>,
    stroke_paint: Option<&'a tiny_skia::Pixmap>,
}

struct FilterResult {
    name: String,
    image: Image,
}

pub fn apply(
    filter: &Filter,
    ts: tiny_skia::Transform,
    fill_paint: Option<&tiny_skia::Pixmap>,
    stroke_paint: Option<&tiny_skia::Pixmap>,
    source: &mut tiny_skia::Pixmap,
) {
    let inputs = FilterInputs {
        source,
        fill_paint,
        stroke_paint,
    };

    let ts = usvg::Transform::from_native(ts);

    let result = apply_inner(filter, &inputs, &ts);
    let result = result.and_then(|image| apply_to_canvas(image, source));

    // Clear on error.
    if result.is_err() {
        source.fill(tiny_skia::Color::TRANSPARENT);
    }

    match result {
        Ok(_) => {}
        Err(Error::InvalidRegion) => {
            log::warn!("Filter has an invalid region.");
        }
        Err(Error::NoResults) => {}
    }
}

fn apply_inner(
    filter: &Filter,
    inputs: &FilterInputs,
    ts: &usvg::Transform,
) -> Result<Image, Error> {
    let mut results: Vec<FilterResult> = Vec::new();

    let region = filter
        .region
        .transform(ts)
        .map(|r| r.to_int_rect_round_out())
        .ok_or(Error::InvalidRegion)?;

    for primitive in &filter.primitives {
        let cs = primitive.color_interpolation;
        let mut subregion = primitive
            .region
            .transform(ts)
            .map(|r| r.to_int_rect_round_out())
            .ok_or(Error::InvalidRegion)?;

        // `feOffset` inherits its region from the input.
        if let usvg::filter::Kind::Offset(ref fe) = primitive.kind {
            if let usvg::filter::Input::Reference(ref name) = fe.input {
                if let Some(res) = results.iter().rev().find(|v| v.name == *name) {
                    subregion = res.image.region;
                }
            }
        }

        let mut result = match primitive.kind {
            usvg::filter::Kind::Blend(ref fe) => {
                let input1 = get_input(&fe.input1, region, inputs, &results)?;
                let input2 = get_input(&fe.input2, region, inputs, &results)?;
                apply_blend(fe, cs, region, input1, input2)
            }
            usvg::filter::Kind::DropShadow(ref fe) => {
                let input = get_input(&fe.input, region, inputs, &results)?;
                apply_drop_shadow(fe, cs, ts, input)
            }
            usvg::filter::Kind::Flood(ref fe) => apply_flood(fe, region),
            usvg::filter::Kind::GaussianBlur(ref fe) => {
                let input = get_input(&fe.input, region, inputs, &results)?;
                apply_blur(fe, cs, ts, input)
            }
            usvg::filter::Kind::Offset(ref fe) => {
                let input = get_input(&fe.input, region, inputs, &results)?;
                apply_offset(fe, ts, input)
            }
            usvg::filter::Kind::Composite(ref fe) => {
                let input1 = get_input(&fe.input1, region, inputs, &results)?;
                let input2 = get_input(&fe.input2, region, inputs, &results)?;
                apply_composite(fe, cs, region, input1, input2)
            }
            usvg::filter::Kind::Merge(ref fe) => apply_merge(fe, cs, region, inputs, &results),
            usvg::filter::Kind::Tile(ref fe) => {
                let input = get_input(&fe.input, region, inputs, &results)?;
                apply_tile(input, region)
            }
            usvg::filter::Kind::Image(ref fe) => apply_image(fe, region, subregion, ts),
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
                apply_morphology(fe, cs, ts, input)
            }
            usvg::filter::Kind::DisplacementMap(ref fe) => {
                let input1 = get_input(&fe.input1, region, inputs, &results)?;
                let input2 = get_input(&fe.input2, region, inputs, &results)?;
                apply_displacement_map(fe, region, cs, ts, input1, input2)
            }
            usvg::filter::Kind::Turbulence(ref fe) => apply_turbulence(fe, region, cs, ts),
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

                if let Some(rect) = tiny_skia::Rect::from_xywh(subregion2.right() as f32, 0.0, w, h)
                {
                    pixmap.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
                }

                if let Some(rect) =
                    tiny_skia::Rect::from_xywh(0.0, subregion2.bottom() as f32, w, h)
                {
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
        Ok(res.image)
    } else {
        Err(Error::NoResults)
    }
}

// TODO: merge with mask region logic
fn calc_region(
    filter: &usvg::filter::Filter,
    object_bbox: Option<usvg::Rect>,
) -> Option<usvg::Rect> {
    if filter.units == usvg::Units::ObjectBoundingBox {
        Some(filter.rect.bbox_transform(object_bbox?))
    } else {
        Some(filter.rect)
    }
}

pub fn calc_filters_region(
    filters: &[Rc<usvg::filter::Filter>],
    object_bbox: Option<usvg::Rect>,
) -> Option<usvg::Rect> {
    let mut global_region = usvg::Rect::new_bbox();

    for filter in filters {
        if let Some(region) = calc_region(filter, object_bbox) {
            global_region = global_region.expand(region);
        }
    }

    if global_region.fuzzy_ne(&usvg::Rect::new_bbox()) {
        Some(global_region)
    } else {
        None
    }
}

fn calc_subregion(
    filter: &usvg::filter::Filter,
    primitive: &usvg::filter::Primitive,
    bbox: Option<usvg::Rect>,
    region: usvg::Rect,
) -> Option<usvg::Rect> {
    // TODO: rewrite/simplify/explain/whatever

    let region = match primitive.kind {
        usvg::filter::Kind::Flood(..) | usvg::filter::Kind::Image(..) => {
            // `feImage` uses the object bbox.
            if filter.primitive_units == usvg::Units::ObjectBoundingBox {
                let bbox = bbox?;

                // TODO: wrong
                // let ts_bbox = usvg::Rect::new(ts.e, ts.f, ts.a, ts.d).unwrap();

                let r = usvg::Rect::new(
                    primitive.x.unwrap_or(0.0),
                    primitive.y.unwrap_or(0.0),
                    primitive.width.unwrap_or(1.0),
                    primitive.height.unwrap_or(1.0),
                )?;

                let r = r.bbox_transform(bbox);
                // .bbox_transform(ts_bbox);

                return Some(r);
            } else {
                region
            }
        }
        _ => region,
    };

    // TODO: Wrong! Does not account rotate and skew.
    let subregion = if filter.primitive_units == usvg::Units::ObjectBoundingBox {
        let subregion_bbox = usvg::Rect::new(
            primitive.x.unwrap_or(0.0),
            primitive.y.unwrap_or(0.0),
            primitive.width.unwrap_or(1.0),
            primitive.height.unwrap_or(1.0),
        )?;

        region.bbox_transform(subregion_bbox)
    } else {
        usvg::Rect::new(
            primitive.x.unwrap_or(region.x() as f64),
            primitive.y.unwrap_or(region.y() as f64),
            primitive.width.unwrap_or(region.width() as f64),
            primitive.height.unwrap_or(region.height() as f64),
        )?
    };

    Some(subregion)
}

fn get_input(
    input: &usvg::filter::Input,
    region: IntRect,
    inputs: &FilterInputs,
    results: &[FilterResult],
) -> Result<Image, Error> {
    let convert = |in_image: Option<&tiny_skia::Pixmap>, region: IntRect| {
        let image = if let Some(image) = in_image {
            image.clone()
        } else {
            tiny_skia::Pixmap::try_create(region.width(), region.height())?
        };

        Ok(Image {
            image: Rc::new(image),
            region,
            color_space: usvg::filter::ColorInterpolation::SRGB,
        })
    };

    let convert_alpha = |mut image: tiny_skia::Pixmap| {
        // Set RGB to black. Keep alpha as is.
        for p in image.data_mut().as_rgba_mut() {
            p.r = 0;
            p.g = 0;
            p.b = 0;
        }

        Ok(Image {
            image: Rc::new(image),
            region,
            color_space: usvg::filter::ColorInterpolation::SRGB,
        })
    };

    match input {
        usvg::filter::Input::SourceGraphic => {
            let image = inputs.source.clone();

            Ok(Image {
                image: Rc::new(image),
                region,
                color_space: usvg::filter::ColorInterpolation::SRGB,
            })
        }
        usvg::filter::Input::SourceAlpha => {
            let image = inputs.source.clone();
            convert_alpha(image)
        }
        usvg::filter::Input::BackgroundImage => {
            get_input(&usvg::filter::Input::SourceGraphic, region, inputs, results)
        }
        usvg::filter::Input::BackgroundAlpha => {
            get_input(&usvg::filter::Input::SourceAlpha, region, inputs, results)
        }
        usvg::filter::Input::FillPaint => convert(inputs.fill_paint, region),
        usvg::filter::Input::StrokePaint => convert(inputs.stroke_paint, region),
        usvg::filter::Input::Reference(ref name) => {
            if let Some(v) = results.iter().rev().find(|v| v.name == *name) {
                Ok(v.image.clone())
            } else {
                // Technically unreachable.
                log::warn!("Unknown filter primitive reference '{}'.", name);
                get_input(&usvg::filter::Input::SourceGraphic, region, inputs, results)
            }
        }
    }
}

fn apply_drop_shadow(
    fe: &usvg::filter::DropShadow,
    cs: usvg::filter::ColorInterpolation,
    ts: &usvg::Transform,
    input: Image,
) -> Result<Image, Error> {
    let mut pixmap = tiny_skia::Pixmap::try_create(input.width(), input.height())?;
    let input_pixmap = input.into_color_space(cs)?.take()?;
    let mut shadow_pixmap = input_pixmap.clone();

    let (sx, sy) = ts.get_scale();
    if let Some((std_dx, std_dy, box_blur)) =
        resolve_std_dev(fe.std_dev_x.get() * sx, fe.std_dev_y.get() * sy)
    {
        if box_blur {
            svgfilters::box_blur(std_dx, std_dy, into_svgfilters_image_mut!(shadow_pixmap));
        } else {
            svgfilters::iir_blur(std_dx, std_dy, into_svgfilters_image_mut!(shadow_pixmap));
        }
    }

    // flood
    let color = tiny_skia::Color::from_rgba8(
        fe.color.red,
        fe.color.green,
        fe.color.blue,
        fe.opacity.to_u8(),
    );
    for p in shadow_pixmap.pixels_mut() {
        let mut color = color;
        color.apply_opacity(p.alpha() as f32 / 255.0);
        *p = color.premultiply().to_color_u8();
    }

    match cs {
        usvg::filter::ColorInterpolation::SRGB => shadow_pixmap.into_srgb(),
        usvg::filter::ColorInterpolation::LinearRGB => shadow_pixmap.into_linear_rgb(),
    }

    pixmap.draw_pixmap(
        (fe.dx * sx) as i32,
        (fe.dy * sy) as i32,
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
    cs: usvg::filter::ColorInterpolation,
    ts: &usvg::Transform,
    input: Image,
) -> Result<Image, Error> {
    let (sx, sy) = ts.get_scale();
    let (std_dx, std_dy, box_blur) =
        match resolve_std_dev(fe.std_dev_x.get() * sx, fe.std_dev_y.get() * sy) {
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
    ts: &usvg::Transform,
    input: Image,
) -> Result<Image, Error> {
    let (sx, sy) = ts.get_scale();
    let dx = fe.dx * sx;
    let dy = fe.dy * sy;

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
    region: IntRect,
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

    pixmap.draw_pixmap(
        0,
        0,
        input1.as_ref().as_ref(),
        &tiny_skia::PixmapPaint {
            blend_mode: crate::tree::convert_blend_mode(fe.mode),
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
    region: IntRect,
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
            k1,
            k2,
            k3,
            k4,
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
    region: IntRect,
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

fn apply_flood(fe: &usvg::filter::Flood, region: IntRect) -> Result<Image, Error> {
    let c = fe.color;

    let mut pixmap = tiny_skia::Pixmap::try_create(region.width(), region.height())?;
    pixmap.fill(tiny_skia::Color::from_rgba8(
        c.red,
        c.green,
        c.blue,
        fe.opacity.to_u8(),
    ));

    Ok(Image::from_image(
        pixmap,
        usvg::filter::ColorInterpolation::SRGB,
    ))
}

fn apply_tile(input: Image, region: IntRect) -> Result<Image, Error> {
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
    let rect = tiny_skia::Rect::from_xywh(0.0, 0.0, region.width() as f32, region.height() as f32)
        .unwrap();
    pixmap.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);

    Ok(Image::from_image(
        pixmap,
        usvg::filter::ColorInterpolation::SRGB,
    ))
}

fn apply_image(
    fe: &usvg::filter::Image,
    region: IntRect,
    subregion: IntRect,
    ts: &usvg::Transform,
) -> Result<Image, Error> {
    let mut pixmap = tiny_skia::Pixmap::try_create(region.width(), region.height())?;

    match fe.data {
        usvg::filter::ImageKind::Image(ref kind) => {
            let dx = (subregion.x() - region.x()) as f32;
            let dy = (subregion.y() - region.y()) as f32;
            let transform = tiny_skia::Transform::from_translate(dx, dy);

            let view_box = usvg::ViewBox {
                rect: subregion.translate_to(0, 0).to_rect(),
                aspect: fe.aspect,
            };

            let uimage = usvg::Image {
                id: String::new(),
                transform: usvg::Transform::default(),
                visibility: usvg::Visibility::Visible,
                view_box,
                rendering_mode: fe.rendering_mode,
                kind: kind.clone(),
            };

            let mut children = Vec::new();
            crate::image::convert(&uimage, &mut children);
            if let Some(Node::Image(image)) = children.first() {
                crate::image::render_image(&image, transform, &mut pixmap.as_mut());
            }
        }
        usvg::filter::ImageKind::Use(ref node) => {
            let (sx, sy) = ts.get_scale();
            let transform = tiny_skia::Transform::from_scale(sx as f32, sy as f32);

            if let Some(mut rtree) = crate::Tree::from_usvg_node(node) {
                rtree.view_box.rect = rtree.view_box.rect.translate_to(0.0, 0.0);
                rtree.render(transform, &mut pixmap.as_mut());
            }
        }
    }

    Ok(Image::from_image(
        pixmap,
        usvg::filter::ColorInterpolation::SRGB,
    ))
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
        usvg::filter::ColorMatrixKind::Matrix(ref data) => {
            svgfilters::ColorMatrix::Matrix(data.as_slice().try_into().unwrap())
        }
        usvg::filter::ColorMatrixKind::Saturate(n) => svgfilters::ColorMatrix::Saturate(n.get()),
        usvg::filter::ColorMatrixKind::HueRotate(n) => svgfilters::ColorMatrix::HueRotate(n),
        usvg::filter::ColorMatrixKind::LuminanceToAlpha => {
            svgfilters::ColorMatrix::LuminanceToAlpha
        }
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
        fe.matrix.target_x,
        fe.matrix.target_y,
        fe.matrix.columns,
        fe.matrix.rows,
        &fe.matrix.data,
    )
    .unwrap();

    let edge_mode = match fe.edge_mode {
        usvg::filter::EdgeMode::None => svgfilters::EdgeMode::None,
        usvg::filter::EdgeMode::Duplicate => svgfilters::EdgeMode::Duplicate,
        usvg::filter::EdgeMode::Wrap => svgfilters::EdgeMode::Wrap,
    };

    svgfilters::convolve_matrix(
        matrix,
        fe.divisor.value(),
        fe.bias,
        edge_mode,
        fe.preserve_alpha,
        into_svgfilters_image_mut!(pixmap),
    );

    Ok(Image::from_image(pixmap, cs))
}

fn apply_morphology(
    fe: &usvg::filter::Morphology,
    cs: usvg::filter::ColorInterpolation,
    ts: &usvg::Transform,
    input: Image,
) -> Result<Image, Error> {
    let mut pixmap = input.into_color_space(cs)?.take()?;

    let (sx, sy) = ts.get_scale();
    let rx = fe.radius_x.get() * sx;
    let ry = fe.radius_y.get() * sy;

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
    region: IntRect,
    cs: usvg::filter::ColorInterpolation,
    ts: &usvg::Transform,
    input1: Image,
    input2: Image,
) -> Result<Image, Error> {
    let pixmap1 = input1.into_color_space(cs)?.take()?;
    let pixmap2 = input2.into_color_space(cs)?.take()?;

    let mut pixmap = tiny_skia::Pixmap::try_create(region.width(), region.height())?;

    let (sx, sy) = ts.get_scale();

    svgfilters::displacement_map(
        fe.x_channel_selector.into_svgf(),
        fe.y_channel_selector.into_svgf(),
        fe.scale * sx,
        fe.scale * sy,
        into_svgfilters_image!(&pixmap1),
        into_svgfilters_image!(&pixmap2),
        into_svgfilters_image_mut!(pixmap),
    );

    Ok(Image::from_image(pixmap, cs))
}

fn apply_turbulence(
    fe: &usvg::filter::Turbulence,
    region: IntRect,
    cs: usvg::filter::ColorInterpolation,
    ts: &usvg::Transform,
) -> Result<Image, Error> {
    let mut pixmap = tiny_skia::Pixmap::try_create(region.width(), region.height())?;

    let (sx, sy) = ts.get_scale();
    let (dx, dy) = ts.get_translate();
    if sx.is_fuzzy_zero() || sy.is_fuzzy_zero() {
        return Ok(Image::from_image(pixmap, cs));
    }

    svgfilters::turbulence(
        region.x() as f64 - dx,
        region.y() as f64 - dy,
        sx,
        sy,
        fe.base_frequency.x.get(),
        fe.base_frequency.y.get(),
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
    region: IntRect,
    cs: usvg::filter::ColorInterpolation,
    ts: &usvg::Transform,
    input: Image,
) -> Result<Image, Error> {
    let mut pixmap = tiny_skia::Pixmap::try_create(region.width(), region.height())?;

    let light_source = transform_light_source(fe.light_source, region, ts);

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
    region: IntRect,
    cs: usvg::filter::ColorInterpolation,
    ts: &usvg::Transform,
    input: Image,
) -> Result<Image, Error> {
    let mut pixmap = tiny_skia::Pixmap::try_create(region.width(), region.height())?;

    let light_source = transform_light_source(fe.light_source, region, ts);

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

fn transform_light_source(
    mut source: usvg::filter::LightSource,
    region: IntRect,
    ts: &Transform,
) -> usvg::filter::LightSource {
    use std::f64::consts::SQRT_2;
    use usvg::filter::LightSource;

    match source {
        LightSource::DistantLight(..) => {}
        LightSource::PointLight(ref mut light) => {
            let (x, y) = ts.apply(light.x, light.y);
            light.x = x - region.x() as f64;
            light.y = y - region.y() as f64;
            light.z = light.z * (ts.a * ts.a + ts.d * ts.d).sqrt() / SQRT_2;
        }
        LightSource::SpotLight(ref mut light) => {
            let sz = (ts.a * ts.a + ts.d * ts.d).sqrt() / SQRT_2;

            let (x, y) = ts.apply(light.x, light.y);
            light.x = x - region.x() as f64;
            light.y = y - region.x() as f64;
            light.z *= sz;

            let (x, y) = ts.apply(light.points_at_x, light.points_at_y);
            light.points_at_x = x - region.x() as f64;
            light.points_at_y = y - region.x() as f64;
            light.points_at_z *= sz;
        }
    }

    source
}

fn apply_to_canvas(input: Image, pixmap: &mut tiny_skia::Pixmap) -> Result<(), Error> {
    let input = input.into_color_space(usvg::filter::ColorInterpolation::SRGB)?;

    pixmap.fill(tiny_skia::Color::TRANSPARENT);
    pixmap.draw_pixmap(
        0,
        0,
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
fn resolve_std_dev(mut std_dx: f64, mut std_dy: f64) -> Option<(f64, f64, bool)> {
    use usvg::ApproxEqUlps;

    // 'A negative value or a value of zero disables the effect of the given filter primitive
    // (i.e., the result is the filter input image).'
    if std_dx.approx_eq_ulps(&0.0, 4) && std_dy.approx_eq_ulps(&0.0, 4) {
        return None;
    }

    // Ignore tiny sigmas. In case of IIR blur it can lead to a transparent image.
    if std_dx < 0.05 {
        std_dx = 0.0;
    }

    if std_dy < 0.05 {
        std_dy = 0.0;
    }

    const BLUR_SIGMA_THRESHOLD: f64 = 2.0;
    // Check that the current feGaussianBlur filter can be applied using a box blur.
    let box_blur = std_dx >= BLUR_SIGMA_THRESHOLD || std_dy >= BLUR_SIGMA_THRESHOLD;

    Some((std_dx, std_dy, box_blur))
}

/// Converts coordinates from `objectBoundingBox` to the `userSpaceOnUse`.
fn scale_coordinates(
    x: f64,
    y: f64,
    units: usvg::Units,
    bbox: Option<usvg::Rect>,
) -> Option<(f64, f64)> {
    if units == usvg::Units::ObjectBoundingBox {
        let bbox = bbox?;
        Some((x * bbox.width(), y * bbox.height()))
    } else {
        Some((x, y))
    }
}
