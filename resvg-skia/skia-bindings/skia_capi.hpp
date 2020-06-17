#ifndef SKIA_CAPI_H
#define SKIA_CAPI_H

#include <stdint.h>

#define INIT_STRUCT(x) \
    struct x; \
    typedef struct x x;

INIT_STRUCT(skiac_surface)
INIT_STRUCT(skiac_canvas)
INIT_STRUCT(skiac_matrix)
INIT_STRUCT(skiac_paint)
INIT_STRUCT(skiac_path)
INIT_STRUCT(skiac_shader)
INIT_STRUCT(skiac_path_effect)

#undef INIT_STRUCT

struct skia_matrix {
    double a;
    double b;
    double c;
    double d;
    double e;
    double f;
};

struct skia_point {
    float x;
    float y;
};

struct skiac_surface_data {
    uint8_t *ptr;
    uint32_t size;
};

// The same order as in core/SkPaint.h
enum class PaintStyle {
    Fill,
    Stroke,
};

// The same order as in core/SkPaint.h
enum class StrokeCap {
    Butt,
    Round,
    Square,
};

// The same order as in core/SkPaint.h
enum class StrokeJoin {
    Miter,
    Round,
    Bevel,
};

// The same order as in core/SkPath.h
enum class FillType {
    Winding,
    EvenOdd,
};

// The same order as in core/SkTileMode.h
enum class TileMode {
    Clamp,
    Repeat,
    Mirror,
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

// The same order as in core/SkFilterQuality.h
enum class FilterQuality {
    None,
    Low,
    Medium,
    High,
};

extern "C" {

// Surface
skiac_surface* skiac_surface_create_rgba_premultiplied(int width, int height);
skiac_surface* skiac_surface_create_rgba(int width, int height);
void skiac_surface_destroy(skiac_surface* c_surface);
skiac_surface* skiac_surface_copy_rgba(skiac_surface *c_surface, uint32_t x, uint32_t y, uint32_t width, uint32_t height);
bool skiac_surface_save(skiac_surface* c_surface, const char *path);
skiac_canvas* skiac_surface_get_canvas(skiac_surface* c_surface);
int skiac_surface_get_width(skiac_surface *c_surface);
int skiac_surface_get_height(skiac_surface *c_surface);
void skiac_surface_read_pixels(skiac_surface* c_surface, skiac_surface_data* data);
bool skiac_is_surface_bgra();

// Canvas
void skiac_canvas_clear(skiac_canvas* c_canvas, uint32_t color);
void skiac_canvas_flush(skiac_canvas* c_canvas);
void skiac_canvas_set_matrix(skiac_canvas* c_canvas, skiac_matrix *c_matrix);
void skiac_canvas_concat(skiac_canvas* c_canvas, skiac_matrix* c_matrix);
void skiac_canvas_scale(skiac_canvas* c_canvas, double sx, double sy);
void skiac_canvas_translate(skiac_canvas* c_canvas, double dx, double dy);
skiac_matrix* skiac_canvas_get_total_matrix(skiac_canvas* c_canvas);
void skiac_canvas_draw_path(skiac_canvas* c_canvas, skiac_path* c_path, skiac_paint* c_paint);
void skiac_canvas_draw_rect(skiac_canvas* c_canvas, double x, double y, double w, double h, skiac_paint* c_paint);
void skiac_canvas_draw_surface(skiac_canvas* c_canvas, skiac_surface* c_surface, double left, double top,
                               uint8_t alpha, BlendMode blend_mode, FilterQuality filter_quality);
void skiac_canvas_draw_surface_rect(skiac_canvas* c_canvas, skiac_surface* c_surface,
                                    double x, double y, double w, double h,
                                    FilterQuality filter_quality);
void skiac_canvas_reset_matrix(skiac_canvas* c_canvas);
void skiac_canvas_clip_rect(skiac_canvas* c_canvas, double x, double y, double w, double h);
void skiac_canvas_save(skiac_canvas* c_canvas);
void skiac_canvas_restore(skiac_canvas* c_canvas);

// Matrix
skiac_matrix *skiac_matrix_create();
skiac_matrix *skiac_matrix_create_from(double a, double b, double c, double d, double e, double f);
skiac_matrix *skiac_matrix_create_inverse(skiac_matrix *c_matrix);
skia_matrix skiac_matrix_get_data(skiac_matrix *c_matrix);
void skiac_matrix_destroy(skiac_matrix *c_matrix);

// Paint
skiac_paint* skiac_paint_create();
void skiac_paint_destroy(skiac_paint* c_paint);
void skiac_paint_set_style(skiac_paint* c_paint, PaintStyle style);
void skiac_paint_set_color(skiac_paint* c_paint, uint8_t r, uint8_t g, uint8_t b, uint8_t a);
void skiac_paint_set_alpha(skiac_paint* c_paint, uint8_t a);
void skiac_paint_set_anti_alias(skiac_paint* c_paint, bool aa);
void skiac_paint_set_blend_mode(skiac_paint* c_paint, BlendMode blend_mode);
void skiac_paint_set_shader(skiac_paint* c_paint, skiac_shader* c_shader);
void skiac_paint_set_stroke_width(skiac_paint* c_paint, double width);
void skiac_paint_set_stroke_cap(skiac_paint* c_paint, StrokeCap cap);
void skiac_paint_set_stroke_join(skiac_paint* c_paint, StrokeJoin join);
void skiac_paint_set_stroke_miter(skiac_paint* c_paint, float miter);
void skiac_paint_set_path_effect(skiac_paint* c_paint, skiac_path_effect* c_path_effect);

// Path
skiac_path* skiac_path_create();
void skiac_path_destroy(skiac_path* c_path);
void skiac_path_set_fill_type(skiac_path* c_path, FillType type);
void skiac_path_move_to(skiac_path* c_path, double x, double y);
void skiac_path_line_to(skiac_path* c_path, double x, double y);
void skiac_path_cubic_to(skiac_path* c_path, double x1, double y1, double x2, double y2, double x3, double y3);
void skiac_path_close(skiac_path* c_path);

// PathEffect
skiac_path_effect* skiac_path_effect_make_dash_path(const float* intervals, int count, float phase);
void skiac_path_effect_destroy(skiac_path_effect* c_path_effect);

// Shader
skiac_shader* skiac_shader_make_linear_gradient(const skia_point* points, const uint32_t* colors,
                                                const float* positions, int count, TileMode tile_mode,
                                                uint32_t flags, skiac_matrix *c_matrix);

skiac_shader* skiac_shader_make_two_point_conical_gradient(
    const skia_point start_point, float start_radius,
    const skia_point end_point, float end_radius,
    const uint32_t* colors, const float* positions,
    int count, TileMode tile_mode,
    uint32_t flags, skiac_matrix *c_matrix);

skiac_shader* skiac_shader_make_from_surface_image(skiac_surface* c_surface, skiac_matrix *c_matrix);

void skiac_shader_destroy(skiac_shader* c_shader);
}

#endif // SKIA_CAPI_H
