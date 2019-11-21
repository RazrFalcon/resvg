use std::ffi::CString;
use std::ops::{Deref, DerefMut};
use std::slice;

#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
mod ffi;

pub use ffi::skiac_surface;
pub use ffi::skiac_canvas;

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
    ptr: *mut ffi::skiac_surface,
    canvas: Canvas,
}

impl Surface {
    pub fn new_rgba(width: u32, height: u32) -> Option<Surface> {
        unsafe {
            Self::from_ptr(ffi::skiac_surface_create_rgba(width as i32, height as i32))
        }
    }

    pub fn new_rgba_premultiplied(width: u32, height: u32) -> Option<Surface> {
        unsafe {
            Self::from_ptr(ffi::skiac_surface_create_rgba_premultiplied(width as i32, height as i32))
        }
    }

    pub unsafe fn from_ptr(ptr: *mut ffi::skiac_surface) -> Option<Surface> {
        if ptr.is_null() {
            None
        } else {
            Some(Surface {
                ptr,
                canvas: Canvas(ffi::skiac_surface_get_canvas(ptr))
            })
        }
    }

    pub fn destroy(&mut self) {
        unsafe { ffi::skiac_surface_destroy(self.ptr); }
    }

    pub fn copy_rgba(&self, x: u32, y: u32, width: u32, height: u32) -> Option<Surface> {
        unsafe { Self::from_ptr(ffi::skiac_surface_copy_rgba(self.ptr, x, y, width, height)) }
    }

    pub fn try_clone(&self) -> Option<Surface> {
        unsafe { Self::from_ptr(ffi::skiac_surface_copy_rgba(self.ptr, 0, 0, self.width(), self.height())) }
    }

    pub fn save_png(&self, path: &str) -> bool {
        let c_path = CString::new(path).unwrap();
        unsafe { ffi::skiac_surface_save(self.ptr, c_path.as_ptr()) }
    }

    pub fn width(&self) -> u32 {
        unsafe { ffi::skiac_surface_get_width(self.ptr) as u32 }
    }

    pub fn height(&self) -> u32 {
        unsafe { ffi::skiac_surface_get_height(self.ptr) as u32 }
    }

    pub fn data(&self) -> SurfaceData {
        unsafe {
            let mut data = ffi::skiac_surface_data {
                ptr: std::ptr::null_mut(),
                size: 0,
            };
            ffi::skiac_surface_read_pixels(self.ptr, &mut data);

            SurfaceData {
                slice: slice::from_raw_parts_mut(data.ptr, data.size as usize),
            }
        }
    }

    pub fn data_mut(&mut self) -> SurfaceData {
        unsafe {
            let mut data = ffi::skiac_surface_data {
                ptr: std::ptr::null_mut(),
                size: 0,
            };
            ffi::skiac_surface_read_pixels(self.ptr, &mut data);

            SurfaceData {
                slice: slice::from_raw_parts_mut(data.ptr, data.size as usize),
            }
        }
    }

    pub fn is_bgra() -> bool {
        unsafe { ffi::skiac_is_surface_bgra() }
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
        unsafe {
            ffi::skiac_surface_destroy(self.ptr);
        }
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


pub struct Matrix(*mut ffi::skiac_matrix);

impl Matrix {
    pub fn new() -> Matrix {
        unsafe { Matrix(ffi::skiac_matrix_create()) }
    }

    pub fn new_from(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Matrix {
        unsafe { Matrix(ffi::skiac_matrix_create_from(a, b, c, d, e, f)) }
    }

    pub fn invert(&self) -> Option<Matrix> {
        unsafe {
            let ptr = ffi::skiac_matrix_create_inverse(self.0);
            if ptr.is_null() {
                None
            } else {
                Some(Matrix(ptr))
            }
        }
    }

    pub fn data(&self) -> (f64, f64, f64, f64, f64, f64) {
        let mat = unsafe { ffi::skiac_matrix_get_data(self.0) };
        (mat.a, mat.b, mat.c, mat.d, mat.e, mat.f)
    }
}

impl Default for Matrix {
    fn default() -> Matrix {
        unsafe { Matrix(ffi::skiac_matrix_create()) }
    }
}

impl Drop for Matrix {
    fn drop(&mut self) {
        unsafe { ffi::skiac_matrix_destroy(self.0) }
    }
}

pub struct Canvas(*mut ffi::skiac_canvas);

impl Canvas {
    pub unsafe fn from_ptr(ptr: *mut ffi::skiac_canvas) -> Option<Canvas> {
        if ptr.is_null() {
            None
        } else {
            Some(Canvas(ptr))
        }
    }

    pub fn clear(&mut self) {
        unsafe { ffi::skiac_canvas_clear(self.0, 0); }
    }

    pub fn fill(&mut self, r: u8, g: u8, b: u8, a: u8) {
        unsafe { ffi::skiac_canvas_clear(self.0, (a as u32) << 24 | (r as u32) << 16 | (g as u32) << 8 | b as u32); }
    }

    pub fn flush(&mut self) {
         unsafe { ffi::skiac_canvas_flush(self.0); }
    }

    pub fn set_matrix(&mut self, matrix: &Matrix) {
        unsafe { ffi::skiac_canvas_set_matrix(self.0, matrix.0); }
    }

    pub fn concat(&mut self, matrix: &Matrix) {
        unsafe { ffi::skiac_canvas_concat(self.0, matrix.0); }
    }

    pub fn scale(&mut self, sx: f64, sy: f64) {
        unsafe { ffi::skiac_canvas_scale(self.0, sx, sy); }
    }

    pub fn translate(&mut self, dx: f64, dy: f64) {
        unsafe { ffi::skiac_canvas_translate(self.0, dx, dy); }
    }

    pub fn get_matrix(&self) -> Matrix {
        unsafe { Matrix(ffi::skiac_canvas_get_total_matrix(self.0)) }
    }

    pub fn draw_path(&mut self, path: &Path, paint: &Paint) {
        unsafe { ffi::skiac_canvas_draw_path(self.0, path.0, paint.0); }
    }

    pub fn draw_rect(&mut self, x: f64, y: f64, w: f64, h: f64, paint: &Paint) {
        unsafe { ffi::skiac_canvas_draw_rect(self.0, x, y, w, h, paint.0); }
    }

    pub fn draw_surface(&mut self, surface: &Surface, left: f64, top: f64, alpha: u8,
                        blend_mode: BlendMode, filter_quality: FilterQuality) {
        unsafe {
            ffi::skiac_canvas_draw_surface(
                self.0, surface.ptr, left, top, alpha, blend_mode as i32, filter_quality as i32,
            );
        }
    }

    pub fn draw_surface_rect(&mut self, surface: &Surface, x: f64, y: f64, w: f64, h: f64,
                             filter_quality: FilterQuality) {
        unsafe {
            ffi::skiac_canvas_draw_surface_rect(
                self.0, surface.ptr, x, y, w, h, filter_quality as i32,
            );
        }
    }

    pub fn reset_matrix(&mut self) {
        unsafe { ffi::skiac_canvas_reset_matrix(self.0); }
    }

    pub fn set_clip_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        unsafe { ffi::skiac_canvas_clip_rect(self.0, x, y, w, h); }
    }

    pub fn save(&mut self) {
        unsafe { ffi::skiac_canvas_save(self.0); }
    }

    pub fn restore(&mut self) {
        unsafe { ffi::skiac_canvas_restore(self.0); }
    }
}

pub struct Paint(*mut ffi::skiac_paint);

impl Paint {
    pub fn new() -> Paint {
        unsafe { Paint(ffi::skiac_paint_create()) }
    }
    pub fn set_style(&mut self, style: PaintStyle) {
        unsafe { ffi::skiac_paint_set_style(self.0, style as i32); }
    }
    pub fn set_color(&mut self, r: u8, g: u8, b: u8, a: u8) {
        unsafe { ffi::skiac_paint_set_color(self.0, r, g, b, a); }
    }
    pub fn set_alpha(&mut self, a: u8) {
        unsafe { ffi::skiac_paint_set_alpha(self.0, a); }
    }
    pub fn set_anti_alias(&mut self, aa: bool) {
        unsafe { ffi::skiac_paint_set_anti_alias(self.0, aa); }
    }
    pub fn set_blend_mode(&mut self, blend_mode: BlendMode) {
        unsafe { ffi::skiac_paint_set_blend_mode(self.0, blend_mode as i32); }
    }
    pub fn set_shader(&mut self, shader: &Shader) {
        unsafe { ffi::skiac_paint_set_shader(self.0, shader.0); }
    }
    pub fn set_stroke_width(&mut self, width: f64) {
        unsafe { ffi::skiac_paint_set_stroke_width(self.0, width); }
    }
    pub fn set_stroke_cap(&mut self, cap: StrokeCap) {
        unsafe { ffi::skiac_paint_set_stroke_cap(self.0, cap as i32); }
    }
    pub fn set_stroke_join(&mut self, join: StrokeJoin) {
        unsafe { ffi::skiac_paint_set_stroke_join(self.0, join as i32); }
    }
    pub fn set_stroke_miter(&mut self, miter: f64) {
        unsafe { ffi::skiac_paint_set_stroke_miter(self.0, miter as f32); }
    }
    pub fn set_path_effect(&mut self, path_effect: PathEffect) {
        unsafe { ffi::skiac_paint_set_path_effect(self.0, path_effect.0); }
    }
}

impl Drop for Paint {
    fn drop(&mut self) {
        unsafe { ffi::skiac_paint_destroy(self.0) }
    }
}


pub struct Path(*mut ffi::skiac_path);

impl Path {
    pub fn new() -> Path {
        unsafe { Path(ffi::skiac_path_create()) }
    }

    pub fn set_fill_type(&mut self, kind: FillType) {
        unsafe { ffi::skiac_path_set_fill_type(self.0, kind as i32); }
    }

    pub fn move_to(&mut self, x: f64, y: f64) {
        unsafe { ffi::skiac_path_move_to(self.0, x, y); }
    }

    pub fn line_to(&mut self, x: f64, y: f64) {
        unsafe { ffi::skiac_path_line_to(self.0, x, y); }
    }

    pub fn cubic_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) {
        unsafe { ffi::skiac_path_cubic_to(self.0, x1, y1, x2, y2, x3, y3); }
    }

    pub fn close(&mut self) {
        unsafe { ffi::skiac_path_close(self.0); }
    }
}

impl Drop for Path {
    fn drop(&mut self) {
        unsafe { ffi::skiac_path_destroy(self.0); }
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

pub struct Shader(*mut ffi::skiac_shader);

impl Shader {
    pub fn new_linear_gradient(grad:  LinearGradient) -> Shader {
        let points = [
            ffi::skia_point {x: grad.start_point.0 as f32, y: grad.start_point.1 as f32},
            ffi::skia_point {x: grad.end_point.0 as f32, y: grad.end_point.1 as f32}
        ];

        unsafe {
            Shader(ffi::skiac_shader_make_linear_gradient(
                points.as_ptr(),
                grad.base.colors.as_ptr(),
                grad.base.positions.as_ptr(),
                grad.base.colors.len() as i32,
                grad.base.tile_mode as i32,
                0 as u32,
                grad.base.matrix.0)
            )
        }
    }

    pub fn new_radial_gradient(grad: RadialGradient) -> Shader {
        let start_point = ffi::skia_point {
            x: grad.start_circle.0 as f32,
            y: grad.start_circle.1 as f32
        };

        let end_point = ffi::skia_point {
            x: grad.end_circle.0 as f32,
            y: grad.end_circle.1 as f32
        };

        let start_radius = grad.start_circle.2 as f32;
        let end_radius = grad.end_circle.2 as f32;

        unsafe {
            Shader(ffi::skiac_shader_make_two_point_conical_gradient(
                start_point, start_radius,
                end_point, end_radius,
                grad.base.colors.as_ptr(),
                grad.base.positions.as_ptr(),
                grad.base.colors.len() as i32,
                grad.base.tile_mode as i32,
                0 as u32,
                grad.base.matrix.0)
            )
        }
    }

    pub fn new_from_surface_image(surface: &Surface, matrix: Matrix) -> Shader {
        unsafe {
            Shader(ffi::skiac_shader_make_from_surface_image(surface.ptr, matrix.0))
        }
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe { ffi::skiac_shader_destroy(self.0); }
    }
}


pub struct PathEffect(*mut ffi::skiac_path_effect);

impl PathEffect {
    pub fn new_dash_path(intervals: &[f32], phase: f32) -> PathEffect {
        unsafe {
            PathEffect(ffi::skiac_path_effect_make_dash_path(
                intervals.as_ptr(), intervals.len() as i32, phase
            ))
        }
    }
}

impl Drop for PathEffect {
    fn drop(&mut self) {
        unsafe { ffi::skiac_path_effect_destroy(self.0); }
    }
}
