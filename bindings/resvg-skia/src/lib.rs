use std::ops::{Deref, DerefMut};
use std::io::Write;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PaintStyle {
    Fill = 0,
    Stroke = 1,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum FillType {
    Winding = 0,
    EvenOdd = 1,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum StrokeCap {
    Butt = 0,
    Round = 1,
    Square = 2,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum StrokeJoin {
    Miter = 0,
    Round = 1,
    Bevel = 2,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum TileMode {
    Clamp = 0,
    Repeat = 1,
    Mirror = 2,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum BlendMode {
    Clear = 0,
    SourceOver = 1,
    DestinationOver = 2,
    SourceIn = 3,
    DestinationIn = 4,
    SourceOut = 5,
    DestinationOut = 6,
    SourceAtop = 7,
    Xor = 8,
    Multiply = 9,
    Screen = 10,
    Darken = 11,
    Lighten = 12,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum FilterQuality {
    None = 0,
    Low = 1,
    Medium = 2,
    High = 3,
}

pub struct Surface {
    surface: skia_safe::Surface,
    canvas: Canvas,
}

impl Surface {
    pub fn new_rgba(width: u32, height: u32) -> Option<Surface> {
        Surface::new_rgba_impl(width, height, skia_safe::AlphaType::Unpremul)
    }

    pub fn new_rgba_premultiplied(width: u32, height: u32) -> Option<Surface> {
        Surface::new_rgba_impl(width, height, skia_safe::AlphaType::Premul)
    }
    
    pub fn from_skia_safe_canvas(canvas: &mut skia_safe::Canvas) -> Option<Surface> {
        let surface = unsafe { canvas.surface() }?;
        let canvas = Canvas(surface.clone());
        Some(Surface { surface, canvas })
    }

    pub fn destroy(&mut self) {
        unimplemented!();
    }

    pub fn copy_rgba(&self, x: u32, y: u32, width: u32, height: u32) -> Option<Surface> {
        let mut copy = Surface::new_rgba(width, height)?;
        let sampling = skia_safe::SamplingOptions::from(skia_safe::FilterQuality::Low);
        self.surface.clone().draw(copy.surface.canvas(), (-(x as f32), -(y as f32)), sampling, None);
        Some(copy)
    }

    pub fn try_clone(&self) -> Option<Surface> {
        self.copy_rgba(0, 0, self.width(), self.height())
    }

    pub fn save_png(&self, path: &str) -> bool {
        let mut bytes: Vec<u8> = vec![];
        {
            let mut encoder = png::Encoder::new(&mut bytes, self.width(), self.height());
            encoder.set_color(png::ColorType::RGBA);
            encoder.set_depth(png::BitDepth::Eight);

            let mut writer = encoder.write_header().expect("failed to write file header");

            writer.write_image_data(&self.data()).expect("failed to write image data");
        }
        let data = skia_safe::Data::new_copy(&bytes);

        let mut file = std::fs::File::create(path).expect("failed to create the file");
        file.write_all(data.as_bytes()).expect("failed to write data to the file");

        return true;
    }

    pub fn width(&self) -> u32 {
        self.surface.width() as u32
    }

    pub fn height(&self) -> u32 {
        self.surface.height() as u32
    }

    pub fn data(&self) -> SurfaceData {
        unsafe {
            let mut surface = self.surface.clone();
            let pixmap = surface.peek_pixels().unwrap();
            SurfaceData {
                slice: std::slice::from_raw_parts_mut(pixmap.writable_addr().cast(), pixmap.compute_byte_size()),
            }
        }
    }

    pub fn data_mut(&mut self) -> SurfaceData {
        self.data()
    }

    pub fn is_bgra() -> bool {
        skia_safe::ColorType::n32() == skia_safe::ColorType::BGRA8888
    }
}

impl std::ops::Deref for Surface {
    type Target = Canvas;
    fn deref(&self) -> &Self::Target {
        &self.canvas
    }
}

impl std::ops::DerefMut for Surface {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.canvas
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        // Implemented by Skia-safe.
    }
}

pub struct SurfaceData<'a> {
    slice: &'a mut [u8],
}

impl<'a> Deref for SurfaceData<'a> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.slice
    }
}

impl<'a> DerefMut for SurfaceData<'a> {
    fn deref_mut(&mut self) -> &mut [u8] {
        self.slice
    }
}

pub struct Color(u8, u8, u8, u8);

impl Color {
    pub fn new(a: u8, r: u8, g: u8, b: u8) -> Color {
        Color(a, r, g, b)
    }

    pub fn to_u32(&self) -> u32 {
        (self.0 as u32) << 24 | (self.1 as u32) << 16 | (self.2 as u32) << 8 | (self.3 as u32)
    }
}


pub struct Matrix(skia_safe::Matrix);

impl Matrix {
    pub fn new() -> Matrix {
        Matrix(skia_safe::Matrix::default())
    }

    pub fn new_from(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Matrix {
        Matrix(skia_safe::Matrix::new_all(
            a as f32,
            c as f32,
            e as f32,
            b as f32,
            d as f32,
            f as f32,
            0.0,
            0.0,
            1.0,
        ))
    }

    pub fn invert(&self) -> Option<Matrix> {
        self.0.invert().map(|matrix| Matrix(matrix))
    }

    pub fn data(&self) -> (f64, f64, f64, f64, f64, f64) {
        let data = self.0.to_affine().unwrap();
        (data[0] as f64, data[1] as f64, data[2] as f64, data[3] as f64, data[4] as f64, data[5] as f64)
    }
}

impl Default for Matrix {
    fn default() -> Matrix {
        Matrix::new()
    }
}

impl Drop for Matrix {
    fn drop(&mut self) {
        // Implemented by Skia-safe.
    }
}

pub struct Canvas(skia_safe::Surface);

impl Canvas {
    pub fn clear(&mut self) {
        self.0.canvas().clear(skia_safe::Color::default());
    }

    pub fn fill(&mut self, r: u8, g: u8, b: u8, a: u8) {
        let color = skia_safe::Color::from_argb(a, r, g, b);
        self.0.canvas().clear(color);
    }

    pub fn flush(&mut self) {
        self.0.flush_and_submit();
    }

    pub fn set_matrix(&mut self, matrix: &Matrix) {
        self.0.canvas().set_matrix(&matrix.0.into());
    }

    pub fn concat(&mut self, matrix: &Matrix) {
        self.0.canvas().concat(&matrix.0);
    }

    pub fn scale(&mut self, sx: f64, sy: f64) {
        self.0.canvas().scale((sx as f32, sy as f32));
    }

    pub fn translate(&mut self, dx: f64, dy: f64) {
        self.0.canvas().translate((dx as f32, dy as f32));
    }

    pub fn get_matrix(&self) -> Matrix {
        let mut surface = self.0.clone();
        Matrix(surface.canvas().local_to_device_as_3x3())
    }

    pub fn draw_path(&mut self, path: &Path, paint: &Paint) {
        self.0.canvas().draw_path(&path.0, &paint.0);
    }

    pub fn draw_rect(&mut self, x: f64, y: f64, w: f64, h: f64, paint: &Paint) {
        self.0.canvas().draw_rect(skia_safe::Rect::from_xywh(x as f32, y as f32, w as f32, h as f32), &paint.0);
    }

    pub fn draw_surface(&mut self, surface: &Surface, left: f64, top: f64, alpha: u8,
                        blend_mode: BlendMode, filter_quality: FilterQuality) {
        let mut paint = skia_safe::Paint::default();
        paint.set_alpha(alpha);
        paint.set_blend_mode(blend_mode.to_skia());
        self.0.canvas().draw_image_with_sampling_options(
            &surface.image_snapshot(),
            (left as f32, top as f32),
            skia_safe::SamplingOptions::from(filter_quality.to_skia()),
            Some(&paint)
        );
    }

    pub fn draw_surface_rect(&mut self, surface: &Surface, x: f64, y: f64, w: f64, h: f64,
                             filter_quality: FilterQuality) {
        let dst = skia_safe::Rect::from_xywh(x as f32, y as f32, w as f32, h as f32);
        self.0.canvas().draw_image_rect_with_sampling_options(
            &surface.image_snapshot(),
            None,
            dst,
            filter_quality.to_skia(),
            &skia_safe::Paint::default()
        );
    }

    pub fn reset_matrix(&mut self) {
        self.0.canvas().reset_matrix();
    }

    pub fn set_clip_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.0.canvas().clip_rect(skia_safe::Rect::from_xywh(x as f32, y as f32, w as f32, h as f32), None, None);
    }

    pub fn save(&mut self) {
        self.0.canvas().save();
    }

    pub fn restore(&mut self) {
        self.0.canvas().restore();
    }
}

pub struct Paint(skia_safe::Paint);

impl Paint {
    pub fn new() -> Paint {
        Paint(skia_safe::Paint::default())
    }
    pub fn set_style(&mut self, style: PaintStyle) {
        self.0.set_style(style.to_skia());
    }
    pub fn set_color(&mut self, r: u8, g: u8, b: u8, a: u8) {
        self.0.set_argb(a, r, g, b);
    }
    pub fn set_alpha(&mut self, a: u8) {
        self.0.set_alpha(a);
    }
    pub fn set_anti_alias(&mut self, aa: bool) {
        self.0.set_anti_alias(aa);
    }
    pub fn set_blend_mode(&mut self, blend_mode: BlendMode) {
        self.0.set_blend_mode(blend_mode.to_skia());
    }
    pub fn set_shader(&mut self, shader: &Shader) {
        self.0.set_shader(Some(shader.0.clone()));
    }
    pub fn set_stroke_width(&mut self, width: f64) {
        self.0.set_stroke_width(width as f32);
    }
    pub fn set_stroke_cap(&mut self, cap: StrokeCap) {
        self.0.set_stroke_cap(cap.to_skia());
    }
    pub fn set_stroke_join(&mut self, join: StrokeJoin) {
        self.0.set_stroke_join(join.to_skia());
    }
    pub fn set_stroke_miter(&mut self, miter: f64) {
        self.0.set_stroke_miter(miter as f32);
    }
    pub fn set_path_effect(&mut self, path_effect: PathEffect) {
        self.0.set_path_effect(Some(path_effect.0.clone()));
    }
}

impl Drop for Paint {
    fn drop(&mut self) {
        // Implemented by Skia-safe.
    }
}

pub struct Path(skia_safe::Path);

impl Path {
    pub fn new() -> Path {
        Path(skia_safe::Path::new())
    }

    pub fn set_fill_type(&mut self, kind: FillType) {
        self.0.set_fill_type(kind.to_skia());
    }

    pub fn move_to(&mut self, x: f64, y: f64) {
        self.0.move_to((x as f32, y as f32));
    }

    pub fn line_to(&mut self, x: f64, y: f64) {
        self.0.line_to((x as f32, y as f32));
    }

    pub fn cubic_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) {
        self.0.cubic_to((x1 as f32, y1 as f32), (x2 as f32, y2 as f32), (x3 as f32, y3 as f32));
    }

    pub fn close(&mut self) {
        self.0.close();
    }
}

impl Drop for Path {
    fn drop(&mut self) {
        // Implemented by Skia-safe.
    }
}

pub struct Gradient {
    pub colors: Vec<u32>,
    pub positions: Vec<f32>,
    pub tile_mode: TileMode,
    pub matrix: Matrix
}

pub struct LinearGradient {
    pub start_point: (f64, f64),
    pub end_point: (f64, f64),
    pub base: Gradient
}

pub struct RadialGradient {
    pub start_circle: (f64, f64, f64),
    pub end_circle: (f64, f64, f64),
    pub base: Gradient
}

pub struct Shader(skia_safe::Shader);

impl Shader {
    pub fn new_linear_gradient(grad:  LinearGradient) -> Shader {
        let points = ((grad.start_point.0 as f32, grad.start_point.1 as f32), (grad.end_point.0 as f32, grad.end_point.1 as f32));
        let colors_list: Vec<skia_safe::Color> = grad.base.colors.into_iter().map(|color| skia_safe::Color::new(color)).collect();
        let colors = skia_safe::gradient_shader::GradientShaderColors::Colors(&colors_list);
        let positions = Some(grad.base.positions.as_slice());
        let tile_mode = grad.base.tile_mode.to_skia();
        let matrix = &grad.base.matrix.0;
        Shader(skia_safe::Shader::linear_gradient(
            points,
            colors,
            positions,
            tile_mode,
            None,
            matrix,
        ).unwrap())
    }

    pub fn new_radial_gradient(grad: RadialGradient) -> Shader {
        let colors_list: Vec<skia_safe::Color> = grad.base.colors.into_iter().map(|color| skia_safe::Color::new(color)).collect();
        let colors = skia_safe::gradient_shader::GradientShaderColors::Colors(&colors_list);
        let positions = Some(grad.base.positions.as_slice());
        let tile_mode = grad.base.tile_mode.to_skia();
        let matrix = &grad.base.matrix.0;
        Shader(skia_safe::Shader::two_point_conical_gradient(
            (grad.start_circle.0 as f32, grad.start_circle.1 as f32),
            grad.start_circle.2 as f32,
            (grad.end_circle.0 as f32, grad.end_circle.1 as f32),
            grad.end_circle.2 as f32,
            colors,
            positions,
            tile_mode,
            None,
            matrix,
        ).unwrap())
    }

    pub fn new_from_surface_image(surface: &Surface, matrix: Matrix) -> Shader {
        Shader(surface.image_snapshot().to_shader(
            (skia_safe::TileMode::Repeat, skia_safe::TileMode::Repeat),
            skia_safe::SamplingOptions::default(),
            Some(&matrix.0),
        ).unwrap())
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        // Implemented by Skia-safe.
    }
}

pub struct PathEffect(skia_safe::PathEffect);

impl PathEffect {
    pub fn new_dash_path(intervals: &[f32], phase: f32) -> PathEffect {
        PathEffect(skia_safe::PathEffect::dash(intervals, phase).unwrap())
    }
}

impl Drop for PathEffect {
    fn drop(&mut self) {
        // Implemented by Skia-safe.
    }
}

// New Bindings additions

impl Surface {
    fn new_rgba_impl(width: u32, height: u32, alpha_type: skia_safe::AlphaType) -> Option<Surface> {
        let size = skia_safe::ISize::new(width as i32, height as i32);
        let image_info = skia_safe::ImageInfo::new(size, skia_safe::ColorType::n32(), alpha_type, None);
        let surface = skia_safe::Surface::new_raster(&image_info, None, None).unwrap();
        let canvas = Canvas(surface.clone());
        Some(Surface { surface, canvas })
    }
    fn image_snapshot(&self) -> skia_safe::Image {
        self.surface.clone().image_snapshot()
    }
}

trait ToSkia<SkType> {
    fn to_skia(&self) -> SkType;
}

impl ToSkia<skia_safe::BlendMode> for BlendMode {
    fn to_skia(&self) -> skia_safe::BlendMode {
        match self {
            BlendMode::Clear => skia_safe::BlendMode::Clear,
            BlendMode::SourceOver => skia_safe::BlendMode::SrcOver,
            BlendMode::DestinationOver => skia_safe::BlendMode::DstOver,
            BlendMode::SourceIn => skia_safe::BlendMode::SrcIn,
            BlendMode::DestinationIn => skia_safe::BlendMode::DstIn,
            BlendMode::SourceOut => skia_safe::BlendMode::SrcOut,
            BlendMode::DestinationOut => skia_safe::BlendMode::DstOut,
            BlendMode::SourceAtop => skia_safe::BlendMode::SrcATop,
            BlendMode::Xor => skia_safe::BlendMode::Xor,
            BlendMode::Multiply => skia_safe::BlendMode::Multiply,
            BlendMode::Screen => skia_safe::BlendMode::Screen,
            BlendMode::Darken => skia_safe::BlendMode::Darken,
            BlendMode::Lighten => skia_safe::BlendMode::Lighten,
        }
    }
}

impl ToSkia<skia_safe::PaintStyle> for PaintStyle {
    fn to_skia(&self) -> skia_safe::PaintStyle {
        match self {
            PaintStyle::Fill => skia_safe::PaintStyle::Fill,
            PaintStyle::Stroke => skia_safe::PaintStyle::Stroke,
        }
    }
}

impl ToSkia<skia_safe::PathFillType> for FillType {
    fn to_skia(&self) -> skia_safe::PathFillType {
        match self {
            FillType::Winding => skia_safe::PathFillType::Winding,
            FillType::EvenOdd => skia_safe::PathFillType::EvenOdd,
        }
    }
}

impl ToSkia<skia_safe::PaintCap> for StrokeCap {
    fn to_skia(&self) -> skia_safe::PaintCap {
        match self {
            StrokeCap::Butt => skia_safe::PaintCap::Butt,
            StrokeCap::Round => skia_safe::PaintCap::Round,
            StrokeCap::Square => skia_safe::PaintCap::Square,
        }
    }
}

impl ToSkia<skia_safe::PaintJoin> for StrokeJoin {
    fn to_skia(&self) -> skia_safe::PaintJoin {
        match self {
            StrokeJoin::Miter => skia_safe::PaintJoin::Miter,
            StrokeJoin::Round => skia_safe::PaintJoin::Round,
            StrokeJoin::Bevel => skia_safe::PaintJoin::Bevel,
        }
    }
}

impl ToSkia<skia_safe::TileMode> for TileMode {
    fn to_skia(&self) -> skia_safe::TileMode {
        match self {
            TileMode::Clamp => skia_safe::TileMode::Clamp,
            TileMode::Repeat => skia_safe::TileMode::Repeat,
            TileMode::Mirror => skia_safe::TileMode::Mirror,
        }
    }
}

impl ToSkia<skia_safe::FilterQuality> for FilterQuality {
    fn to_skia(&self) -> skia_safe::FilterQuality {
        match self {
            FilterQuality::None => skia_safe::FilterQuality::None,
            FilterQuality::Low => skia_safe::FilterQuality::Low,
            FilterQuality::Medium => skia_safe::FilterQuality::Medium,
            FilterQuality::High => skia_safe::FilterQuality::High,
        }
    }
}
