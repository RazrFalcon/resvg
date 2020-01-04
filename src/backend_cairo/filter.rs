// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::cmp;
use std::rc::Rc;

use rgb::FromSlice;
use log::warn;
use usvg::ColorInterpolation as ColorSpace;

use crate::prelude::*;
use crate::filter::{self, Filter, ImageExt, IntoSvgFilters, Error};
use crate::ConvTransform;
use super::ReCairoContextExt;

type Image = filter::Image<cairo::ImageSurface>;
type FilterInputs<'a> = filter::FilterInputs<'a, cairo::ImageSurface>;
type FilterResult = filter::FilterResult<cairo::ImageSurface>;


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
    CairoFilter::apply(filter, bbox, ts, opt, tree, background, fill_paint, stroke_paint, canvas);
}


impl ImageExt for cairo::ImageSurface {
    fn width(&self) -> u32 {
        self.get_width() as u32
    }

    fn height(&self) -> u32 {
        self.get_height() as u32
    }

    fn try_clone(&self) -> Result<Self, Error> {
        let new_image = create_image(self.width(), self.height())?;

        let cr = cairo::Context::new(&new_image);
        cr.set_source_surface(self, 0.0, 0.0);
        cr.paint();

        Ok(new_image)
    }

    fn clip(&mut self, region: ScreenRect) {
        let cr = cairo::Context::new(self);
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        cr.set_operator(cairo::Operator::Clear);

        cr.rectangle(0.0, 0.0, self.width() as f64, region.y() as f64);
        cr.rectangle(0.0, 0.0, region.x() as f64, self.height() as f64);
        cr.rectangle(region.right() as f64, 0.0, self.width() as f64, self.height() as f64);
        cr.rectangle(0.0, region.bottom() as f64, self.width() as f64, self.height() as f64);

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

fn create_image(width: u32, height: u32) -> Result<cairo::ImageSurface, Error> {
    cairo::ImageSurface::create(cairo::Format::ARgb32, width as i32, height as i32)
        .map_err(|_| Error::AllocFailed)
}

fn copy_image(
    image: &cairo::ImageSurface,
    region: ScreenRect,
) -> Result<cairo::ImageSurface, Error> {
    let x = cmp::max(0, region.x()) as f64;
    let y = cmp::max(0, region.y()) as f64;

    let new_image = create_image(region.width(), region.height())?;

    let cr = cairo::Context::new(&new_image);
    cr.set_source_surface(&*image, -x, -y);
    cr.paint();

    Ok(new_image)
}

struct CairoFilter;

impl Filter<cairo::ImageSurface> for CairoFilter {
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
                let image = Self::get_input(
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
                    Self::get_input(&usvg::FilterInput::SourceGraphic, region, inputs, results)
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
        let (std_dx, std_dy, box_blur)
            = try_opt_or!(Self::resolve_std_dev(fe, units, bbox, ts), Ok(input));

        let mut buffer = input.into_color_space(cs)?.take()?;
        let (w, h) = (buffer.width(), buffer.height());
        if let Ok(ref mut data) = buffer.get_data() {
            let img = svgfilters::ImageRefMut::new(data.as_bgra_mut(), w, h);
            if box_blur {
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
        let (dx, dy) = try_opt_or!(Self::scale_coordinates(fe.dx, fe.dy, units, bbox, ts), Ok(input));
        if dx.is_fuzzy_zero() && dy.is_fuzzy_zero() {
            return Ok(input);
        }

        // TODO: do not use an additional buffer
        let buffer = create_image(input.width(), input.height())?;

        let cr = cairo::Context::new(&buffer);
        cr.set_source_surface(input.as_ref(), dx, dy);
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

        let buffer = create_image(region.width(), region.height())?;
        let cr = cairo::Context::new(&buffer);

        cr.set_source_surface(input2.as_ref(), 0.0, 0.0);
        cr.paint();

        let operator = match fe.mode {
            usvg::FeBlendMode::Normal => cairo::Operator::Over,
            usvg::FeBlendMode::Multiply => cairo::Operator::Multiply,
            usvg::FeBlendMode::Screen => cairo::Operator::Screen,
            usvg::FeBlendMode::Darken => cairo::Operator::Darken,
            usvg::FeBlendMode::Lighten => cairo::Operator::Lighten,
        };

        cr.set_operator(operator);
        cr.set_source_surface(input1.as_ref(), 0.0, 0.0);
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

        let mut buffer = create_image(region.width(), region.height())?;

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
        let buffer = create_image(region.width(), region.height())?;
        let cr = cairo::Context::new(&buffer);

        for input in &fe.inputs {
            let input = Self::get_input(input, region, inputs, results)?;
            let input = input.into_color_space(cs)?;

            cr.set_source_surface(input.as_ref(), 0.0, 0.0);
            cr.paint();
        }

        Ok(Image::from_image(buffer, cs))
    }

    fn apply_flood(
        fe: &usvg::FeFlood,
        region: ScreenRect,
    ) -> Result<Image, Error> {
        let buffer = create_image(region.width(), region.height())?;

        let cr = cairo::Context::new(&buffer);
        cr.set_source_color(fe.color, fe.opacity);
        cr.paint();

        Ok(Image::from_image(buffer, ColorSpace::SRGB))
    }

    fn apply_tile(
        input: Image,
        region: ScreenRect,
    ) -> Result<Image, Error> {
        let buffer = create_image(region.width(), region.height())?;

        let subregion = input.region.translate(-region.x(), -region.y());

        let tile = copy_image(&input.image, subregion)?;
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
        let buffer = create_image(region.width(), region.height())?;

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
                    let mut layers = super::create_layers(region.size());
                    let cr = cairo::Context::new(&buffer);

                    let (sx, sy) = ts.get_scale();
                    cr.scale(sx, sy);
                    cr.transform(node.transform().to_native());

                    super::render_node(node, opt, &mut crate::RenderState::Ok, &mut layers, &cr);
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
        let (w, h) = (buffer.width(), buffer.height());
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
        let (w, h) = (buffer.width(), buffer.height());
        if let Ok(ref mut data) = buffer.get_data() {
            svgfilters::demultiply_alpha(data.as_bgra_mut());
            svgfilters::color_matrix(
                fe.kind.into_svgf(), svgfilters::ImageRefMut::new(data.as_bgra_mut(), w, h),
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

        let (w, h) = (buffer.width(), buffer.height());
        if let Ok(ref mut data) = buffer.get_data() {
            svgfilters::convolve_matrix(
                fe.matrix.into_svgf(), fe.divisor.value(), fe.bias,
                fe.edge_mode.into_svgf(), fe.preserve_alpha,
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
            Self::scale_coordinates(fe.radius_x.value(), fe.radius_y.value(), units, bbox, ts),
            Ok(Image::from_image(buffer, cs))
        );

        if !(rx > 0.0 && ry > 0.0) {
            buffer.clear();
            return Ok(Image::from_image(buffer, cs));
        }

        let (w, h) = (buffer.width(), buffer.height());
        if let Ok(ref mut data) = buffer.get_data() {
            svgfilters::morphology(
                fe.operator.into_svgf(), rx, ry,
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
            Self::scale_coordinates(fe.scale, fe.scale, units, bbox, ts),
            Ok(Image::from_image(buffer1, cs))
        );

        let mut buffer = create_image(region.width(), region.height())?;

        let (w, h) = (buffer.width(), buffer.height());
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
        let mut buffer = create_image(region.width(), region.height())?;
        let (sx, sy) = ts.get_scale();
        if sx.is_fuzzy_zero() || sy.is_fuzzy_zero() {
            return Ok(Image::from_image(buffer, cs));
        }

        let (w, h) = (buffer.width(), buffer.height());
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
        let mut buffer = create_image(region.width(), region.height())?;

        let light_source = crate::filter::transform_light_source(region, ts, fe.light_source);

        let (w, h) = (buffer.width(), buffer.height());
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
        let mut buffer = create_image(region.width(), region.height())?;

        let light_source = crate::filter::transform_light_source(region, ts, fe.light_source);

        let (w, h) = (buffer.width(), buffer.height());
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
        cr.set_source_surface(input.as_ref(), region.x() as f64, region.y() as f64);
        cr.paint();

        Ok(())
    }
}
