// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::cmp;
use std::rc::Rc;

use crate::skia;
use rgb::FromSlice;
use log::warn;
use usvg::ColorInterpolation as ColorSpace;

use crate::prelude::*;
use crate::filter::{self, Error, Filter, ImageExt, IntoSvgFilters};
use crate::ConvTransform;

type Image = filter::Image<skia::Surface>;
type FilterInputs<'a> = filter::FilterInputs<'a, skia::Surface>;
type FilterResult = filter::FilterResult<skia::Surface>;

macro_rules! into_svgfilters_image {
    ($img:expr) => { svgfilters::ImageRef::new($img.data().as_bgra(), $img.width(), $img.height()) };
}

macro_rules! into_svgfilters_image_mut {
    ($img:expr) => { into_svgfilters_image_mut($img.width(), $img.height(), &mut $img.data_mut()) };
}

// We need a macro and a function to resolve lifetimes.
fn into_svgfilters_image_mut<'a>(width: u32, height: u32, data: &'a mut skia::SurfaceData)
    -> svgfilters::ImageRefMut<'a>
{
    svgfilters::ImageRefMut::new(data.as_bgra_mut(), width, height)
}

pub fn apply(
    filter: &usvg::Filter,
    bbox: Option<Rect>,
    ts: &usvg::Transform,
    opt: &Options,
    tree: &usvg::Tree,
    background: Option<&skia::Surface>,
    fill_paint: Option<&skia::Surface>,
    stroke_paint: Option<&skia::Surface>,
    canvas: &mut skia::Surface,
) {
    SkiaFilter::apply(filter, bbox, ts, opt, tree, background, fill_paint, stroke_paint, canvas);
}

impl ImageExt for skia::Surface {
    fn width(&self) -> u32 {
        self.width() as u32
    }

    fn height(&self) -> u32 {
        self.height() as u32
    }

    fn try_clone(&self) -> Result<Self, Error> {
        self.try_clone().ok_or(Error::AllocFailed)
    }

    fn clip(&mut self, region: ScreenRect) {
        // This is cropping by clearing the pixels outside the region.
        let mut paint = skia::Paint::new();
        paint.set_color(0, 0, 0, 0);
        paint.set_blend_mode(skia::BlendMode::Clear);

        let w = self.width() as f64;
        let h = self.height() as f64;

        self.draw_rect(0.0, 0.0, w, region.y() as f64, &paint);
        self.draw_rect(0.0, 0.0, region.x() as f64, h, &paint);
        self.draw_rect(region.right() as f64, 0.0, w, h, &paint);
        self.draw_rect(0.0, region.bottom() as f64, w, h, &paint);
    }

    fn clear(&mut self) {
        skia::Canvas::clear(self);
    }

    fn into_srgb(&mut self) {
        svgfilters::from_linear_rgb(self.data_mut().as_bgra_mut());
    }

    fn into_linear_rgb(&mut self) {
        svgfilters::into_linear_rgb(self.data_mut().as_bgra_mut());
    }
}

fn create_surface(width: u32, height: u32) -> Result<skia::Surface, Error> {
    let mut surface = skia::Surface::new_rgba(width, height).ok_or(Error::AllocFailed)?;
    surface.clear();
    Ok(surface)
}

fn copy_surface(surface: &skia::Surface, region: ScreenRect) -> Result<skia::Surface, Error> {
    let x = cmp::max(0, region.x()) as u32;
    let y = cmp::max(0, region.y()) as u32;
    surface.copy_rgba(x, y, region.width(), region.height()).ok_or(Error::AllocFailed)
}

struct SkiaFilter;

impl Filter<skia::Surface> for SkiaFilter {
    fn get_input(
        input: &usvg::FilterInput,
        region: ScreenRect,
        inputs: &FilterInputs,
        results: &[FilterResult],
    ) -> Result<Image, Error> {
        let convert = |in_image, region| {
            let image = if let Some(image) = in_image {
                copy_surface(image, region)?
            } else {
                create_surface(region.width(), region.height())?
            };

            Ok(Image {
                image: Rc::new(image),
                region: region.translate_to(0, 0),
                color_space: ColorSpace::SRGB,
            })
        };

        let convert_alpha = |image: skia::Surface| {
            // Set RGB to black. Keep alpha as is.
            for p in image.data().chunks_mut(4) {
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
                let image = copy_surface(inputs.source, region)?;

                Ok(Image {
                    image: Rc::new(image),
                    region: region.translate_to(0, 0),
                    color_space: ColorSpace::SRGB,
                })
            }
            usvg::FilterInput::SourceAlpha => {
                let image = copy_surface(inputs.source, region)?;
                convert_alpha(image)
            }
            usvg::FilterInput::BackgroundImage => {
                convert(inputs.background, region)
            }
            usvg::FilterInput::BackgroundAlpha => {
                let image = Self::get_input(
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
                    Self::get_input(
                        &usvg::FilterInput::SourceGraphic, region, inputs, results,
                    )
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

        // Skia surface can be RGBA, but it will not affect the blur algorithm.
        svgfilters::multiply_alpha(buffer.data_mut().as_bgra_mut());

        if box_blur {
            svgfilters::box_blur(std_dx, std_dy, into_svgfilters_image_mut!(buffer));
        } else {
            svgfilters::iir_blur(std_dx, std_dy, into_svgfilters_image_mut!(buffer));
        }

        svgfilters::demultiply_alpha(buffer.data_mut().as_bgra_mut());

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

        let mut buffer = create_surface(input.width(), input.height())?;

        buffer.reset_matrix();
        buffer.draw_surface(input.as_ref(), dx, dy, 255, skia::BlendMode::SourceOver,
                            skia::FilterQuality::Low);
        buffer.flush();

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

        let mut buffer = create_surface(region.width(), region.height())?;

        buffer.draw_surface(input2.as_ref(), 0.0, 0.0, 255, skia::BlendMode::SourceOver,
                            skia::FilterQuality::Low);

        let blend_mode = match fe.mode {
            usvg::FeBlendMode::Normal => skia::BlendMode::SourceOver,
            usvg::FeBlendMode::Multiply => skia::BlendMode::Multiply,
            usvg::FeBlendMode::Screen => skia::BlendMode::Screen,
            usvg::FeBlendMode::Darken => skia::BlendMode::Darken,
            usvg::FeBlendMode::Lighten => skia::BlendMode::Lighten,
        };

        buffer.draw_surface(input1.as_ref(), 0.0, 0.0, 255, blend_mode,
                            skia::FilterQuality::Low);

        Ok(Image::from_image(buffer, cs))
    }

    fn apply_composite(
        fe: &usvg::FeComposite,
        cs: ColorSpace,
        region: ScreenRect,
        input1: Image,
        input2: Image,
    ) -> Result<Image, Error> {
        use usvg::FeCompositeOperator as Operator;

        let input1 = input1.into_color_space(cs)?;
        let input2 = input2.into_color_space(cs)?;

        let mut buffer = create_surface(region.width(), region.height())?;

        if let Operator::Arithmetic { k1, k2, k3, k4 } = fe.operator {
            let mut buffer1 = input1.take()?;
            let mut buffer2 = input2.take()?;
            svgfilters::multiply_alpha(buffer1.data_mut().as_bgra_mut());
            svgfilters::multiply_alpha(buffer2.data_mut().as_bgra_mut());

            svgfilters::arithmetic_composite(
                k1, k2, k3, k4,
                into_svgfilters_image!(buffer1),
                into_svgfilters_image!(buffer2),
                into_svgfilters_image_mut!(buffer),
            );

            svgfilters::demultiply_alpha(buffer.data_mut().as_bgra_mut());

            return Ok(Image::from_image(buffer, cs));
        }

        buffer.draw_surface(input2.as_ref(), 0.0, 0.0, 255, skia::BlendMode::SourceOver,
                            skia::FilterQuality::Low);
        let blend_mode = match fe.operator {
            Operator::Over => skia::BlendMode::SourceOver,
            Operator::In => skia::BlendMode::SourceIn,
            Operator::Out => skia::BlendMode::SourceOut,
            Operator::Atop => skia::BlendMode::SourceAtop,
            Operator::Xor => skia::BlendMode::Xor,
            Operator::Arithmetic { .. } => skia::BlendMode::SourceOver,
        };
        buffer.draw_surface(input1.as_ref(), 0.0, 0.0, 255, blend_mode,
                            skia::FilterQuality::Low);

        Ok(Image::from_image(buffer, cs))
    }

    fn apply_merge(
        fe: &usvg::FeMerge,
        cs: ColorSpace,
        region: ScreenRect,
        inputs: &FilterInputs,
        results: &[FilterResult],
    ) -> Result<Image, Error> {
        let mut buffer = create_surface(region.width(), region.height())?;
        buffer.reset_matrix();

        for input in &fe.inputs {
            let input = Self::get_input(input, region, inputs, results)?;
            let input = input.into_color_space(cs)?;
            buffer.draw_surface(input.as_ref(), 0.0, 0.0, 255, skia::BlendMode::SourceOver,
                                skia::FilterQuality::Low);
        }
        buffer.flush();

        Ok(Image::from_image(buffer, cs))
    }

    fn apply_flood(
        fe: &usvg::FeFlood,
        region: ScreenRect,
    ) -> Result<Image, Error> {
        let c = fe.color;
        let alpha = (fe.opacity.value() * 255.0) as u8;

        let mut buffer = create_surface(region.width(), region.height())?;
        buffer.fill(c.red, c.green, c.blue, alpha);

        Ok(Image::from_image(buffer, ColorSpace::SRGB))
    }

    fn apply_tile(
        input: Image,
        region: ScreenRect,
    ) -> Result<Image, Error> {
        let mut buffer = create_surface(region.width(), region.height())?;

        let subregion = input.region.translate(-region.x(), -region.y());

        let tile_surface = copy_surface(&input.image, subregion)?;
        let brush_ts = usvg::Transform::new_translate(subregion.x() as f64, subregion.y() as f64);
        let shader = skia::Shader::new_from_surface_image(&tile_surface, brush_ts.to_native());
        let mut paint = skia::Paint::new();
        paint.set_shader(&shader);

        buffer.draw_rect(0.0, 0.0, region.width() as f64, region.height() as f64, &paint);

        buffer.reset_matrix();
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
        let mut buffer = create_surface(region.width(), region.height())?;

        match fe.data {
            usvg::FeImageKind::Image(ref data, format) => {
                let dx = (subregion.x() - region.x()) as f64;
                let dy = (subregion.y() - region.y()) as f64;
                buffer.translate(dx, dy);

                let view_box = usvg::ViewBox {
                    rect: subregion.translate_to(0, 0).to_rect(),
                    aspect: fe.aspect,
                };

                if format == usvg::ImageFormat::SVG {
                    super::image::draw_svg(data, view_box, opt, &mut buffer);
                } else {
                    super::image::draw_raster(
                        format, data, view_box, fe.rendering_mode, opt, &mut buffer,
                    );
                }
            }
            usvg::FeImageKind::Use(ref id) => {
                if let Some(ref node) = tree.defs_by_id(id).or(tree.node_by_id(id)) {
                    let mut layers = super::create_layers(region.size());

                    let (sx, sy) = ts.get_scale();
                    buffer.scale(sx, sy);
                    buffer.concat(&node.transform().to_native());

                    super::render_node(node, opt, &mut crate::RenderState::Ok, &mut layers, &mut buffer);
                }
            }
        }

        buffer.reset_matrix();
        Ok(Image::from_image(buffer, ColorSpace::SRGB))
    }

    fn apply_component_transfer(
        fe: &usvg::FeComponentTransfer,
        cs: ColorSpace,
        input: Image,
    ) -> Result<Image, Error> {
        let mut buffer = input.into_color_space(cs)?.take()?;

        reverse_rgb(&mut buffer);
        svgfilters::component_transfer(
            fe.func_b.into_svgf(),
            fe.func_g.into_svgf(),
            fe.func_r.into_svgf(),
            fe.func_a.into_svgf(),
            into_svgfilters_image_mut!(buffer),
        );
        reverse_rgb(&mut buffer);

        Ok(Image::from_image(buffer, cs))
    }

    fn apply_color_matrix(
        fe: &usvg::FeColorMatrix,
        cs: ColorSpace,
        input: Image,
    ) -> Result<Image, Error> {
        let mut buffer = input.into_color_space(cs)?.take()?;

        reverse_rgb(&mut buffer);
        svgfilters::color_matrix(fe.kind.into_svgf(), into_svgfilters_image_mut!(buffer));
        reverse_rgb(&mut buffer);

        Ok(Image::from_image(buffer, cs))
    }

    fn apply_convolve_matrix(
        fe: &usvg::FeConvolveMatrix,
        cs: ColorSpace,
        input: Image,
    ) -> Result<Image, Error> {
        let mut buffer = input.into_color_space(cs)?.take()?;

        reverse_rgb(&mut buffer);

        if !fe.preserve_alpha {
            svgfilters::multiply_alpha(buffer.data_mut().as_bgra_mut());
        }

        svgfilters::convolve_matrix(
            fe.matrix.into_svgf(), fe.divisor.value(), fe.bias,
            fe.edge_mode.into_svgf(), fe.preserve_alpha,
            into_svgfilters_image_mut!(buffer),
        );

        // `convolve_matrix` filter will premultiply channels,
        // so we have to undo it.
        svgfilters::demultiply_alpha(buffer.data_mut().as_bgra_mut());

        reverse_rgb(&mut buffer);

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

        reverse_rgb(&mut buffer);
        svgfilters::multiply_alpha(buffer.data_mut().as_bgra_mut());
        svgfilters::morphology(fe.operator.into_svgf(), rx, ry, into_svgfilters_image_mut!(buffer));
        svgfilters::demultiply_alpha(buffer.data_mut().as_bgra_mut());
        reverse_rgb(&mut buffer);

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
        let buffer1 = input1.into_color_space(cs)?.take()?;
        let buffer2 = input2.into_color_space(cs)?.take()?;
        let (sx, sy) = try_opt_or!(
            Self::scale_coordinates(fe.scale, fe.scale, units, bbox, ts),
            Ok(Image::from_image(buffer1, cs))
        );

        let mut buffer = create_surface(region.width(), region.height())?;

        reverse_rgb(&mut buffer);

        svgfilters::displacement_map(
            fe.x_channel_selector.into_svgf(),
            fe.y_channel_selector.into_svgf(),
            sx, sy,
            into_svgfilters_image!(&buffer1),
            into_svgfilters_image!(&buffer2),
            into_svgfilters_image_mut!(buffer),
        );

        Ok(Image::from_image(buffer, cs))
    }

    fn apply_turbulence(
        fe: &usvg::FeTurbulence,
        region: ScreenRect,
        cs: ColorSpace,
        ts: &usvg::Transform,
    ) -> Result<Image, Error> {
        let mut buffer = create_surface(region.width(), region.height())?;

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

        reverse_rgb(&mut buffer);

        Ok(Image::from_image(buffer, cs))
    }

    fn apply_diffuse_lighting(
        fe: &usvg::FeDiffuseLighting,
        region: ScreenRect,
        cs: ColorSpace,
        ts: &usvg::Transform,
        input: Image,
    ) -> Result<Image, Error> {
        let mut buffer = create_surface(region.width(), region.height())?;

        let light_source = crate::filter::transform_light_source(region, ts, fe.light_source);

        svgfilters::diffuse_lighting(
            fe.surface_scale,
            fe.diffuse_constant,
            fe.lighting_color.into_svgf(),
            light_source.into_svgf(),
            into_svgfilters_image!(input.as_ref()),
            into_svgfilters_image_mut!(buffer),
        );

        reverse_rgb(&mut buffer);

        Ok(Image::from_image(buffer, cs))
    }

    fn apply_specular_lighting(
        fe: &usvg::FeSpecularLighting,
        region: ScreenRect,
        cs: ColorSpace,
        ts: &usvg::Transform,
        input: Image,
    ) -> Result<Image, Error> {
        let mut buffer = create_surface(region.width(), region.height())?;

        let light_source = crate::filter::transform_light_source(region, ts, fe.light_source);

        svgfilters::specular_lighting(
            fe.surface_scale,
            fe.specular_constant,
            fe.specular_exponent,
            fe.lighting_color.into_svgf(),
            light_source.into_svgf(),
            into_svgfilters_image!(input.as_ref()),
            into_svgfilters_image_mut!(buffer),
        );

        svgfilters::demultiply_alpha(buffer.data_mut().as_bgra_mut());
        reverse_rgb(&mut buffer);

        Ok(Image::from_image(buffer, cs))
    }

    fn apply_to_canvas(
        input: Image,
        region: ScreenRect,
        canvas: &mut skia::Surface,
    ) -> Result<(), Error> {
        let input = input.into_color_space(ColorSpace::SRGB)?;

        canvas.reset_matrix();
        canvas.clear();
        canvas.draw_surface(input.as_ref(), region.x() as f64, region.y() as f64, 255,
                            skia::BlendMode::SourceOver, skia::FilterQuality::Low);

        Ok(())
    }
}

fn reverse_rgb(buffer: &mut skia::Surface) {
    use std::mem::swap;

    if !skia::Surface::is_bgra() {
        buffer.data_mut().as_bgra_mut().iter_mut().for_each(|p| swap(&mut p.r, &mut p.b));
    }
}
