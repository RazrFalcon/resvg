use std::ffi::CString;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::slice;

#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
mod ffi;

pub use ffi::skiac_canvas;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PaintStyle {
    Fill = 0,
    Stroke = 1,
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

pub struct Context(*mut ffi::skiac_context);

impl Context {

    pub fn set_from_canvas(canvas: &Canvas) {
        unsafe {
            let context = ffi::skiac_canvas_get_context(canvas.0);
            ffi::skiac_set_context(context);
        }
    }
} 

pub struct Surface(*mut ffi::skiac_surface);
     
impl Surface {


    pub fn new_rgba(width: u32, height: u32) -> Option<Surface> {        
        unsafe {
            Self::from_ptr(ffi::skiac_surface_create_rgba(width, height))
        }
    }

    pub fn new_rgba_premultiplied(width: u32, height: u32) -> Option<Surface> {        
        unsafe {
            Self::from_ptr(ffi::skiac_surface_create_rgba_premultiplied(width, height))
        }
    }

    pub fn from_file(path: &PathBuf) -> Option<Surface> {
        let c_path = CString::new(path.to_str().unwrap()).unwrap();
        unsafe { Self::from_ptr(ffi::skiac_surface_create_from_file(c_path.as_ptr())) }
    }

    pub fn from_data(data: &[u8]) -> Option<Surface> {
        unsafe { Self::from_ptr(ffi::skiac_surface_create_from_image_data(data.as_ptr(), data.len() as u32)) }
    }

    unsafe fn from_ptr(surface: *mut ffi::skiac_surface) -> Option<Surface> {
        if surface.is_null() {
            None
        } else {
            Some(Surface(surface))
        }
    }

    pub fn destroy(&mut self) {
        unsafe { ffi::skiac_surface_destroy(self.0); }
    }

    pub fn copy_rgba(&self, x: u32, y: u32, width: u32, height: u32) -> Option<Surface> {
        unsafe { Self::from_ptr(ffi::skiac_surface_copy_rgba(self.0, x, y, width, height)) }
    }

    pub fn try_clone(&self) -> Option<Surface> {
        unsafe { Self::from_ptr(ffi::skiac_surface_copy_rgba(self.0, 0, 0, self.get_width() as u32, self.get_height() as u32)) }
    }

    pub fn save(&self, path: &str) -> bool {
        let c_path = CString::new(path).unwrap();
        unsafe { ffi::skiac_surface_save(self.0, c_path.as_ptr()) }
    }

    pub fn get_canvas(&mut self) -> Canvas {
        unsafe { Canvas(ffi::skiac_surface_get_canvas(self.0)) }
    }

    pub fn get_width(&self) -> i32 {
        unsafe { ffi::skiac_surface_get_width(self.0) }
    }

    pub fn get_height(&self) -> i32 {
        unsafe { ffi::skiac_surface_get_height(self.0) }
    }

    pub fn data(&self) -> SurfaceData {
        unsafe {
            
            let mut data: ffi::skiac_surface_data = Default::default();
            ffi::skiac_surface_read_pixels(self.0, &mut data);

            SurfaceData {
                data: data,
                slice: slice::from_raw_parts_mut(data.ptr, data.size as usize),
            }
        }
    }

    pub fn data_mut(&mut self) -> SurfaceData {
        unsafe {
            
            let mut data: ffi::skiac_surface_data = Default::default();
            ffi::skiac_surface_read_pixels(self.0, &mut data);

            SurfaceData {
                data: data,
                slice: slice::from_raw_parts_mut(data.ptr, data.size as usize),
            }
        }
    }

    pub fn commit_data(&self, data: SurfaceData) {
        unsafe {            
            ffi::skiac_surface_write_pixels(self.0, &data.data);
        }
    }

}

impl Drop for Surface {

    fn drop(&mut self) {
        unsafe { 
            ffi::skiac_surface_destroy(self.0);
        }
    }
}


pub struct SurfaceData<'a> {
    data: ffi::skiac_surface_data,
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

impl<'a> Drop for SurfaceData <'a>{

    fn drop(&mut self) {
        unsafe { 
            ffi::skiac_surface_data_delete(&mut self.data);
        }
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
    pub fn invert(&self) -> Matrix {
        unsafe { Matrix(ffi::skiac_matrix_create_inverse(self.0)) }
    }
    pub fn map_rect(&self, l: f64, t: f64, r: f64, b: f64) -> (f32, f32, f32, f32) {
        let src = ffi::skia_rect { 
            left: l as f32,
            top: t as f32,
            right: r as f32,
            bottom: b as f32
        };
        let mut dst = ffi::skia_rect{ left: 0.0, top: 0.0, right: 0.0, bottom: 0.0 };
        unsafe {
            ffi::skiac_matrix_map_rect(self.0, &mut dst, &src);
            (dst.left, dst.top, dst.right, dst.bottom)
        }
    }

    pub fn data(&self) -> (f64, f64, f64, f64, f64, f64) {
        let mat = unsafe { ffi::skiac_matrix_get_data(self.0) };
        (mat.a, mat.b, mat.c, mat.d, mat.e, mat.f)
    }
    pub fn reset(&self) {
        unsafe { ffi::skiac_matrix_reset(self.0); }
    }    
    pub fn pre_translate(&self, dx: f64, dy: f64) {
        unsafe { ffi::skiac_matrix_pre_translate(self.0, dx, dy); }
    }    
    pub fn pre_scale(&self, sx: f64, sy: f64) {
        unsafe { ffi::skiac_matrix_pre_scale(self.0, sx, sy); }
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

    pub unsafe fn from_raw(ptr: *mut ffi::skiac_canvas) -> Canvas {
        Canvas(ptr)
    }
    pub fn get_context(&self) -> Context {
         unsafe { Context(ffi::skiac_canvas_get_context(self.0)) }
    }
    pub fn clear(&self, color: u32) {
         unsafe { ffi::skiac_canvas_clear(self.0, color); }
    }
    pub fn clear_rgba(&self, r: u8, g: u8, b: u8, a: u8) {        
        self.clear((a as u32) << 24 | (r as u32) << 16 | (g as u32) << 8 | b as u32);
    }
    pub fn flush(&self) {
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
    pub fn get_total_matrix(&mut self) -> Matrix {
        unsafe { Matrix(ffi::skiac_canvas_get_total_matrix(self.0)) }
    }
    pub fn draw_path(&mut self, path: &Path, paint: &Paint) {
        unsafe { ffi::skiac_canvas_draw_path(self.0, path.0, paint.0); }
    }
    pub fn draw_rect(&mut self, x: f64, y: f64, w: f64, h: f64, paint: &Paint) {
        unsafe { ffi::skiac_canvas_draw_rect(self.0, x, y, w, h, paint.0); }
    }
    pub fn draw_surface(&mut self, surface: &Surface, left: f64, top: f64, alpha: u8, blend_mode: BlendMode) {
        unsafe { ffi::skiac_canvas_draw_surface(self.0, surface.0, left, top, alpha, blend_mode as u32); }
    }
    pub fn draw_surface_rect(&mut self, surface: &Surface, x: f64, y: f64, w: f64, h: f64) {
        unsafe { ffi::skiac_canvas_draw_surface_rect(self.0, surface.0, x, y, w, h) }
    }
    pub fn reset_matrix(&self) {
        unsafe { ffi::skiac_canvas_reset_matrix(self.0); }
    }
    pub fn clip_rect(&self, l: f64, t: f64, r: f64, b: f64) {
        let rect = ffi::skia_rect { 
            left: l as f32,
            top: t as f32,
            right: r as f32,
            bottom: b as f32
        };
        unsafe { 
            ffi::skiac_canvas_clip_rect(self.0, &rect)
        }
    } 
    pub fn save(&self) {
        unsafe { ffi::skiac_canvas_save(self.0); }
    }
    pub fn restore(&self) {
        unsafe { ffi::skiac_canvas_restore(self.0); }
    }
}

pub struct Paint(*mut ffi::skiac_paint);

impl Paint {
    pub fn new() -> Paint {
        unsafe { Paint(ffi::skiac_paint_create()) }
    }
    pub fn set_style(&mut self, style: PaintStyle) {
        unsafe { ffi::skiac_paint_set_style(self.0, style as u32); }
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
        unsafe { ffi::skiac_paint_set_blend_mode(self.0, blend_mode as u32); }
    }
    pub fn set_shader(&mut self, shader: &Shader) {
        unsafe { ffi::skiac_paint_set_shader(self.0, shader.0); }
    }
    pub fn set_stroke_width(&mut self, width: f64) {
        unsafe { ffi::skiac_paint_set_stroke_width(self.0, width); }
    }
    pub fn set_stroke_cap(&mut self, cap: StrokeCap) {
        unsafe { ffi::skiac_paint_set_stroke_cap(self.0, cap as u32); }
    }
    pub fn set_stroke_join(&mut self, join: StrokeJoin) {
        unsafe { ffi::skiac_paint_set_stroke_join(self.0, join as u32); }
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
            ffi::skia_point{x: grad.start_point.0 as f32, y: grad.start_point.1 as f32}, 
            ffi::skia_point{x: grad.end_point.0 as f32, y: grad.end_point.1 as f32}
        ];

        unsafe { 
            Shader(ffi::skiac_shader_make_linear_gradient(
                points.as_ptr(), 
                grad.base.colors.as_ptr(), 
                grad.base.positions.as_ptr(), 
                grad.base.colors.len() as i32,
                grad.base.tile_mode as u32,
                0 as u32,
                grad.base.matrix.0)
            )
        }
    }

    pub fn new_radial_gradient(grad: RadialGradient) -> Shader {

        let start_point = ffi::skia_point{ 
            x: grad.start_circle.0 as f32, 
            y: grad.start_circle.1 as f32
        };    
        
        let end_point = ffi::skia_point{
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
                grad.base.tile_mode as u32,
                0 as u32,
                grad.base.matrix.0)
            )
        }
    }

    pub fn new_from_surface_image(surface: &Surface, matrix: Matrix) -> Shader {
        unsafe {
            Shader(ffi::skiac_shader_make_from_surface_image(surface.0, matrix.0))
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

    pub fn new_dash_path(intervals: &Vec<f64>, count: i32, phase: f32) -> PathEffect {       

        // Convert to 32-bit float
        let mut intervals32: Vec<f32> = Vec::with_capacity(intervals.len());
        for dash in intervals {
            intervals32.push(*dash as f32);
        }

        unsafe { 
            PathEffect(ffi::skiac_path_effect_make_dash_path(intervals32.as_ptr(), count, phase))
        }
    }

}

impl Drop for PathEffect {
    fn drop(&mut self) {
        unsafe { ffi::skiac_path_effect_destroy(self.0); }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
