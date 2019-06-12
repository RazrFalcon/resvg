// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::cmp;
use std::rc::Rc;

use rgb::FromSlice;
use log::warn;

use usvg::{try_opt_or, ColorInterpolation as ColorSpace};

use crate::{prelude::*, backend_utils::*};
use crate::backend_utils::filter::{Error, Filter, ImageExt};
use super::ColorExt;

type Image = filter::Image<raqote::DrawTarget>;
type FilterResult = filter::FilterResult<raqote::DrawTarget>;


pub fn apply(
    filter: &usvg::Filter,
    bbox: Rect,
    ts: &usvg::Transform,
    opt: &Options,
    canvas: &mut raqote::DrawTarget,
) {
    RaqoteFilter::apply(filter, bbox, ts, opt, canvas);
}


impl ImageExt for raqote::DrawTarget {
    fn width(&self) -> u32 {
        self.width() as u32
    }

    fn height(&self) -> u32 {
        self.height() as u32
    }

    fn try_clone(&self) -> Result<Self, Error> {
        let mut dt = raqote::DrawTarget::new(self.width(), self.height());

        let src_img = raqote::Image {
            width: self.width() as i32,
            height: self.height() as i32,
            data: self.get_data(),
        };
        dt.draw_image_at(0.0, 0.0, &src_img, &raqote::DrawOptions {
            blend_mode: raqote::BlendMode::Src,
            alpha: 1.,
        });

        Ok(dt)
    }

    fn clip(&mut self, region: ScreenRect) {
        let mut pb = raqote::PathBuilder::new();
        pb.rect(0.0, 0.0, self.width() as f32, region.y() as f32);
        pb.rect(0.0, 0.0, region.x() as f32, self.height() as f32);
        pb.rect(region.right() as f32, 0.0, self.width() as f32, self.height() as f32);
        pb.rect(0.0, region.bottom() as f32, self.width() as f32, self.height() as f32);

        self.fill(&pb.finish(), &raqote::Source::Solid(raqote::SolidSource {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }), &raqote::DrawOptions {
            blend_mode: raqote::BlendMode::Clear,
            ..Default::default()
        });
    }

    fn clear(&mut self) {
        self.clear(raqote::SolidSource {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        });
    }

    fn into_srgb(&mut self) {
        let data =  self.get_data_u8_mut();
        from_premultiplied(data);

        for p in data.as_bgra_mut() {
            p.r = filter::LINEAR_RGB_TO_SRGB_TABLE[p.r as usize];
            p.g = filter::LINEAR_RGB_TO_SRGB_TABLE[p.g as usize];
            p.b = filter::LINEAR_RGB_TO_SRGB_TABLE[p.b as usize];
        }

        into_premultiplied(data);
    }

    fn into_linear_rgb(&mut self) {
        let data =  self.get_data_u8_mut();
        from_premultiplied(data);

        for p in data.as_bgra_mut() {
            p.r = filter::SRGB_TO_LINEAR_RGB_TABLE[p.r as usize];
            p.g = filter::SRGB_TO_LINEAR_RGB_TABLE[p.g as usize];
            p.b = filter::SRGB_TO_LINEAR_RGB_TABLE[p.b as usize];
        }

        into_premultiplied(data);
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

    let src_img = raqote::Image {
        width: image.width() as i32,
        height: image.height() as i32,
        data: image.get_data(),
    };
    new_image.draw_image_at(-x, -y, &src_img, &raqote::DrawOptions {
        blend_mode: raqote::BlendMode::Src,
        alpha: 1.,
    });

    Ok(new_image)
}

fn from_premultiplied(data: &mut [u8]) {
    // https://www.cairographics.org/manual/cairo-Image-Surfaces.html#cairo-format-t

    for p in data.as_bgra_mut() {
        let a = p.a as f64 / 255.0;
        p.b = (p.b as f64 / a + 0.5) as u8;
        p.g = (p.g as f64 / a + 0.5) as u8;
        p.r = (p.r as f64 / a + 0.5) as u8;
    }
}

fn into_premultiplied(data: &mut [u8]) {
    // https://www.cairographics.org/manual/cairo-Image-Surfaces.html#cairo-format-t

    for p in data.as_bgra_mut() {
        let a = p.a as f64 / 255.0;
        p.b = (p.b as f64 * a + 0.5) as u8;
        p.g = (p.g as f64 * a + 0.5) as u8;
        p.r = (p.r as f64 * a + 0.5) as u8;
    }
}

struct RaqoteFilter;

impl Filter<raqote::DrawTarget> for RaqoteFilter {
    fn get_input(
        input: &usvg::FilterInput,
        region: ScreenRect,
        results: &[FilterResult],
        canvas: &raqote::DrawTarget,
    ) -> Result<Image, Error> {
        match input {
            usvg::FilterInput::SourceGraphic => {
                let image = copy_image(canvas, region)?;

                Ok(Image {
                    image: Rc::new(image),
                    region: region.translate_to(0, 0),
                    color_space: ColorSpace::SRGB,
                })
            }
            usvg::FilterInput::SourceAlpha => {
                let mut image = copy_image(canvas, region)?;

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
            }
            usvg::FilterInput::Reference(ref name) => {
                if let Some(ref v) = results.iter().rev().find(|v| v.name == *name) {
                    Ok(v.image.clone())
                } else {
                    // Technically unreachable.
                    warn!("Unknown filter primitive reference '{}'.", name);
                    Self::get_input(&usvg::FilterInput::SourceGraphic, region, results, canvas)
                }
            }
            _ => {
                warn!("Filter input '{}' is not supported.", input.to_string());
                Self::get_input(&usvg::FilterInput::SourceGraphic, region, results, canvas)
            }
        }
    }

    fn apply_blur(
        fe: &usvg::FeGaussianBlur,
        units: usvg::Units,
        cs: ColorSpace,
        bbox: Rect,
        ts: &usvg::Transform,
        input: Image,
    ) -> Result<Image, Error> {
        let (std_dx, std_dy) = try_opt_or!(Self::resolve_std_dev(fe, units, bbox, ts), Ok(input));

        let input = input.into_color_space(cs)?;
        let mut buffer = input.take()?;

        let (w, h) = (buffer.width() as u32, buffer.height() as u32);

        let data = buffer.get_data_u8_mut();
        from_premultiplied(data);
        filter::blur::apply(data, w, h, std_dx, std_dy, 4);
        into_premultiplied(data);

        Ok(Image::from_image(buffer, cs))
    }

    fn apply_offset(
        fe: &usvg::FeOffset,
        units: usvg::Units,
        bbox: Rect,
        ts: &usvg::Transform,
        input: Image,
    ) -> Result<Image, Error> {
        let (dx, dy) = try_opt_or!(Self::resolve_offset(fe, units, bbox, ts), Ok(input));

        // TODO: do not use an additional buffer
        let mut dt = create_image(input.width(), input.height())?;

        let src_img = raqote::Image {
            width: input.width() as i32,
            height: input.height() as i32,
            data: input.as_ref().get_data(),
        };
        dt.draw_image_at(dx as f32, dy as f32, &src_img, &raqote::DrawOptions::default());

        Ok(Image::from_image(dt, input.color_space))
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

        let mut dt = create_image(region.width(), region.height())?;
        let src_img = raqote::Image {
            width: input2.width() as i32,
            height: input2.height() as i32,
            data: input2.as_ref().get_data(),
        };
        dt.draw_image_at(0.0, 0.0, &src_img, &raqote::DrawOptions {
            blend_mode: raqote::BlendMode::Src,
            alpha: 1.,
        });

        let blend_mode = match fe.mode {
            usvg::FeBlendMode::Normal => raqote::BlendMode::SrcOver,
            usvg::FeBlendMode::Multiply => raqote::BlendMode::Multiply,
            usvg::FeBlendMode::Screen => raqote::BlendMode::Screen,
            usvg::FeBlendMode::Darken => raqote::BlendMode::Darken,
            usvg::FeBlendMode::Lighten => raqote::BlendMode::Lighten,
        };

        let src_img = raqote::Image {
            width: input1.width() as i32,
            height: input1.height() as i32,
            data: input1.as_ref().get_data(),
        };
        dt.draw_image_at(0.0, 0.0, &src_img, &raqote::DrawOptions {
            blend_mode,
            alpha: 1.,
        });

        Ok(Image::from_image(dt, cs))
    }

    fn apply_composite(
        fe: &usvg::FeComposite,
        cs: ColorSpace,
        region: ScreenRect,
        input1: Image,
        input2: Image,
    ) -> Result<Image, Error> {
        use rgb::alt::BGRA8;

        let mut input1 = input1.into_color_space(cs)?.take()?;
        let mut input2 = input2.into_color_space(cs)?.take()?;

        let mut dt = create_image(region.width(), region.height())?;

        if let Operator::Arithmetic { k1, k2, k3, k4 } = fe.operator {
            let data1 = input1.get_data_u8_mut();
            let data2 = input2.get_data_u8_mut();

            let calc = |i1, i2, max| {
                let i1 = i1 as f64 / 255.0;
                let i2 = i2 as f64 / 255.0;
                let result = k1.value() * i1 * i2 + k2.value() * i1 + k3.value() * i2 + k4.value();
                f64_bound(0.0, result, max)
            };

            {
                let mut i = 0;
                let data3 = dt.get_data_u8_mut();
                let data3 = data3.as_bgra_mut();
                for (c1, c2) in data1.as_bgra().iter().zip(data2.as_bgra()) {
                    let a = calc(c1.a, c2.a, 1.0);
                    if a.is_fuzzy_zero() {
                        continue;
                    }

                    let r = (calc(c1.r, c2.r, a) * 255.0) as u8;
                    let g = (calc(c1.g, c2.g, a) * 255.0) as u8;
                    let b = (calc(c1.b, c2.b, a) * 255.0) as u8;
                    let a = (a * 255.0) as u8;

                    data3[i] = BGRA8 { r, g, b, a };

                    i += 1;
                }
            }

            return Ok(Image::from_image(dt, cs));
        }

        let src_img = raqote::Image {
            width: input2.width() as i32,
            height: input2.height() as i32,
            data: input2.get_data(),
        };
        dt.draw_image_at(0.0, 0.0, &src_img, &raqote::DrawOptions {
            blend_mode: raqote::BlendMode::Src,
            alpha: 1.,
        });

        use usvg::FeCompositeOperator as Operator;
        let blend_mode = match fe.operator {
            Operator::Over => raqote::BlendMode::SrcOver,
            Operator::In => raqote::BlendMode::SrcIn,
            Operator::Out => raqote::BlendMode::SrcOut,
            Operator::Atop => raqote::BlendMode::SrcAtop,
            Operator::Xor => raqote::BlendMode::Xor,
            Operator::Arithmetic { .. } => raqote::BlendMode::SrcOver,
        };

        let src_img = raqote::Image {
            width: input1.width() as i32,
            height: input1.height() as i32,
            data: input1.get_data(),
        };
        dt.draw_image_at(0.0, 0.0, &src_img, &raqote::DrawOptions {
            blend_mode,
            alpha: 1.,
        });

        Ok(Image::from_image(dt, cs))
    }

    fn apply_merge(
        fe: &usvg::FeMerge,
        cs: ColorSpace,
        region: ScreenRect,
        results: &[FilterResult],
        canvas: &raqote::DrawTarget,
    ) -> Result<Image, Error> {
        let mut dt = create_image(region.width(), region.height())?;

        for input in &fe.inputs {
            let input = Self::get_input(input, region, &results, canvas)?;
            let input = input.into_color_space(cs)?;

            let src_img = raqote::Image {
                width: input.width() as i32,
                height: input.height() as i32,
                data: input.as_ref().get_data(),
            };
            dt.draw_image_at(0.0, 0.0, &src_img, &raqote::DrawOptions::default());
        }

        Ok(Image::from_image(dt, cs))
    }

    fn apply_flood(
        fe: &usvg::FeFlood,
        region: ScreenRect,
    ) -> Result<Image, Error> {
        let mut dt = create_image(region.width(), region.height())?;

        let alpha = (fe.opacity.value() * 255.0) as u8;
        dt.clear(fe.color.to_solid(alpha));

        Ok(Image::from_image(dt, ColorSpace::SRGB))
    }

    fn apply_tile(
        input: Image,
        region: ScreenRect,
    ) -> Result<Image, Error> {
        let mut dt = create_image(region.width(), region.height())?;

        let subregion = input.region.translate(-region.x(), -region.y());

        let tile = copy_image(&input.image, subregion)?;
        let brush_ts = usvg::Transform::new_translate(subregion.x() as f64, subregion.y() as f64);

        let img = raqote::Image {
            width: tile.width() as i32,
            height: tile.height() as i32,
            data: tile.get_data(),
        };
        let t: raqote::Transform = brush_ts.to_native();
        let t = t.inverse().unwrap();
        let patt = raqote::Source::Image(img, raqote::ExtendMode::Repeat, t);

        let mut pb = raqote::PathBuilder::new();
        pb.rect(0.0, 0.0, region.width() as f32, region.height() as f32);
        dt.fill(&pb.finish(), &patt, &raqote::DrawOptions::default());

        Ok(Image::from_image(dt, ColorSpace::SRGB))
    }

    fn apply_image(
        fe: &usvg::FeImage,
        region: ScreenRect,
        subregion: ScreenRect,
        opt: &Options,
    ) -> Result<Image, Error> {
        let mut dt = create_image(region.width(), region.height())?;

        match fe.data {
            usvg::FeImageKind::None => {}
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
                    super::image::draw_raster(data, view_box, fe.rendering_mode, opt, &mut dt);
                }
            }
            usvg::FeImageKind::Use(..) => {}
        }

        Ok(Image::from_image(dt, ColorSpace::SRGB))
    }

    fn apply_to_canvas(
        input: Image,
        region: ScreenRect,
        canvas: &mut raqote::DrawTarget,
    ) -> Result<(), Error> {
        let input = input.into_color_space(ColorSpace::SRGB)?;

        canvas.set_transform(&raqote::Transform::identity());
        canvas.clear(raqote::SolidSource {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        });

        let src_img = raqote::Image {
            width: input.width() as i32,
            height: input.height() as i32,
            data: input.as_ref().get_data(),
        };
        canvas.draw_image_at(region.x() as f32, region.y() as f32, &src_img, &raqote::DrawOptions {
            blend_mode: raqote::BlendMode::SrcOver,
            alpha: 1.,
        });

        Ok(())
    }
}
