#include <assert.h>

#ifdef SKIA_VER_M58
#include <SkCanvas.h>
#include <SkGraphics.h>
#include <SkPaint.h>
#include <SkSurface.h>
#include <SkDashPathEffect.h>
#include <SkGradientShader.h>
#else
#include <include/core/SkCanvas.h>
#include <include/core/SkGraphics.h>
#include <include/core/SkPaint.h>
#include <include/core/SkSurface.h>
#include <include/effects/SkDashPathEffect.h>
#include <include/effects/SkGradientShader.h>
#include <include/encode/SkPngEncoder.h>
#endif

#include <math.h>

#include "skia_capi.hpp"

#define SURFACE_CAST reinterpret_cast<SkSurface*>(c_surface)
#define CANVAS_CAST reinterpret_cast<SkCanvas*>(c_canvas)
#define PAINT_CAST reinterpret_cast<SkPaint*>(c_paint)
#define PATH_CAST reinterpret_cast<SkPath*>(c_path)
#define MATRIX_CAST reinterpret_cast<SkMatrix*>(c_matrix)

static SkBlendMode blendModes_[static_cast<int>(BlendMode::__Size)] = {
    SkBlendMode::kClear,
    SkBlendMode::kSrcOver,
    SkBlendMode::kDstOver,
    SkBlendMode::kSrcIn,
    SkBlendMode::kDstIn,
    SkBlendMode::kSrcOut,
    SkBlendMode::kDstOut,
    SkBlendMode::kSrcATop,
    SkBlendMode::kXor,
    SkBlendMode::kMultiply,
    SkBlendMode::kScreen,
    SkBlendMode::kDarken,
    SkBlendMode::kLighten,
};

extern "C" {

// Surface

static SkSurface* skiac_surface_create(int width, int height, SkAlphaType alphaType)
{
    // Init() is indempotent, so can be called more than once with no adverse effect.
    SkGraphics::Init();

    SkImageInfo info = SkImageInfo::Make(width, height, kN32_SkColorType, alphaType);
    sk_sp<SkSurface> surface = SkSurface::MakeRaster(info);

    // The surface ref count will equal one after the pointer is returned.
    return surface.release();
}

skiac_surface* skiac_surface_create_rgba_premultiplied(int width, int height)
{
    return reinterpret_cast<skiac_surface*>(skiac_surface_create(width, height, kPremul_SkAlphaType));
}

skiac_surface* skiac_surface_create_rgba(int width, int height)
{
    return reinterpret_cast<skiac_surface*>(skiac_surface_create(width, height, kUnpremul_SkAlphaType));
}

SkSurface* skiac_surface_create_data(sk_sp<SkData> data)
{
    SkSurface* surface = nullptr;

    sk_sp<SkImage> image = SkImage::MakeFromEncoded(data);
	if (image) {
        surface = skiac_surface_create(image->width(), image->height(), kPremul_SkAlphaType);
	}

    if (surface) {
        surface->getCanvas()->drawImage(image, 0, 0);
    }

    return surface;
}

bool skiac_surface_save(skiac_surface* c_surface, const char *path)
{
    sk_sp<SkImage> image = SURFACE_CAST->makeImageSnapshot();
#ifdef SKIA_VER_M58
    SkData *data = image->encode(SkEncodedImageFormat::kPNG, 0);
    if (data) {
        SkFILEWStream stream(path);
        if (stream.write(data->data(), data->size())) {
            stream.flush();
            return true;
        }
    }
#else
    SkPngEncoder::Options opt;
    opt.fZLibLevel = 2; // Use a lower ratio to speed up compression.

    SkPixmap pixmap;
    if (SURFACE_CAST->getCanvas()->peekPixels(&pixmap)) {
        SkFILEWStream stream(path);
        SkPngEncoder::Encode(&stream, pixmap, opt);
        return true;
    }
#endif

    return false;
}

void skiac_surface_destroy(skiac_surface* c_surface)
{
    // SkSurface is ref counted.
    SkSurface* surface = reinterpret_cast<SkSurface*>(c_surface);
    SkSafeUnref(surface);
}

skiac_surface* skiac_surface_copy_rgba(skiac_surface *c_surface, uint32_t x, uint32_t y, uint32_t width, uint32_t height)
{
    // x, y, width, height are source rectangle coordinates.
    SkSurface* copy = skiac_surface_create((int)width, (int)height, kUnpremul_SkAlphaType);

    SkPaint paint;
    paint.setFilterQuality(SkFilterQuality::kLow_SkFilterQuality);
    paint.setAlpha(SK_AlphaOPAQUE);

    // The original surface draws itself to the copy's canvas.
    SURFACE_CAST->draw(copy->getCanvas(), -(SkScalar)x, -(SkScalar)y, &paint);

    return reinterpret_cast<skiac_surface*>(copy);
}

int skiac_surface_get_width(skiac_surface* c_surface)
{
    return SURFACE_CAST->width();
}

int skiac_surface_get_height(skiac_surface* c_surface)
{
    return SURFACE_CAST->height();
}

skiac_canvas* skiac_surface_get_canvas(skiac_surface* c_surface)
{
    return reinterpret_cast<skiac_canvas*>(SURFACE_CAST->getCanvas());
}

void skiac_surface_read_pixels(skiac_surface* c_surface, skiac_surface_data* data)
{
    data->ptr = nullptr;
    data->size = 0;

    SkPixmap pixmap;
    if (SURFACE_CAST->getCanvas()->peekPixels(&pixmap)) {
        data->ptr = static_cast<uint8_t*>(pixmap.writable_addr());
#ifdef SKIA_VER_M58
        data->size = static_cast<uint32_t>(pixmap.getSafeSize());
#else
        data->size = static_cast<uint32_t>(pixmap.computeByteSize());
#endif
    }
}

bool skiac_is_surface_bgra()
{
    return kN32_SkColorType == kBGRA_8888_SkColorType;
}

// Canvas

void skiac_canvas_clear(skiac_canvas* c_canvas, uint32_t color)
{
    CANVAS_CAST->clear(static_cast<SkColor>(color));
}

void skiac_canvas_flush(skiac_canvas* c_canvas)
{
    CANVAS_CAST->flush();
}

void skiac_canvas_set_matrix(skiac_canvas* c_canvas, skiac_matrix* c_matrix)
{
    CANVAS_CAST->setMatrix(*MATRIX_CAST);
}

void skiac_canvas_concat(skiac_canvas* c_canvas, skiac_matrix* c_matrix)
{
    CANVAS_CAST->concat(*MATRIX_CAST);
}

void skiac_canvas_scale(skiac_canvas* c_canvas, double sx, double sy)
{
    CANVAS_CAST->scale((SkScalar)sx, (SkScalar)sy);
}

void skiac_canvas_translate(skiac_canvas* c_canvas, double dx, double dy)
{
    CANVAS_CAST->translate((SkScalar)dx, (SkScalar)dy);
}

skiac_matrix* skiac_canvas_get_total_matrix(skiac_canvas* c_canvas)
{
    SkMatrix* matrix = new SkMatrix();
    *matrix = CANVAS_CAST->getTotalMatrix();
    return reinterpret_cast<skiac_matrix*>(matrix);
}

void skiac_canvas_draw_path(skiac_canvas* c_canvas, skiac_path* c_path, skiac_paint* c_paint)
{
    CANVAS_CAST->drawPath(*PATH_CAST, *PAINT_CAST);
}

void skiac_canvas_draw_rect(skiac_canvas* c_canvas, double x, double y, double w, double h, skiac_paint* c_paint)
{
    SkRect rect = SkRect::MakeXYWH((SkScalar)x, (SkScalar)y, (SkScalar)w, (SkScalar)h);
    CANVAS_CAST->drawRect(rect, *PAINT_CAST);
}

void skiac_canvas_draw_surface(skiac_canvas* c_canvas, skiac_surface* c_surface, double left,
                               double top, uint8_t alpha, BlendMode blend_mode,
                               FilterQuality filter_quality)
{
    sk_sp<SkImage> image = SURFACE_CAST->makeImageSnapshot();
    SkPaint paint;
    paint.setFilterQuality((SkFilterQuality)filter_quality);
    paint.setAlpha(alpha);
    paint.setBlendMode(blendModes_[static_cast<int>(blend_mode)]);
    CANVAS_CAST->drawImage(image, (SkScalar)left, (SkScalar)top, &paint);
}

void skiac_canvas_draw_surface_rect(skiac_canvas* c_canvas, skiac_surface* c_surface,
                                    double x, double y, double w, double h,
                                    FilterQuality filter_quality)
{
    sk_sp<SkImage> image = SURFACE_CAST->makeImageSnapshot();
    SkPaint paint;
    paint.setFilterQuality((SkFilterQuality)filter_quality);
    SkRect src = SkRect::MakeXYWH(0, 0, (SkScalar)image->width(), (SkScalar)image->height());
    SkRect dst = SkRect::MakeXYWH((SkScalar)x, (SkScalar)y, (SkScalar)w, (SkScalar)h);
    CANVAS_CAST->drawImageRect(image, src, dst, &paint);
}

void skiac_canvas_reset_matrix(skiac_canvas* c_canvas)
{
    CANVAS_CAST->resetMatrix();
}

void skiac_canvas_clip_rect(skiac_canvas* c_canvas, double x, double y, double w, double h)
{
    SkRect rect = SkRect::MakeXYWH((SkScalar)x, (SkScalar)y, (SkScalar)w, (SkScalar)h);
    CANVAS_CAST->clipRect(rect, true);
}

void skiac_canvas_save(skiac_canvas* c_canvas)
{
    CANVAS_CAST->save();
}

void skiac_canvas_restore(skiac_canvas* c_canvas)
{
    CANVAS_CAST->restore();
}

// SkMatrix

skiac_matrix *skiac_matrix_create()
{
    SkMatrix* matrix = new SkMatrix();
    matrix->reset();
    return reinterpret_cast<skiac_matrix*>(matrix);
}

skiac_matrix *skiac_matrix_create_from(double a, double b, double c, double d, double e, double f)
{
    SkMatrix* matrix = new SkMatrix();
    matrix->setAll((SkScalar)a, (SkScalar)c, (SkScalar)e, (SkScalar)b, (SkScalar)d, (SkScalar)f, 0.0f, 0.0f, 1.0f);
    return reinterpret_cast<skiac_matrix*>(matrix);
}

skiac_matrix *skiac_matrix_create_inverse(skiac_matrix *c_matrix)
{
    SkMatrix* inverse = new SkMatrix();
    if (MATRIX_CAST->invert(inverse)) {
        return reinterpret_cast<skiac_matrix*>(inverse);
    } else {
        return nullptr;
    }
}

skia_matrix skiac_matrix_get_data(skiac_matrix *c_matrix)
{
    const auto mat = MATRIX_CAST;
    skia_matrix raw_mat;
    raw_mat.a = (double)mat->getScaleX();
    raw_mat.b = (double)mat->getSkewY();
    raw_mat.c = (double)mat->getSkewX();
    raw_mat.d = (double)mat->getScaleY();
    raw_mat.e = (double)mat->getTranslateX();
    raw_mat.f = (double)mat->getTranslateY();

    return raw_mat;
}

void skiac_matrix_destroy(skiac_matrix *c_matrix)
{
    // SkMatrix is NOT ref counted
    delete MATRIX_CAST;
}

// Paint

skiac_paint* skiac_paint_create()
{
    SkPaint* paint = new SkPaint();
    return reinterpret_cast<skiac_paint*>(paint);
}

void skiac_paint_destroy(skiac_paint* c_paint)
{
    SkPaint* paint = PAINT_CAST;

    // Setting these references to nullptr should decrement their ref count.
    paint->setShader(nullptr);
    paint->setPathEffect(nullptr);

    // SkPaint is not ref counted, so explicitly delete.
    delete paint;
}

void skiac_paint_set_color(skiac_paint* c_paint, uint8_t r, uint8_t g, uint8_t b, uint8_t a)
{
    PAINT_CAST->setARGB(a, r, g, b);
}

void skiac_paint_set_alpha(skiac_paint* c_paint, uint8_t a)
{
    PAINT_CAST->setAlpha(a);
}

void skiac_paint_set_anti_alias(skiac_paint* c_paint, bool aa)
{
    PAINT_CAST->setAntiAlias(aa);
}

void skiac_paint_set_blend_mode(skiac_paint* c_paint, BlendMode blend_mode)
{
    PAINT_CAST->setBlendMode(blendModes_[static_cast<int>(blend_mode)]);
}

void skiac_paint_set_shader(skiac_paint* c_paint, skiac_shader* c_shader)
{
    sk_sp<SkShader> shader(reinterpret_cast<SkShader*>(c_shader));
    PAINT_CAST->setShader(shader);
}

void skiac_paint_set_style(skiac_paint* c_paint, PaintStyle style)
{
    PAINT_CAST->setStyle((SkPaint::Style)style);
}

void skiac_paint_set_stroke_width(skiac_paint* c_paint, double width)
{
    PAINT_CAST->setStrokeWidth((SkScalar)width);
}

void skiac_paint_set_stroke_cap(skiac_paint* c_paint, StrokeCap cap)
{
    PAINT_CAST->setStrokeCap((SkPaint::Cap)cap);
}

void skiac_paint_set_stroke_join(skiac_paint* c_paint, StrokeJoin join)
{
    PAINT_CAST->setStrokeJoin((SkPaint::Join)join);
}

void skiac_paint_set_stroke_miter(skiac_paint* c_paint, float miter)
{
    PAINT_CAST->setStrokeMiter(miter);
}

void skiac_paint_set_path_effect(skiac_paint* c_paint, skiac_path_effect* c_path_effect)
{
    sk_sp<SkPathEffect> pathEffect(reinterpret_cast<SkPathEffect*>(c_path_effect));
    PAINT_CAST->setPathEffect(pathEffect);
}

// Path

skiac_path* skiac_path_create()
{
    return reinterpret_cast<skiac_path*>(new SkPath());
}

void skiac_path_destroy(skiac_path* c_path)
{
    // SkPath is NOT ref counted
    delete PATH_CAST;
}

void skiac_path_set_fill_type(skiac_path* c_path, FillType type)
{
    PATH_CAST->setFillType((SkPath::FillType)type);
}

void skiac_path_move_to(skiac_path* c_path, double x, double y)
{
    PATH_CAST->moveTo((SkScalar)x, (SkScalar)y);
}

void skiac_path_line_to(skiac_path* c_path, double x, double y)
{
    PATH_CAST->lineTo((SkScalar)x, (SkScalar)y);
}

void skiac_path_cubic_to(skiac_path* c_path, double x1, double y1, double x2, double y2, double x3, double y3)
{
    PATH_CAST->cubicTo((SkScalar)x1, (SkScalar)y1, (SkScalar)x2, (SkScalar)y2, (SkScalar)x3, (SkScalar)y3);
}

void skiac_path_close(skiac_path* c_path)
{
    PATH_CAST->close();
}

// PathEffect

skiac_path_effect* skiac_path_effect_make_dash_path(const float* intervals, int count, float phase)
{
    SkPathEffect* effect = SkDashPathEffect::Make(intervals, count, phase).release();
    effect->ref();
    return reinterpret_cast<skiac_path_effect*>(effect);
}

void skiac_path_effect_destroy(skiac_path_effect* c_path_effect)
{
    // SkPathEffect is ref counted.
    SkPathEffect* effect = reinterpret_cast<SkPathEffect*>(c_path_effect);
    SkSafeUnref(effect);
}

// Shader

skiac_shader* skiac_shader_make_linear_gradient(
    const skia_point* c_points, const uint32_t* colors,
    const float* positions, int count, TileMode tile_mode,
    uint32_t flags, skiac_matrix *c_matrix)
{
    const SkPoint* points = reinterpret_cast<const SkPoint*>(c_points);

#ifdef SKIA_VER_M58
    auto skia_tile_mode = (SkShader::TileMode)tile_mode;
#else
    auto skia_tile_mode = (SkTileMode)tile_mode;
#endif

    SkShader* shader = SkGradientShader::MakeLinear(
        points, colors, positions, count, skia_tile_mode,
        flags, MATRIX_CAST
    ).release();
    shader->ref();

    return reinterpret_cast<skiac_shader*>(shader);
}

skiac_shader* skiac_shader_make_two_point_conical_gradient(
    const skia_point c_start_point, SkScalar start_radius,
    const skia_point c_end_point, SkScalar end_radius,
    const uint32_t* colors, const SkScalar* positions,
    int count, TileMode tile_mode,
    uint32_t flags, skiac_matrix *c_matrix)
{
    const SkPoint startPoint = { c_start_point.x, c_start_point.y };
    const SkPoint endPoint = { c_end_point.x, c_end_point.y };

#ifdef SKIA_VER_M58
    auto skia_tile_mode = (SkShader::TileMode)tile_mode;
#else
    auto skia_tile_mode = (SkTileMode)tile_mode;
#endif

    SkShader* shader = SkGradientShader::MakeTwoPointConical(
        startPoint, start_radius,
        endPoint, end_radius,
        colors, positions, count, skia_tile_mode,
        flags, MATRIX_CAST
    ).release();
    shader->ref();

    return reinterpret_cast<skiac_shader*>(shader);
}

skiac_shader* skiac_shader_make_from_surface_image(skiac_surface* c_surface, skiac_matrix* c_matrix)
{
#ifdef SKIA_VER_M58
    auto skia_tile_mode = SkShader::TileMode::kRepeat_TileMode;
#else
    auto skia_tile_mode = SkTileMode::kRepeat;
#endif

    sk_sp<SkImage> image = SURFACE_CAST->makeImageSnapshot();
    SkShader* shader = image->makeShader(skia_tile_mode, skia_tile_mode, MATRIX_CAST).release();
    shader->ref();

    return reinterpret_cast<skiac_shader*>(shader);
}

void skiac_shader_destroy(skiac_shader* c_shader)
{
    // SkShader is ref counted.
    SkShader* shader = reinterpret_cast<SkShader*>(c_shader);
    SkSafeUnref(shader);
}

}
