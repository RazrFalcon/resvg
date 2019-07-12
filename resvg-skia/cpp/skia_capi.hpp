#ifndef SKIA_CAPI_H
#define SKIA_CAPI_H

#include <stdint.h>
#include <SkScalar.h>
#include <SkColor.h>

#define INIT_STRUCT(x) \
    struct x; \
    typedef struct x x;

INIT_STRUCT(skiac_context)
INIT_STRUCT(skiac_bitmap)
INIT_STRUCT(skiac_surface)
INIT_STRUCT(skiac_canvas)
INIT_STRUCT(skiac_matrix)
INIT_STRUCT(skiac_paint)
INIT_STRUCT(skiac_path)
INIT_STRUCT(skiac_shader)
INIT_STRUCT(skiac_path_effect)

#undef INIT_STRUCT

typedef SkScalar skia_scalar;

struct skia_matrix {
    double a;
    double b;
    double c;
    double d;
    double e;
    double f;
};

struct skia_point {
    SkScalar x;
    SkScalar y;
};

struct skia_rect {
    SkScalar left;
    SkScalar top;
    SkScalar right;
    SkScalar bottom;
};

struct skiac_surface_data {
    char* ptr;
    uint32_t size;
};

enum class PaintStyle {
    Fill = 0,
    Stroke = 1,
    __Size
};

enum class StrokeCap {
    Butt = 0,
    Round = 1,
    Square = 2,
    __Size
};

enum class StrokeJoin {
    Miter = 0,
    Round = 1,
    Bevel = 2,
    __Size
};

enum class TileMode {
    Clamp = 0,
    Repeat = 1,
    Mirror = 2,
    __Size
};

enum class BlendMode {
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
    __Size
};

extern "C" {

    // Surface
    skiac_surface* skiac_surface_create_rgba_premultiplied(int width, int height);
    skiac_surface* skiac_surface_create_rgba(int width, int height);
    skiac_surface* skiac_surface_create_from_image_data(const void* buffer, uint32_t size);
    skiac_surface* skiac_surface_create_from_file(const char *path);
    void skiac_surface_destroy(skiac_surface* c_surface);
    skiac_surface* skiac_surface_copy_rgba(skiac_surface *c_surface, uint32_t x, uint32_t y, uint32_t width, uint32_t height);
    bool skiac_surface_save(skiac_surface* c_surface, const char *path);
    skiac_canvas* skiac_surface_get_canvas(skiac_surface* c_surface);
    int skiac_surface_get_width(const skiac_surface* c_surface);
    int skiac_surface_get_height(const skiac_surface* c_surface);
    bool skiac_surface_read_pixels(skiac_surface* c_surface, skiac_surface_data* data);
    bool skiac_surface_write_pixels(skiac_surface* c_surface, const skiac_surface_data* data);

    // Bitmap
    skiac_bitmap* skiac_bitmap_create_rgba(uint32_t width, uint32_t height);
    void skiac_bitmap_destroy(skiac_bitmap* c_bitmap);

    // Canvas
    void skiac_canvas_clear(skiac_canvas* c_canvas, uint32_t color);
    void skiac_canvas_flush(skiac_canvas* c_canvas);
    void skiac_canvas_set_matrix(skiac_canvas* c_canvas, skiac_matrix *c_mat);
    void skiac_canvas_concat(skiac_canvas* c_canvas, skiac_matrix* c_mat);
    void skiac_canvas_scale(skiac_canvas* c_canvas, double sx, double sy);
    void skiac_canvas_translate(skiac_canvas* c_canvas, double dx, double dy);
    skiac_matrix* skiac_canvas_get_total_matrix(skiac_canvas* c_canvas);
    void skiac_canvas_draw_path(skiac_canvas* c_canvas, skiac_path* c_path, skiac_paint* c_paint);
    void skiac_canvas_draw_rect(skiac_canvas* c_canvas, double x, double y, double w, double h, skiac_paint* c_paint);
    void skiac_canvas_draw_surface(skiac_canvas* c_canvas, skiac_surface* c_surface, double left, double top, uint8_t alpha, BlendMode blendMode);
    void skiac_canvas_draw_surface_rect(skiac_canvas* c_canvas, skiac_surface* c_surface, double x, double y, double w, double h);
    void skiac_canvas_reset_matrix(skiac_canvas* c_canvas);
    void skiac_canvas_clip_rect(skiac_canvas* c_canvas, const skia_rect* c_rect);
    void skiac_canvas_save(skiac_canvas* c_canvas);
    void skiac_canvas_restore(skiac_canvas* c_canvas);
    
    // Matrix
    skiac_matrix *skiac_matrix_create();
    skiac_matrix *skiac_matrix_create_from(double a, double b, double c, double d, double e, double f);
    skiac_matrix *skiac_matrix_create_inverse(skiac_matrix *c_mat);
    void skiac_matrix_map_rect(skiac_matrix *c_mat, skia_rect *dst, const skia_rect* src);
    void skiac_matrix_destroy(skiac_matrix *c_mat);
    void skiac_matrix_reset(skiac_matrix *c_mat);
    void skiac_matrix_pre_translate(skiac_matrix *c_mat, double dx, double dy);
    void skiac_matrix_pre_scale(skiac_matrix *c_mat, double sx, double sy);

    // Paint
    skiac_paint* skiac_paint_create();
    void skiac_paint_destroy(skiac_paint* c_paint);
    void skiac_paint_set_style(skiac_paint* c_paint, PaintStyle style);
    void skiac_paint_set_color(skiac_paint* c_paint, uint8_t r, uint8_t g, uint8_t b, uint8_t a);
    void skiac_paint_set_alpha(skiac_paint* c_paint, uint8_t a);
    void skiac_paint_set_anti_alias(skiac_paint* c_paint, bool aa);
    void skiac_paint_set_blend_mode(skiac_paint* c_paint, BlendMode blendMode);
    void skiac_paint_set_shader(skiac_paint* c_paint, skiac_shader* c_shader);
    void skiac_paint_set_stroke_width(skiac_paint* c_paint, double width);
    void skiac_paint_set_stroke_cap(skiac_paint* c_paint, StrokeCap cap);
    void skiac_paint_set_stroke_join(skiac_paint* c_paint, StrokeJoin join);
    void skiac_paint_set_stroke_miter(skiac_paint* c_paint, SkScalar miter);
    void skiac_paint_set_path_effect(skiac_paint* c_paint, skiac_path_effect* c_path_effect);
    
     // Path
    skiac_path* skiac_path_create();
    void skiac_path_destroy(skiac_path* c_path);
    void skiac_path_move_to(skiac_path* c_path, double x, double y);
    void skiac_path_line_to(skiac_path* c_path, double x, double y);
    void skiac_path_cubic_to(skiac_path* c_path, double x1, double y1, double x2, double y2, double x3, double y3);
    void skiac_path_close(skiac_path* c_path);

    // PathEffect
    skiac_path_effect* skiac_path_effect_make_dash_path(const SkScalar* intervals, int count, SkScalar phase);
    void skiac_path_effect_destroy(skiac_path_effect* c_path_effect);

    // Shader
    skiac_shader* skiac_shader_make_linear_gradient(
        const skia_point* points, 
        const SkColor* colors, const SkScalar* positions, 
        int count, TileMode tile_mode,
        uint32_t flags, skiac_matrix *c_mat);
    
    skiac_shader* skiac_shader_make_two_point_conical_gradient(
        const skia_point start_point, SkScalar start_radius,
        const skia_point end_point, SkScalar end_radius,
        const SkColor* colors, const SkScalar* positions, 
        int count, TileMode tile_mode,
        uint32_t flags, skiac_matrix *c_mat);
    
    skiac_shader* skiac_shader_make_from_surface_image(skiac_surface* c_surface, const skiac_matrix* c_matrix);

    void skiac_shader_destroy(skiac_shader* c_shader);
}

#endif // SKIA_CAPI_H