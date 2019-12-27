// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::cmp;
use std::rc::Rc;

use rgb::FromSlice;
use log::warn;

use usvg::ColorInterpolation as ColorSpace;

use crate::prelude::*;
use crate::filter::{self, Error, Filter, ImageExt, TransferFunctionExt};
use crate::ConvTransform;
use super::{ColorExt, RaqoteDrawTargetExt};

type Image = filter::Image<raqote::DrawTarget>;
type FilterInputs<'a> = filter::FilterInputs<'a, raqote::DrawTarget>;
type FilterResult = filter::FilterResult<raqote::DrawTarget>;


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
    RaqoteFilter::apply(filter, bbox, ts, opt, tree, background, fill_paint, stroke_paint, canvas);
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
        dt.draw_image_at(0.0, 0.0, &self.as_image(), &raqote::DrawOptions {
            blend_mode: raqote::BlendMode::Src,
            ..raqote::DrawOptions::default()
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
        self.make_transparent();
    }

    fn into_srgb(&mut self) {
        let data =  self.get_data_u8_mut();
        filter::from_premultiplied(data.as_bgra_mut());

        for p in data.as_bgra_mut() {
            p.r = filter::LINEAR_RGB_TO_SRGB_TABLE[p.r as usize];
            p.g = filter::LINEAR_RGB_TO_SRGB_TABLE[p.g as usize];
            p.b = filter::LINEAR_RGB_TO_SRGB_TABLE[p.b as usize];
        }

        filter::into_premultiplied(data.as_bgra_mut());
    }

    fn into_linear_rgb(&mut self) {
        let data =  self.get_data_u8_mut();
        filter::from_premultiplied(data.as_bgra_mut());

        for p in data.as_bgra_mut() {
            p.r = filter::SRGB_TO_LINEAR_RGB_TABLE[p.r as usize];
            p.g = filter::SRGB_TO_LINEAR_RGB_TABLE[p.g as usize];
            p.b = filter::SRGB_TO_LINEAR_RGB_TABLE[p.b as usize];
        }

        filter::into_premultiplied(data.as_bgra_mut());
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

struct RaqoteFilter;

impl Filter<raqote::DrawTarget> for RaqoteFilter {
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

        let input = input.into_color_space(cs)?;
        let mut buffer = input.take()?;

        let (w, h) = (buffer.width() as u32, buffer.height() as u32);

        let data = buffer.get_data_u8_mut();
        if box_blur {
            filter::box_blur::apply(data, w, h, std_dx, std_dy);
        } else {
            filter::iir_blur::apply(data, w, h, std_dx, std_dy);
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
        let mut dt = create_image(input.width(), input.height())?;
        dt.draw_image_at(
            dx as f32, dy as f32, &input.as_ref().as_image(), &raqote::DrawOptions::default(),
        );

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
                        i += 1;
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

        let draw_opt = raqote::DrawOptions {
            blend_mode: raqote::BlendMode::Src,
            ..raqote::DrawOptions::default()
        };
        dt.draw_image_at(0.0, 0.0, &input2.as_image(), &draw_opt);

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
        dt.draw_image_at(0.0, 0.0, &input1.as_image(), &draw_opt);

        Ok(Image::from_image(dt, cs))
    }

    fn apply_merge(
        fe: &usvg::FeMerge,
        cs: ColorSpace,
        region: ScreenRect,
        inputs: &FilterInputs,
        results: &[FilterResult],
    ) -> Result<Image, Error> {
        let mut dt = create_image(region.width(), region.height())?;

        for input in &fe.inputs {
            let input = Self::get_input(input, region, inputs, results)?;
            let input = input.into_color_space(cs)?;
            dt.draw_image_at(0.0, 0.0, &input.as_ref().as_image(), &raqote::DrawOptions::default());
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
                    super::image::draw_raster(
                        format, data, view_box, fe.rendering_mode, opt, &mut dt
                    );
                }
            }
            usvg::FeImageKind::Use(ref id) => {
                if let Some(ref node) = tree.defs_by_id(id).or(tree.node_by_id(id)) {
                    let mut layers = super::create_layers(region.size());
                    dt.transform(&ts.to_native());
                    dt.transform(&node.transform().to_native());
                    super::render_node(node, opt, &mut crate::RenderState::Ok, &mut layers, &mut dt);
                }
            }
        }

        dt.set_transform(&raqote::Transform::default());
        Ok(Image::from_image(dt, ColorSpace::SRGB))
    }

    fn apply_component_transfer(
        fe: &usvg::FeComponentTransfer,
        cs: ColorSpace,
        input: Image,
    ) -> Result<Image, Error> {
        let input = input.into_color_space(cs)?;
        let mut buffer = input.take()?;

        let data = buffer.get_data_u8_mut();
        filter::from_premultiplied(data.as_bgra_mut());

        for pixel in data.as_bgra_mut() {
            pixel.r = fe.func_r.apply(pixel.r);
            pixel.g = fe.func_g.apply(pixel.g);
            pixel.b = fe.func_b.apply(pixel.b);
            pixel.a = fe.func_a.apply(pixel.a);
        }

        filter::into_premultiplied(data.as_bgra_mut());

        Ok(Image::from_image(buffer, cs))
    }

    fn apply_color_matrix(
        fe: &usvg::FeColorMatrix,
        cs: ColorSpace,
        input: Image,
    ) -> Result<Image, Error> {
        let input = input.into_color_space(cs)?;
        let mut buffer = input.take()?;

        let data = buffer.get_data_u8_mut();
        filter::from_premultiplied(data.as_bgra_mut());
        filter::color_matrix::apply(&fe.kind, data.as_bgra_mut());
        filter::into_premultiplied(data.as_bgra_mut());

        Ok(Image::from_image(buffer, cs))
    }

    fn apply_convolve_matrix(
        fe: &usvg::FeConvolveMatrix,
        cs: ColorSpace,
        input: Image,
    ) -> Result<Image, Error> {
        let input = input.into_color_space(cs)?;
        let mut buffer = input.take()?;
        let w = buffer.width() as u32;
        let h = buffer.height() as u32;

        let data = buffer.get_data_u8_mut();
        if fe.preserve_alpha {
            filter::from_premultiplied(data.as_bgra_mut());
        }

        filter::convolve_matrix::apply(fe, w, h, data.as_bgra_mut());

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
        let input = input.into_color_space(cs)?;
        let (rx, ry) = try_opt_or!(
            Self::scale_coordinates(fe.radius_x.value(), fe.radius_y.value(), units, bbox, ts),
            Ok(input)
        );

        let mut buffer = input.take()?;
        let w = buffer.width() as u32;
        let h = buffer.height() as u32;
        let data = buffer.get_data_u8_mut();
        filter::morphology::apply(fe.operator, rx, ry, w, h, data.as_bgra_mut());

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
        let input1 = input1.into_color_space(cs)?;
        let input2 = input2.into_color_space(cs)?;
        let (sx, sy) = try_opt_or!(
            Self::scale_coordinates(fe.scale, fe.scale, units, bbox, ts),
            Ok(input1)
        );

        let mut buffer1 = input1.take()?;
        let mut buffer2 = input2.take()?;
        let mut buffer = create_image(region.width(), region.height())?;

        filter::displacement_map::apply(
            fe.x_channel_selector, fe.y_channel_selector,
            region.width(), region.height(),
            sx, sy,
            buffer1.get_data_u8_mut().as_bgra(),
            buffer2.get_data_u8_mut().as_bgra(),
            buffer.get_data_u8_mut().as_bgra_mut(),
        );

        Ok(Image::from_image(buffer, cs))
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

        canvas.copy_surface(image,
                            raqote::IntRect::new(raqote::IntPoint::new(0, 0),
                                                 raqote::IntPoint::new(image.width(), image.height())),
                            raqote::IntPoint::new(region.x(), region.y()));

        Ok(())
    }
}
