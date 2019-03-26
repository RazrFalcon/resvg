// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::cmp;
use std::rc::Rc;

// external
use qt;
use rgb::FromSlice;
use usvg::ColorInterpolation as ColorSpace;

// self
use super::prelude::*;
use backend_utils::filter::{
    self,
    Error,
    Filter,
    ImageExt,
};

type Image = filter::Image<qt::Image>;
type FilterResult = filter::FilterResult<qt::Image>;


pub fn apply(
    filter: &usvg::Filter,
    bbox: Rect,
    ts: &usvg::Transform,
    opt: &Options,
    canvas: &mut qt::Image,
) {
    QtFilter::apply(filter, bbox, ts, opt, canvas);
}


impl ImageExt for qt::Image {
    fn width(&self) -> u32 {
        self.width()
    }

    fn height(&self) -> u32 {
        self.height()
    }

    fn try_clone(&self) -> Result<Self, Error> {
        self.try_clone().ok_or(Error::AllocFailed)
    }

    fn clip(&mut self, region: ScreenRect) {
        let mut brush = qt::Brush::new();
        brush.set_color(0, 0, 0, 0);

        let mut p = qt::Painter::new(self);
        p.set_composition_mode(qt::CompositionMode::Clear);
        p.reset_pen();
        p.set_brush(brush);
        p.draw_rect(0.0, 0.0, self.width() as f64, region.y as f64);
        p.draw_rect(0.0, 0.0, region.x as f64, self.height() as f64);
        p.draw_rect(region.right() as f64, 0.0, self.width() as f64, self.height() as f64);
        p.draw_rect(0.0, region.bottom() as f64, self.width() as f64, self.height() as f64);
    }

    fn clear(&mut self) {
        self.fill(0, 0, 0, 0);
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

fn create_image(width: u32, height: u32) -> Result<qt::Image, Error> {
    let mut image = qt::Image::new_rgba(width, height).ok_or(Error::AllocFailed)?;
    image.fill(0, 0, 0, 0);
    Ok(image)
}

fn copy_image(image: &qt::Image, region: ScreenRect) -> Result<qt::Image, Error> {
    let x = cmp::max(0, region.x) as u32;
    let y = cmp::max(0, region.y) as u32;

    image.copy(x, y, region.width, region.height).ok_or(Error::AllocFailed)
}

struct QtFilter;

impl Filter<qt::Image> for QtFilter {
    fn get_input(
        input: &usvg::FilterInput,
        region: ScreenRect,
        results: &[FilterResult],
        canvas: &qt::Image,
    ) -> Result<Image, Error> {
        match input {
            usvg::FilterInput::SourceGraphic => {
                let image = copy_image(canvas, region)?;
                let image = image.to_rgba().ok_or(Error::AllocFailed)?; // TODO: optional

                Ok(Image {
                    image: Rc::new(image),
                    region: ScreenRect::new(0, 0, region.width, region.height),
                    color_space: ColorSpace::SRGB,
                })
            }
            usvg::FilterInput::SourceAlpha => {
                let image = copy_image(canvas, region)?;
                let mut image = image.to_rgba().ok_or(Error::AllocFailed)?;

                // Set RGB to black. Keep alpha as is.
                for p in image.data_mut().chunks_mut(4) {
                    p[0] = 0;
                    p[1] = 0;
                    p[2] = 0;
                }

                Ok(Image {
                    image: Rc::new(image),
                    region: ScreenRect::new(0, 0, region.width, region.height),
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
        let (std_dx, std_dy) = try_opt!(Self::resolve_std_dev(fe, units, bbox, ts), Ok(input));

        let input = input.into_color_space(cs)?;
        let mut buffer = input.take()?;

        let (w, h) = (buffer.width(), buffer.height());
        filter::blur::apply(&mut buffer.data_mut(), w, h, std_dx, std_dy, 4);

        Ok(Image::from_image(buffer, cs))
    }

    fn apply_offset(
        fe: &usvg::FeOffset,
        units: usvg::Units,
        bbox: Rect,
        ts: &usvg::Transform,
        input: Image,
    ) -> Result<Image, Error> {
        let (dx, dy) = try_opt!(Self::resolve_offset(fe, units, bbox, ts), Ok(input));

        // TODO: do not use an additional buffer
        let mut buffer = create_image(input.width(), input.height())?;

        let mut p = qt::Painter::new(&mut buffer);
        // TODO: fractional doesn't work
        p.draw_image(dx, dy, input.as_ref());

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

        let mut buffer = create_image(region.width, region.height)?;
        let mut p = qt::Painter::new(&mut buffer);

        p.draw_image(0.0, 0.0, input2.as_ref());

        let qt_mode = match fe.mode {
            usvg::FeBlendMode::Normal => qt::CompositionMode::SourceOver,
            usvg::FeBlendMode::Multiply => qt::CompositionMode::Multiply,
            usvg::FeBlendMode::Screen => qt::CompositionMode::Screen,
            usvg::FeBlendMode::Darken => qt::CompositionMode::Darken,
            usvg::FeBlendMode::Lighten => qt::CompositionMode::Lighten,
        };
        p.set_composition_mode(qt_mode);
        p.draw_image(0.0, 0.0, input1.as_ref());

        Ok(Image::from_image(buffer, cs))
    }

    fn apply_composite(
        fe: &usvg::FeComposite,
        cs: ColorSpace,
        region: ScreenRect,
        input1: Image,
        input2: Image,
    ) -> Result<Image, Error> {
        let input1 = input1.into_color_space(cs)?;
        let input2 = input2.into_color_space(cs)?;

        let mut buffer = create_image(region.width, region.height)?;

        if fe.operator == Operator::Arithmetic {
            warn!("feComposite with 'arithmetic' operator is not supported.");
            return Ok(Image::from_image(buffer, cs));
        };

        let mut p = qt::Painter::new(&mut buffer);

        p.draw_image(0.0, 0.0, input2.as_ref());

        use usvg::FeCompositeOperator as Operator;
        let qt_mode = match fe.operator {
            Operator::Over => qt::CompositionMode::SourceOver,
            Operator::In => qt::CompositionMode::SourceIn,
            Operator::Out => qt::CompositionMode::SourceOut,
            Operator::Atop => qt::CompositionMode::SourceAtop,
            Operator::Xor => qt::CompositionMode::Xor,
            Operator::Arithmetic => qt::CompositionMode::SourceOver,
        };
        p.set_composition_mode(qt_mode);
        p.draw_image(0.0, 0.0, input1.as_ref());

        Ok(Image::from_image(buffer, cs))
    }

    fn apply_merge(
        fe: &usvg::FeMerge,
        cs: ColorSpace,
        region: ScreenRect,
        results: &[FilterResult],
        canvas: &qt::Image,
    ) -> Result<Image, Error> {
        let mut buffer = create_image(region.width, region.height)?;
        let mut p = qt::Painter::new(&mut buffer);

        for input in &fe.inputs {
            let input = Self::get_input(input, region, &results, canvas)?;
            let input = input.into_color_space(cs)?;

            p.draw_image(0.0, 0.0, input.as_ref());
        }

        Ok(Image::from_image(buffer, cs))
    }

    fn apply_flood(
        fe: &usvg::FeFlood,
        region: ScreenRect,
    ) -> Result<Image, Error> {
        let c = fe.color;
        let alpha = f64_bound(0.0, fe.opacity.value() * 255.0, 255.0) as u8;

        let mut buffer = create_image(region.width, region.height)?;
        buffer.fill(c.red, c.green, c.blue, alpha);

        Ok(Image::from_image(buffer, ColorSpace::SRGB))
    }

    fn apply_tile(
        input: Image,
        region: ScreenRect,
    ) -> Result<Image, Error> {
        let mut buffer = create_image(region.width, region.height)?;

        let mut subregion = input.region;
        subregion.x -= region.x;
        subregion.y -= region.y;

        let mut brush = qt::Brush::new();
        brush.set_pattern(copy_image(&input.image, subregion)?);
        let brush_ts = usvg::Transform::new_translate(subregion.x as f64, subregion.y as f64);
        brush.set_transform(brush_ts.to_native());

        let mut p = qt::Painter::new(&mut buffer);
        p.reset_pen();
        p.set_brush(brush);
        p.draw_rect(0.0, 0.0, region.width as f64, region.height as f64);

        Ok(Image::from_image(buffer, ColorSpace::SRGB))
    }

    fn apply_image(
        fe: &usvg::FeImage,
        region: ScreenRect,
        subregion: ScreenRect,
        opt: &Options,
    ) -> Result<Image, Error> {
        let mut buffer = create_image(region.width, region.height)?;

        match fe.data {
            usvg::FeImageKind::None => {}
            usvg::FeImageKind::Image(ref data, format) => {
                let mut p = qt::Painter::new(&mut buffer);

                let dx = (subregion.x - region.x) as f64;
                let dy = (subregion.y - region.y) as f64;
                p.translate(dx, dy);

                let view_box = usvg::ViewBox {
                    rect: ScreenRect::new(0, 0, subregion.width, subregion.height).to_rect(),
                    aspect: fe.aspect,
                };

                if format == usvg::ImageFormat::SVG {
                    super::image::draw_svg(data, view_box, opt, &mut p);
                } else {
                    super::image::draw_raster(data, view_box, fe.rendering_mode, opt, &mut p);
                }
            }
            usvg::FeImageKind::Use(..) => {}
        }

        Ok(Image::from_image(buffer, ColorSpace::SRGB))
    }

    fn apply_to_canvas(
        input: Image,
        region: ScreenRect,
        canvas: &mut qt::Image,
    ) -> Result<(), Error> {
        let input = input.into_color_space(ColorSpace::SRGB)?;

        // Clear.
        canvas.fill(0, 0, 0, 0);

        let mut p = qt::Painter::new(canvas);
        p.draw_image(region.x as f64, region.y as f64, input.as_ref());

        Ok(())
    }
}
