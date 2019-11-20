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
use crate::filter::{self, Error, Filter, ImageExt, TransferFunctionExt};
use crate::ConvTransform;

type Image = filter::Image<skia::Surface>;
type FilterResult = filter::FilterResult<skia::Surface>;


pub fn apply(
    filter: &usvg::Filter,
    bbox: Option<Rect>,
    ts: &usvg::Transform,
    opt: &Options,
    background: Option<&skia::Surface>,
    canvas: &mut skia::Surface,
) {
    SkiaFilter::apply(filter, bbox, ts, opt, background, canvas);
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
        for p in self.data_mut().as_rgba_mut() {
            p.r = filter::LINEAR_RGB_TO_SRGB_TABLE[p.r as usize];
            p.g = filter::LINEAR_RGB_TO_SRGB_TABLE[p.g as usize];
            p.b = filter::LINEAR_RGB_TO_SRGB_TABLE[p.b as usize];
        }
    }

    fn into_linear_rgb(&mut self) {
        for p in self.data_mut().as_rgba_mut() {
            p.r = filter::SRGB_TO_LINEAR_RGB_TABLE[p.r as usize];
            p.g = filter::SRGB_TO_LINEAR_RGB_TABLE[p.g as usize];
            p.b = filter::SRGB_TO_LINEAR_RGB_TABLE[p.b as usize];
        }
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
        results: &[FilterResult],
        background: Option<&skia::Surface>,
        canvas: &skia::Surface,
    ) -> Result<Image, Error> {
        match input {
            usvg::FilterInput::SourceGraphic => {
                let image = copy_surface(canvas, region)?;

                Ok(Image {
                    image: Rc::new(image),
                    region: region.translate_to(0, 0),
                    color_space: ColorSpace::SRGB,
                })
            }
            usvg::FilterInput::SourceAlpha => {
                let image = copy_surface(canvas, region)?;

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
            }
            usvg::FilterInput::BackgroundImage => {
                let image = if let Some(image) = background {
                    copy_surface(image, region)?
                } else {
                    create_surface(canvas.width(), canvas.height())?
                };

                Ok(Image {
                    image: Rc::new(image),
                    region: region.translate_to(0, 0),
                    color_space: ColorSpace::SRGB,
                })
            }
            usvg::FilterInput::BackgroundAlpha => {
                let image = Self::get_input(
                    &usvg::FilterInput::BackgroundImage, region, results, background, canvas,
                )?;
                let image = image.take()?;

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
            }
            usvg::FilterInput::Reference(ref name) => {
                if let Some(ref v) = results.iter().rev().find(|v| v.name == *name) {
                    Ok(v.image.clone())
                } else {
                    // Technically unreachable.
                    warn!("Unknown filter primitive reference '{}'.", name);
                    Self::get_input(
                        &usvg::FilterInput::SourceGraphic, region, results, background, canvas,
                    )
                }
            }
            _ => {
                warn!("Filter input '{:?}' is not supported.", input);
                Self::get_input(
                    &usvg::FilterInput::SourceGraphic, region, results, background, canvas,
                )
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

        let input = input.into_color_space(cs)?;
        let mut buffer = input.take()?;

        let (w, h) = (buffer.width(), buffer.height());
        // Skia surface can be RGBA, but it will not affect the blur algorithm.
        filter::into_premultiplied(buffer.data_mut().as_bgra_mut());

        if box_blur {
            filter::box_blur::apply(&mut buffer.data_mut(), w, h, std_dx, std_dy);
        } else {
            filter::iir_blur::apply(&mut buffer.data_mut(), w, h, std_dx, std_dy);
        }

        filter::from_premultiplied(buffer.data_mut().as_bgra_mut());

        Ok(Image::from_image(buffer, cs))
    }

    fn apply_offset(
        fe: &usvg::FeOffset,
        units: usvg::Units,
        bbox: Option<Rect>,
        ts: &usvg::Transform,
        input: Image,
    ) -> Result<Image, Error> {
        let (dx, dy) = try_opt_or!(Self::resolve_offset(fe, units, bbox, ts), Ok(input));

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
        use rgb::RGBA8;
        use usvg::FeCompositeOperator as Operator;

        let input1 = input1.into_color_space(cs)?;
        let input2 = input2.into_color_space(cs)?;

        let mut buffer = create_surface(region.width(), region.height())?;

        if let Operator::Arithmetic { k1, k2, k3, k4 } = fe.operator {
            fn premultiply_alpha(c: RGBA8) -> RGBA8 {
                let a =  c.a as f64 / 255.0;
                let b = (c.b as f64 * a + 0.5) as u8;
                let g = (c.g as f64 * a + 0.5) as u8;
                let r = (c.r as f64 * a + 0.5) as u8;

                RGBA8 { r, g, b, a: c.a }
            }

            fn unmultiply_alpha(c: RGBA8) -> RGBA8 {
                let a =  c.a as f64 / 255.0;
                let b = (c.b as f64 / a + 0.5) as u8;
                let g = (c.g as f64 / a + 0.5) as u8;
                let r = (c.r as f64 / a + 0.5) as u8;

                RGBA8 { r, g, b, a: c.a }
            }

            let data1 = input1.as_ref().data();
            let data2 = input2.as_ref().data();

            let calc = |i1, i2, max| {
                let i1 = i1 as f64 / 255.0;
                let i2 = i2 as f64 / 255.0;
                let result = k1.value() * i1 * i2 + k2.value() * i1 + k3.value() * i2 + k4.value();
                f64_bound(0.0, result, max)
            };

            {
                let mut i = 0;
                let mut data3 = buffer.data_mut();
                let data3 = data3.as_rgba_mut();
                for (c1, c2) in data1.as_rgba().iter().zip(data2.as_rgba()) {
                    let c1 = premultiply_alpha(*c1);
                    let c2 = premultiply_alpha(*c2);

                    let a = calc(c1.a, c2.a, 1.0);
                    if a.is_fuzzy_zero() {
                        i += 1;
                        continue;
                    }

                    let r = (calc(c1.r, c2.r, a) * 255.0) as u8;
                    let g = (calc(c1.g, c2.g, a) * 255.0) as u8;
                    let b = (calc(c1.b, c2.b, a) * 255.0) as u8;
                    let a = (a * 255.0) as u8;

                    data3[i] = unmultiply_alpha(RGBA8 { r, g, b, a });

                    i += 1;
                }
            }

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
        results: &[FilterResult],
        background: Option<&skia::Surface>,
        canvas: &skia::Surface,
    ) -> Result<Image, Error> {
        let mut buffer = create_surface(region.width(), region.height())?;
        buffer.reset_matrix();

        for input in &fe.inputs {
            let input = Self::get_input(input, region, &results, background, canvas)?;
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
        let alpha = f64_bound(0.0, fe.opacity.value() * 255.0, 255.0) as u8;

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
    ) -> Result<Image, Error> {
        let mut buffer = create_surface(region.width(), region.height())?;

        match fe.data {
            usvg::FeImageKind::None => {}
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
            usvg::FeImageKind::Use(..) => {}
        }

        buffer.reset_matrix();
        Ok(Image::from_image(buffer, ColorSpace::SRGB))
    }

    fn apply_component_transfer(
        fe: &usvg::FeComponentTransfer,
        cs: ColorSpace,
        input: Image,
    ) -> Result<Image, Error> {
        let input = input.into_color_space(cs)?;
        let mut buffer = input.take()?;

        if skia::Surface::is_bgra() {
            for pixel in buffer.data_mut().as_bgra_mut() {
                pixel.r = fe.func_r.apply(pixel.r);
                pixel.g = fe.func_g.apply(pixel.g);
                pixel.b = fe.func_b.apply(pixel.b);
                pixel.a = fe.func_a.apply(pixel.a);
            }
        } else {
            for pixel in buffer.data_mut().as_rgba_mut() {
                pixel.r = fe.func_r.apply(pixel.r);
                pixel.g = fe.func_g.apply(pixel.g);
                pixel.b = fe.func_b.apply(pixel.b);
                pixel.a = fe.func_a.apply(pixel.a);
            }
        }

        Ok(Image::from_image(buffer, cs))
    }

    fn apply_color_matrix(
        fe: &usvg::FeColorMatrix,
        cs: ColorSpace,
        input: Image,
    ) -> Result<Image, Error> {
        use std::mem::swap;

        let input = input.into_color_space(cs)?;
        let mut buffer = input.take()?;
        let mut data = buffer.data_mut();

        // RGBA -> BGRA.
        if !skia::Surface::is_bgra() {
            data.as_bgra_mut().iter_mut().for_each(|p| swap(&mut p.r, &mut p.b));
        }

        filter::color_matrix::apply(&fe.kind, data.as_bgra_mut());

        // BGRA -> RGBA.
        if !skia::Surface::is_bgra() {
            data.as_bgra_mut().iter_mut().for_each(|p| swap(&mut p.r, &mut p.b));
        }

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
