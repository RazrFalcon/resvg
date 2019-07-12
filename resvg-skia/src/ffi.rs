
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct skiac_bitmap {
    _unused: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct skiac_context {
    _unused: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct skiac_surface {
    _unused: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct skiac_canvas {
    _unused: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct skiac_matrix {
    _unused: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct skia_matrix {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct skia_point {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct skia_rect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct skiac_paint {
    _unused: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct skiac_path {
    _unused: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct skiac_shader {
    _unused: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct skiac_path_effect {
    _unused: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct skiac_surface_data {
    pub ptr: *mut u8,
    pub size: u32,
}

impl Default for skiac_surface_data {
    fn default() -> skiac_surface_data {
        skiac_surface_data {
            ptr: std::ptr::null_mut(),
            size: 0,
        }
    }
}

// Surface

extern "C" {
    pub fn skiac_surface_create_rgba(width: u32, height: u32) -> *mut skiac_surface;
}
extern "C" {
    pub fn skiac_surface_create_rgba_premultiplied(width: u32, height: u32) -> *mut skiac_surface;
}
extern "C" {
    pub fn skiac_surface_copy_rgba(c_surface: *mut skiac_surface, x: u32, y: u32, width: u32, height: u32) -> *mut skiac_surface;
}
extern "C" {
    pub fn skiac_surface_create_from_image_data(buffer: *const u8, size: u32) -> *mut skiac_surface;
}
extern "C" {
    pub fn skiac_surface_create_from_file(path: *const ::std::os::raw::c_char) -> *mut skiac_surface;
}
extern "C" {
    pub fn skiac_surface_destroy(c_surface: *mut skiac_surface);
}
extern "C" {
    pub fn skiac_surface_save(c_surface: *mut skiac_surface, path: *const ::std::os::raw::c_char) -> bool;
}
extern "C" {
    pub fn skiac_surface_get_canvas(c_surface: *mut skiac_surface) -> *mut skiac_canvas;
}
extern "C" {
    pub fn skiac_surface_get_width(c_surface: *const skiac_surface) -> i32;
}
extern "C" {
    pub fn skiac_surface_get_height(c_surface: *const skiac_surface) -> i32;
}
extern "C" {
    pub fn skiac_surface_read_pixels(c_surface: *mut skiac_surface, data: *mut skiac_surface_data) -> bool;
}
extern "C" {
    pub fn skiac_surface_write_pixels(c_surface: *mut skiac_surface, data: *const skiac_surface_data) -> bool;
}

// Surface Data
extern "C" {
    pub fn skiac_surface_data_delete(data: *mut skiac_surface_data);
}

// Canvas

extern "C" {
    pub fn skiac_canvas_clear(c_canvas: *mut skiac_canvas, color: u32);
}
extern "C" {
    pub fn skiac_canvas_flush(c_canvas: *mut skiac_canvas);
}
extern "C" {
    pub fn skiac_canvas_set_matrix(c_canvas: *mut skiac_canvas, c_mat: *mut skiac_matrix);
}
extern "C" {
    pub fn skiac_canvas_concat(c_canvas: *mut skiac_canvas, c_mat: *mut skiac_matrix);
}
extern "C" {
    pub fn skiac_canvas_scale(c_canvas: *mut skiac_canvas, sx: f64, sy: f64);
}
extern "C" {
    pub fn skiac_canvas_translate(c_canvas: *mut skiac_canvas, dx: f64, dy: f64);
}
extern "C" {
    pub fn skiac_canvas_get_total_matrix(c_canvas: *mut skiac_canvas) -> *mut skiac_matrix;
}
extern "C" {
    pub fn skiac_canvas_draw_path(c_canvas: *mut skiac_canvas, c_path: *mut skiac_path, c_paint: *mut skiac_paint);
}
extern "C" {
    pub fn skiac_canvas_draw_rect(c_canvas: *mut skiac_canvas, x: f64, y: f64, w: f64, h: f64, c_paint: *mut skiac_paint);
}
extern "C" {
    pub fn skiac_canvas_draw_surface(c_canvas: *mut skiac_canvas, c_surface: *mut skiac_surface, left: f64, top: f64, alpha: u8, blendMode: u32);
}
extern "C" {
    pub fn skiac_canvas_reset_matrix(c_canvas: *mut skiac_canvas);
}
extern "C" {
    pub fn skiac_canvas_clip_rect(c_canvas: *mut skiac_canvas, c_rect: *const skia_rect);
}

// Matrix

extern "C" {
    pub fn skiac_matrix_create() -> *mut skiac_matrix;
}
extern "C" {
    pub fn skiac_matrix_create_from(a: f64, b: f64, c: f64, d: f64, d: f64, f: f64) -> *mut skiac_matrix;
}
extern "C" {
    pub fn skiac_matrix_create_inverse(c_mat: *mut skiac_matrix) -> *mut skiac_matrix;
}
extern "C" {
    pub fn skiac_matrix_map_rect(c_mat: *mut skiac_matrix, dst: *mut skia_rect, src: *const skia_rect);
}
extern "C" {
    pub fn skiac_matrix_get_data(c_mat: *mut skiac_matrix) -> skia_matrix;
}
extern "C" {
    pub fn skiac_matrix_destroy(c_mat: *mut skiac_matrix);
}
extern "C" {
    pub fn skiac_matrix_reset(c_mat: *mut skiac_matrix);
}
extern "C" {
    pub fn skiac_matrix_pre_translate(c_mat: *mut skiac_matrix, dx: f64, dy: f64);
}
extern "C" {
    pub fn skiac_matrix_pre_scale(c_mat: *mut skiac_matrix, sx: f64, sy: f64);
}

// Paint

extern "C" {
    pub fn skiac_paint_create() -> *mut skiac_paint;
}
extern "C" {
    pub fn skiac_paint_destroy(c_paint: *mut skiac_paint);
}
extern "C" {
    pub fn skiac_paint_set_style(c_paint: *mut skiac_paint, s: u32);
}
extern "C" {
    pub fn skiac_paint_set_color(c_paint: *mut skiac_paint, r: u8, g: u8, b: u8, a: u8);
}
extern "C" {
    pub fn skiac_paint_set_alpha(c_paint: *mut skiac_paint, a: u8);
}
extern "C" {
    pub fn skiac_paint_set_anti_alias(c_paint: *mut skiac_paint, aa: bool);
}
extern "C" {
    pub fn skiac_paint_set_blend_mode(c_paint: *mut skiac_paint, blend_mode: u32);
}
extern "C" {
    pub fn skiac_paint_set_shader(c_paint: *mut skiac_paint, c_shader: *mut skiac_shader);
}
extern "C" {
    pub fn skiac_paint_set_stroke_width(c_paint: *mut skiac_paint, width: f64);
}
extern "C" {
    pub fn skiac_paint_set_stroke_cap(c_paint: *mut skiac_paint, cap: u32);
}
extern "C" {
    pub fn skiac_paint_set_stroke_join(c_paint: *mut skiac_paint, join: u32);
}
extern "C" {
    pub fn skiac_paint_set_stroke_miter(c_paint: *mut skiac_paint, miter: f32);
}
extern "C" {
    pub fn skiac_paint_set_path_effect(c_paint: *mut skiac_paint, c_path_effect: *mut skiac_path_effect);
}

// Path

extern "C" {
    pub fn skiac_path_create() -> *mut skiac_path;
}
extern "C" {
    pub fn skiac_path_destroy(c_paint: *mut skiac_path);
}
extern "C" {
    pub fn skiac_path_move_to(c_path: *mut skiac_path, x: f64, y: f64);
}
extern "C" {
    pub fn skiac_path_line_to(c_path: *mut skiac_path, x: f64, y: f64);
}
extern "C" {
    pub fn skiac_path_cubic_to(c_path: *mut skiac_path, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64);
}
extern "C" {
    pub fn skiac_path_close(c_path: *mut skiac_path);
}

// PathEffect

extern "C" {
    pub fn skiac_path_effect_make_dash_path(intervals: *const f32, count: i32, phase: f32) -> *mut skiac_path_effect;
}
extern "C" {
    pub fn skiac_path_effect_destroy(c_path_effect: *mut skiac_path_effect);
}

// Shader

extern "C" {
    pub fn skiac_shader_make_linear_gradient(
        points: *const skia_point,
        colors: *const u32,
        positions: *const f32,
        size: i32,
        tile_mode: u32,
        flags: u32,
        c_mat: *mut skiac_matrix) -> *mut skiac_shader;
}
extern "C" {
    pub fn skiac_shader_make_two_point_conical_gradient(
        start_point: skia_point,
        start_radius: f32,
        end_point: skia_point,
        end_radius: f32,
        colors: *const u32,
        positions: *const f32,
        size: i32,
        tile_mode: u32,
        flags: u32,
        c_mat: *mut skiac_matrix) -> *mut skiac_shader;
}
extern "C" {
    pub fn skiac_shader_make_from_surface_image(
        c_surface: *mut skiac_surface,
        c_matrix: *const skiac_matrix) -> *mut skiac_shader;
}
extern "C" {
    pub fn skiac_shader_destroy(c_shader: *mut skiac_shader);
}
