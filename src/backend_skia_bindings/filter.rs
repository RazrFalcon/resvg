// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::cmp;
use std::rc::Rc;

use crate::skia;
use rgb::FromSlice;
use log::warn;
use usvg::{try_opt_or, ColorInterpolation as ColorSpace};

use crate::{prelude::*, backend_utils::*};
use crate::backend_utils::filter::{Error, Filter, ImageExt};
use super::ToData;
use skia::{TileMode, BlendMode, FilterQuality, ISize, Paint, Point, Surface};
use crate::backend_utils::ConvTransform;

type Image = filter::Image<Surface>;
type FilterResult = filter::FilterResult<Surface>;

pub fn apply(
    filter: &usvg::Filter,
    bbox: Option<Rect>,
    ts: &usvg::Transform,
    opt: &Options,
    surface: &mut Surface,
) {
    SkiaFilter::apply(filter, bbox, ts, opt, surface);
}

pub trait CanvasExt {
    fn draw_rect_from_top_left(&mut self, left: f32, top: f32, width: f32, height: f32, paint: &skia::Paint);

    fn just_clear(&mut self);

    fn draw_surface(
        &mut self,
        surface: &mut Surface,
        left: f64,
        top: f64,
        alpha: u8,
        blend_mode: BlendMode,
        filter_quality: FilterQuality,
    );

    fn fill(&mut self, r: u8, g: u8, b: u8, a: u8);
}

impl CanvasExt for skia::Canvas {
    fn draw_rect_from_top_left(&mut self, left: f32, top: f32, width: f32, height: f32, paint: &skia::Paint) {
        self.draw_rect(skia::Rect {
            left,
            top,
            right: left + width,
            bottom: top + height,
        }, paint);
    }

    fn just_clear(&mut self) {
        self.clear(skia::Color::TRANSPARENT);
    }

    fn draw_surface(
        &mut self,
        surface: &mut Surface,
        left: f64,
        top: f64,
        alpha: u8,
        blend_mode: BlendMode,
        filter_quality: FilterQuality,
    ) {
        let image = surface.image_snapshot();
        let mut paint = Paint::default();
        paint.set_filter_quality(filter_quality);
        paint.set_alpha(alpha);
        paint.set_blend_mode(blend_mode);
        self.draw_image(&image, Point::new(left as f32, top as f32), Some(&paint));
    }

    fn fill(&mut self, r: u8, g: u8, b: u8, a: u8) {
        self.clear(skia_safe::Color::from_argb(a, r, g, b));
    }
}

impl ImageExt for Surface {
    fn width(&self) -> u32 {
        self.width() as u32
    }

    fn height(&self) -> u32 {
        self.height() as u32
    }

    fn try_clone(&self) -> Result<Self, Error> {
        Ok(self.clone())
    }

    fn clip(&mut self, region: ScreenRect) {
        // This is cropping by clearing the pixels outside the region.
        let mut paint = skia::Paint::default();
        paint.set_argb(0, 0, 0, 0);
        paint.set_blend_mode(skia::BlendMode::Clear);

        let surface_width = self.width() as f32;
        let surface_height = self.height() as f32;

        let canvas = self.canvas();

        canvas.draw_rect_from_top_left(0.0, 0.0, surface_width, region.y() as f32, &paint);
        canvas.draw_rect_from_top_left(0.0, 0.0, region.x() as f32, surface_height, &paint);
        canvas.draw_rect_from_top_left(region.right() as f32, 0.0, surface_width, surface_height, &paint);
        canvas.draw_rect_from_top_left(0.0, region.bottom() as f32, surface_width, surface_height, &paint);
    }

    fn clear(&mut self) {
        self.canvas().just_clear();
    }

    fn into_srgb(&mut self) {
        for p in self.canvas().data_mut().as_rgba_mut() {
            p.r = filter::LINEAR_RGB_TO_SRGB_TABLE[p.r as usize];
            p.g = filter::LINEAR_RGB_TO_SRGB_TABLE[p.g as usize];
            p.b = filter::LINEAR_RGB_TO_SRGB_TABLE[p.b as usize];
        }
    }

    fn into_linear_rgb(&mut self) {
        for p in self.canvas().data_mut().as_rgba_mut() {
            p.r = filter::SRGB_TO_LINEAR_RGB_TABLE[p.r as usize];
            p.g = filter::SRGB_TO_LINEAR_RGB_TABLE[p.g as usize];
            p.b = filter::SRGB_TO_LINEAR_RGB_TABLE[p.b as usize];
        }
    }
}

fn create_surface(width: u32, height: u32) -> Result<Surface, Error> {
    let size = ISize::new(width as i32, height as i32);
    let color_space = skia::ColorSpace::new_srgb();
    let image_info = skia::ImageInfo::new_n32(size, skia::AlphaType::Unpremul, Some(color_space));
    let min_row_bytes = image_info.min_row_bytes();
    let surface = skia::Surface::new_raster(&image_info, min_row_bytes, None).ok_or(Error::AllocFailed)?;
    Ok(surface)
}

fn copy_surface(surface: &Surface, region: ScreenRect) -> Result<Surface, Error> {
    let x = cmp::max(0, region.x()) as i32;
    let y = cmp::max(0, region.y()) as i32;
    let mut mut_surf = surface.clone();
    let mut new_surface = create_surface(region.width(), region.height())?;
    let mut paint = skia::Paint::default();
    paint.set_filter_quality(skia::FilterQuality::Low);
    paint.set_alpha(255);
    mut_surf.draw(new_surface.canvas(), (-x, -y), Some(&paint));
    Ok(new_surface)
}

struct SkiaFilter;

impl Filter<Surface> for SkiaFilter {
    fn get_input(
        input: &usvg::FilterInput,
        region: ScreenRect,
        results: &[FilterResult],
        surface: &Surface,
    ) -> Result<Image, Error> {
        match input {
            usvg::FilterInput::SourceGraphic => {
                let image = copy_surface(surface, region)?;

                Ok(Image {
                    image: Rc::new(image),
                    region: region.translate_to(0, 0),
                    color_space: ColorSpace::SRGB,
                })
            }
            usvg::FilterInput::SourceAlpha => {
                let mut image = copy_surface(surface, region)?;

                // Set RGB to black. Keep alpha as is.
                for p in image.canvas().data_mut().chunks_mut(4) {
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
                    Self::get_input(&usvg::FilterInput::SourceGraphic, region, results, surface)
                }
            }
            _ => {
                warn!("Filter input '{:?}' is not supported.", input);
                Self::get_input(&usvg::FilterInput::SourceGraphic, region, results, surface)
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
        let (std_dx, std_dy) = try_opt_or!(Self::resolve_std_dev(fe, units, bbox, ts), Ok(input));

        let input = input.into_color_space(cs)?;
        let mut buffer = input.take()?;

        let (w, h) = (buffer.width(), buffer.height());
        filter::blur::apply(&mut buffer.canvas().data_mut(), w as u32, h as u32, std_dx, std_dy, 4);

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
        let canvas = buffer.canvas();

        canvas.reset_matrix();
        let mut surface = input.as_ref().clone();
        canvas.draw_surface(&mut surface, dx, dy, 255, skia::BlendMode::SrcOver,
                            skia::FilterQuality::Low);
        canvas.flush();

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

        let mut mut_input1 = input1.as_ref().clone();
        let mut mut_input2 = input2.as_ref().clone();

        let mut buffer = create_surface(region.width(), region.height())?;
        let canvas = buffer.canvas();

        canvas.draw_surface(&mut mut_input2, 0.0, 0.0, 255,
                            skia::BlendMode::SrcOver,
                            skia::FilterQuality::Low);

        let blend_mode = match fe.mode {
            usvg::FeBlendMode::Normal => skia::BlendMode::SrcOver,
            usvg::FeBlendMode::Multiply => skia::BlendMode::Multiply,
            usvg::FeBlendMode::Screen => skia::BlendMode::Screen,
            usvg::FeBlendMode::Darken => skia::BlendMode::Darken,
            usvg::FeBlendMode::Lighten => skia::BlendMode::Lighten,
        };

        canvas.draw_surface(&mut mut_input1, 0.0, 0.0, 255, blend_mode,
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

        let mut mut_input1 = input1.as_ref().clone();
        let mut mut_input2 = input2.as_ref().clone();

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

            let data1 = mut_input1.canvas().data_mut();
            let data2 = mut_input2.canvas().data_mut();

            let calc = |i1, i2, max| {
                let i1 = i1 as f64 / 255.0;
                let i2 = i2 as f64 / 255.0;
                let result = k1.value() * i1 * i2 + k2.value() * i1 + k3.value() * i2 + k4.value();
                f64_bound(0.0, result, max)
            };

            {
                let mut i = 0;
                let data3 = buffer.canvas().data_mut();
                let data3 = data3.as_rgba_mut();
                for (c1, c2) in data1.as_rgba().iter().zip(data2.as_rgba()) {
                    let c1 = premultiply_alpha(*c1);
                    let c2 = premultiply_alpha(*c2);

                    let a = calc(c1.a, c2.a, 1.0);
                    if a.is_fuzzy_zero() {
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

        let canvas = buffer.canvas();
        canvas.draw_surface(&mut mut_input2, 0.0, 0.0, 255,
                            skia::BlendMode::SrcOver,
                            skia::FilterQuality::Low);
        let blend_mode = match fe.operator {
            Operator::Over => skia::BlendMode::SrcOver,
            Operator::In => skia::BlendMode::SrcIn,
            Operator::Out => skia::BlendMode::SrcOut,
            Operator::Atop => skia::BlendMode::SrcATop,
            Operator::Xor => skia::BlendMode::Xor,
            Operator::Arithmetic { .. } => skia::BlendMode::SrcOver,
        };
        canvas.draw_surface(&mut mut_input1, 0.0, 0.0, 255, blend_mode,
                            skia::FilterQuality::Low);

        Ok(Image::from_image(buffer, cs))
    }

    fn apply_merge(
        fe: &usvg::FeMerge,
        cs: ColorSpace,
        region: ScreenRect,
        results: &[FilterResult],
        surface: &Surface,
    ) -> Result<Image, Error> {
        let mut buffer = create_surface(region.width(), region.height())?;
        let canvas = buffer.canvas();
        canvas.reset_matrix();

        for input in &fe.inputs {
            let input = Self::get_input(input, region, &results, surface)?;
            let input = input.into_color_space(cs)?;
            let mut mut_input = input.as_ref().clone();
            canvas.draw_surface(&mut mut_input, 0.0, 0.0, 255,
                                skia::BlendMode::SrcOver,
                                skia::FilterQuality::Low);
        }
        canvas.flush();

        Ok(Image::from_image(buffer, cs))
    }

    fn apply_flood(
        fe: &usvg::FeFlood,
        region: ScreenRect,
    ) -> Result<Image, Error> {
        let c = fe.color;
        let alpha = f64_bound(0.0, fe.opacity.value() * 255.0, 255.0) as u8;

        let mut buffer = create_surface(region.width(), region.height())?;
        buffer.canvas().fill(c.red, c.green, c.blue, alpha);

        Ok(Image::from_image(buffer, ColorSpace::SRGB))
    }

    fn apply_tile(
        input: Image,
        region: ScreenRect,
    ) -> Result<Image, Error> {
        let mut buffer = create_surface(region.width(), region.height())?;

        let subregion = input.region.translate(-region.x(), -region.y());

        let mut tile_surface = copy_surface(&input.image, subregion)?;
        let brush_ts = usvg::Transform::new_translate(subregion.x() as f64, subregion.y() as f64);

        let image = tile_surface.image_snapshot();
        let shader = image.to_shader(Some((TileMode::Repeat, TileMode::Repeat)), Some(&brush_ts.to_native()));

        let mut paint = skia::Paint::default();
        paint.set_shader(shader);

        let canvas = buffer.canvas();
        canvas.draw_rect_from_top_left(0.0, 0.0, region.width() as f32, region.height() as f32, &paint);

        buffer.canvas().reset_matrix();
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
                let mut canvas = buffer.canvas();

                let dx = (subregion.x() - region.x()) as f32;
                let dy = (subregion.y() - region.y()) as f32;
                canvas.translate(Point::new(dx, dy));

                let view_box = usvg::ViewBox {
                    rect: subregion.translate_to(0, 0).to_rect(),
                    aspect: fe.aspect,
                };

                if format == usvg::ImageFormat::SVG {
                    super::image::draw_svg(data, view_box, opt, &mut canvas);
                } else {
                    super::image::draw_raster(
                        format, data, view_box, fe.rendering_mode, opt, &mut buffer.canvas(),
                    );
                }
            }
            usvg::FeImageKind::Use(..) => {}
        }

        buffer.canvas().reset_matrix();
        Ok(Image::from_image(buffer, ColorSpace::SRGB))
    }

    fn apply_to_canvas(
        input: Image,
        region: ScreenRect,
        surface: &mut Surface,
    ) -> Result<(), Error> {
        let input = input.into_color_space(ColorSpace::SRGB)?;

        let mut mut_input = input.as_ref().clone();

        let canvas = surface.canvas();
        canvas.reset_matrix();
        canvas.just_clear();
        canvas.draw_surface(
            &mut mut_input,
            region.x() as f64,
            region.y() as f64,
            255,
            skia::BlendMode::SrcOver,
            skia::FilterQuality::Low
        );

        Ok(())
    }
}
