#ifndef QT_CAPI_H
#define QT_CAPI_H

#include <stdint.h>

#define INIT_STRUCT(x) \
    struct x; \
    typedef struct x x;

INIT_STRUCT(qtc_qimage)
INIT_STRUCT(qtc_qpainter)
INIT_STRUCT(qtc_qpainterpath)
INIT_STRUCT(qtc_qtransform)
INIT_STRUCT(qtc_qpen)
INIT_STRUCT(qtc_qbrush)
INIT_STRUCT(qtc_qlineargradient)
INIT_STRUCT(qtc_qradialgradient)

#undef INIT_STRUCT

struct qtc_transform {
    double a;
    double b;
    double c;
    double d;
    double e;
    double f;
};

// A direct copy from qnamespace.h.
enum PenCapStyle {
    FlatCap   = 0x00,
    SquareCap = 0x10,
    RoundCap  = 0x20,
};

// A direct copy from qnamespace.h.
enum PenJoinStyle {
    BevelJoin = 0x40,
    RoundJoin = 0x80,
    MiterJoin = 0x100,
};

// A direct copy from qnamespace.h.
enum FillRule {
    OddEvenFill,
    WindingFill,
};

// A direct copy from qbrush.h.
enum Spread {
    PadSpread,
    ReflectSpread,
    RepeatSpread,
};

// TODO: remove prefix somehow
// A direct copy from qpainter.h.
enum CompositionMode {
    CompositionMode_SourceOver,
    CompositionMode_DestinationOver,
    CompositionMode_Clear,
    CompositionMode_Source,
    CompositionMode_Destination,
    CompositionMode_SourceIn,
    CompositionMode_DestinationIn,
    CompositionMode_SourceOut,
    CompositionMode_DestinationOut,
    CompositionMode_SourceAtop,
    CompositionMode_DestinationAtop,
    CompositionMode_Xor,

    // SVG 1.2 blend modes
    CompositionMode_Plus,
    CompositionMode_Multiply,
    CompositionMode_Screen,
    CompositionMode_Overlay,
    CompositionMode_Darken,
    CompositionMode_Lighten,
    CompositionMode_ColorDodge,
    CompositionMode_ColorBurn,
    CompositionMode_HardLight,
    CompositionMode_SoftLight,
    CompositionMode_Difference,
    CompositionMode_Exclusion,
};

enum AspectRatioMode {
    IgnoreAspectRatio,
    KeepAspectRatio,
    KeepAspectRatioByExpanding,
};

extern "C" {

// QImage
qtc_qimage* qtc_qimage_create_rgba_premultiplied(uint32_t width, uint32_t height);
qtc_qimage* qtc_qimage_create_rgba(uint32_t width, uint32_t height);
uint8_t* qtc_qimage_get_data(qtc_qimage *c_img);
uint32_t qtc_qimage_get_size_in_bytes(qtc_qimage *c_img);
qtc_qimage* qtc_qimage_resize(qtc_qimage *c_img, uint32_t width, uint32_t height, AspectRatioMode ratio,
                              bool smoothTransformation);
qtc_qimage* qtc_qimage_copy(qtc_qimage *c_img, uint32_t x, uint32_t y, uint32_t width, uint32_t height);
void qtc_qimage_fill(qtc_qimage *c_img, uint8_t r, uint8_t g, uint8_t b, uint8_t a);
qtc_qimage *qtc_qimage_to_rgba(qtc_qimage *c_img);
uint32_t qtc_qimage_get_width(qtc_qimage *c_img);
uint32_t qtc_qimage_get_height(qtc_qimage *c_img);
bool qtc_qimage_save(qtc_qimage *c_img, const char *path);
void qtc_qimage_destroy(qtc_qimage *c_img);


// QPainter
qtc_qpainter* qtc_qpainter_create(qtc_qimage *c_img);
void qtc_qpainter_set_antialiasing(qtc_qpainter *c_p, bool flag);
void qtc_qpainter_set_smooth_pixmap_transform(qtc_qpainter *c_p, bool flag);
void qtc_qpainter_set_pen(qtc_qpainter *c_p, qtc_qpen *c_pen);
void qtc_qpainter_reset_pen(qtc_qpainter *c_p);
void qtc_qpainter_set_brush(qtc_qpainter *c_p, qtc_qbrush *c_brush);
void qtc_qpainter_reset_brush(qtc_qpainter *c_p);
void qtc_qpainter_set_opacity(qtc_qpainter *c_p, double opacity);
void qtc_qpainter_draw_path(qtc_qpainter *c_p, qtc_qpainterpath *c_pp);
void qtc_qpainter_draw_image(qtc_qpainter *c_p, double x, double y, qtc_qimage *c_img);
void qtc_qpainter_draw_image_rect(qtc_qpainter *c_p, double x, double y, double w, double h, qtc_qimage *c_img);
void qtc_qpainter_draw_text(qtc_qpainter *c_p, double x, double y, const char *c_text);
void qtc_qpainter_draw_rect(qtc_qpainter *c_p, double x, double y, double w, double h);
void qtc_qpainter_translate(qtc_qpainter *c_p, double tx, double ty);
void qtc_qpainter_scale(qtc_qpainter *c_p, double sx, double sy);
qtc_qtransform* qtc_qpainter_get_transform(qtc_qpainter *c_p);
void qtc_qpainter_set_transform(qtc_qpainter *c_p, qtc_qtransform *q_ts, bool combine);
void qtc_qpainter_set_clip_rect(qtc_qpainter *c_p, double x, double y, double w, double h);
void qtc_qpainter_set_clip_path(qtc_qpainter *c_p, qtc_qpainterpath *c_pp);
void qtc_qpainter_reset_clip_path(qtc_qpainter *c_p);
void qtc_qpainter_set_composition_mode(qtc_qpainter *c_p, CompositionMode mode);
void qtc_qpainter_end(qtc_qpainter *c_p);
void qtc_qpainter_destroy(qtc_qpainter *c_p);


// QPainterPath
qtc_qpainterpath* qtc_qpainterpath_create();
void qtc_qpainterpath_move_to(qtc_qpainterpath *c_pp, double x, double y);
void qtc_qpainterpath_line_to(qtc_qpainterpath *c_pp, double x, double y);
void qtc_qpainterpath_curve_to(qtc_qpainterpath *c_pp, double x1, double y1, double x2, double y2,
                               double x, double y);
void qtc_qpainterpath_close_path(qtc_qpainterpath *c_pp);
void qtc_qpainterpath_set_fill_rule(qtc_qpainterpath *c_pp, FillRule rule);
void qtc_qpainterpath_destroy(qtc_qpainterpath *c_pp);


// QTransform
qtc_qtransform* qtc_qtransform_create();
qtc_qtransform* qtc_qtransform_create_from(double a, double b, double c,
                                           double d, double e, double f);
qtc_transform qtc_qtransform_get_data(qtc_qtransform *c_ts);
void qtc_qtransform_destroy(qtc_qtransform *c_ts);


// QPen
qtc_qpen* qtc_qpen_create();
void qtc_qpen_set_color(qtc_qpen *c_pen, uint8_t r, uint8_t g, uint8_t b, uint8_t a);
void qtc_qpen_set_brush(qtc_qpen *c_pen, qtc_qbrush *c_brush);
void qtc_qpen_set_line_cap(qtc_qpen *c_pen, PenCapStyle s);
void qtc_qpen_set_line_join(qtc_qpen *c_pen, PenJoinStyle s);
void qtc_qpen_set_width(qtc_qpen *c_pen, double width);
void qtc_qpen_set_miter_limit(qtc_qpen *c_pen, double limit);
void qtc_qpen_set_dash_offset(qtc_qpen *c_pen, double offset);
void qtc_qpen_set_dash_array(qtc_qpen *c_pen, const double *array, int len);
void qtc_qpen_destroy(qtc_qpen *c_pen);


// QBrush
qtc_qbrush* qtc_qbrush_create();
void qtc_qbrush_set_color(qtc_qbrush *c_brush, uint8_t r, uint8_t g, uint8_t b, uint8_t a);
void qtc_qbrush_set_linear_gradient(qtc_qbrush *c_brush, qtc_qlineargradient *c_lg);
void qtc_qbrush_set_radial_gradient(qtc_qbrush *c_brush, qtc_qradialgradient *c_rg);
void qtc_qbrush_set_pattern(qtc_qbrush *c_brush, qtc_qimage *c_img);
void qtc_qbrush_set_transform(qtc_qbrush *c_brush, qtc_qtransform *c_ts);
void qtc_qbrush_destroy(qtc_qbrush *c_brush);


// QLinearGradient
qtc_qlineargradient* qtc_qlineargradient_create(double x1, double y1, double x2, double y2);
void qtc_qlineargradient_set_color_at(qtc_qlineargradient *c_lg, double offset,
                                      uint8_t r, uint8_t g, uint8_t b, uint8_t a);
void qtc_qlineargradient_set_spread(qtc_qlineargradient *c_lg, Spread s);
void qtc_qlineargradient_destroy(qtc_qlineargradient *c_lg);


// QRadialGradient
qtc_qradialgradient* qtc_qradialgradient_create(double cx, double cy, double fx, double fy, double r);
void qtc_qradialgradient_set_color_at(qtc_qradialgradient *c_rg, double offset,
                                      uint8_t r, uint8_t g, uint8_t b, uint8_t a);
void qtc_qradialgradient_set_spread(qtc_qradialgradient *c_rg, Spread s);
void qtc_qradialgradient_destroy(qtc_qradialgradient *c_rg);
}

#endif // QT_CAPI_H
