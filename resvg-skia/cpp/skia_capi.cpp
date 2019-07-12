#include <assert.h>
#include <include/core/SkBlendMode.h>
#include <include/core/SkCanvas.h>
#include <include/core/SkGraphics.h>
#include <include/core/SkPaint.h>
#include <include/core/SkPath.h>
#include <include/core/SkShader.h>
#include <include/core/SkSurface.h>
#include <include/core/SkTileMode.h>
#include <include/effects/SkDashPathEffect.h>
#include <include/effects/SkGradientShader.h>

#include "skia_capi.hpp"

static SkPaint::Style paintStyles_[static_cast<int>(PaintStyle::__Size)] = {
    SkPaint::Style::kFill_Style,
    SkPaint::Style::kStroke_Style,
};

static SkPaint::Cap strokeCaps_[static_cast<int>(StrokeCap::__Size)] = {
    SkPaint::Cap::kButt_Cap,
    SkPaint::Cap::kRound_Cap,
    SkPaint::Cap::kSquare_Cap,
};

static SkPaint::Join strokeJoins_[static_cast<int>(StrokeJoin::__Size)] = {
    SkPaint::Join::kMiter_Join,
    SkPaint::Join::kRound_Join,
    SkPaint::Join::kBevel_Join,
};

static SkTileMode tileModes_[static_cast<int>(TileMode::__Size)] = {
    SkTileMode::kClamp,
    SkTileMode::kRepeat,
    SkTileMode::kMirror,
};

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

skiac_surface* skiac_surface_create_from_image_data(const void* buffer, uint32_t size)
{
    sk_sp<SkData> data(SkData::MakeWithCopy(buffer, size));
    return reinterpret_cast<skiac_surface*>(skiac_surface_create_data(data));
}

skiac_surface* skiac_surface_create_from_file(const char *path)
{
    sk_sp<SkData> data = SkData::MakeFromFileName(path);
    if (data) {
        return reinterpret_cast<skiac_surface*>(skiac_surface_create_data(data));
    }
    return nullptr;
}

bool skiac_surface_save(skiac_surface* c_surface, const char *path)
{
    bool success = false;

    SkSurface* surface = reinterpret_cast<SkSurface*>(c_surface);
    sk_sp<SkImage> image = surface->makeImageSnapshot();
    sk_sp<SkData> data = image->encodeToData(SkEncodedImageFormat::kPNG, 0);
    if (data) {
        SkFILEWStream stream(path);
        if (stream.write(data->data(), data->size())) {
            stream.flush();
            success = true;
        }
    }

    return success;
}

void skiac_surface_destroy(skiac_surface* c_surface)
{
    // SkSurface is ref counted.
    SkSurface* surface = reinterpret_cast<SkSurface*>(c_surface);
    SkSafeUnref(surface);
}

skiac_surface* skiac_surface_copy_rgba(skiac_surface *c_surface, uint32_t x, uint32_t y, uint32_t width, uint32_t height)
{
    SkSurface* surface = reinterpret_cast<SkSurface*>(c_surface);

    // x, y, width, height are source rectangle coordinates.
    SkSurface* copy = skiac_surface_create((int)width, (int)height, kUnpremul_SkAlphaType);

    SkPaint paint;
    paint.setFilterQuality(SkFilterQuality::kLow_SkFilterQuality);
    paint.setAlpha(SK_AlphaOPAQUE);

    // The original surface draws itself to the copy's canvas.
    surface->draw(copy->getCanvas(), -(SkScalar)x, -(SkScalar)y, &paint);

    return reinterpret_cast<skiac_surface*>(copy);
}

int skiac_surface_get_width(const skiac_surface* c_surface)
{
    const SkSurface* surface = reinterpret_cast<const SkSurface*>(c_surface);
    return surface->width();
}

int skiac_surface_get_height(const skiac_surface* c_surface)
{
    const SkSurface* surface = reinterpret_cast<const SkSurface*>(c_surface);
    return surface->height();
}

skiac_canvas* skiac_surface_get_canvas(skiac_surface* c_surface)
{
    SkSurface* surface = reinterpret_cast<SkSurface*>(c_surface);
    return reinterpret_cast<skiac_canvas*>(surface->getCanvas());
}

bool skiac_surface_read_pixels(skiac_surface* c_surface, skiac_surface_data* data)
{
    bool success = true;

    SkSurface* surface = reinterpret_cast<SkSurface*>(c_surface);

    data->ptr = nullptr;
    data->size = 0;

    SkPixmap pixmap;
	if (surface->getCanvas()->peekPixels(&pixmap)) {
        data->ptr = static_cast<char*>(pixmap.writable_addr());
        data->size = static_cast<uint32_t>(pixmap.computeByteSize());
	}

    return success;
}

// Canvas

void skiac_canvas_clear(skiac_canvas* c_canvas, uint32_t color)
{
    SkCanvas* canvas = reinterpret_cast<SkCanvas*>(c_canvas);
    canvas->clear(static_cast<SkColor>(color));
}

void skiac_canvas_flush(skiac_canvas* c_canvas)
{
    SkCanvas* canvas = reinterpret_cast<SkCanvas*>(c_canvas);
    canvas->flush();
}

void skiac_canvas_set_matrix(skiac_canvas* c_canvas, skiac_matrix* c_mat)
{
    SkCanvas* canvas = reinterpret_cast<SkCanvas*>(c_canvas);
    const SkMatrix* matrix = reinterpret_cast<const SkMatrix*>(c_mat);
    canvas->setMatrix(*matrix);
}

void skiac_canvas_concat(skiac_canvas* c_canvas, skiac_matrix* c_mat)
{
    SkCanvas* canvas = reinterpret_cast<SkCanvas*>(c_canvas);
    const SkMatrix* matrix = reinterpret_cast<const SkMatrix*>(c_mat);
    canvas->concat(*matrix);
}

void skiac_canvas_scale(skiac_canvas* c_canvas, double sx, double sy)
{
    SkCanvas* canvas = reinterpret_cast<SkCanvas*>(c_canvas);
    canvas->scale((SkScalar)sx, (SkScalar)sy);
}

void skiac_canvas_translate(skiac_canvas* c_canvas, double dx, double dy)
{
    SkCanvas* canvas = reinterpret_cast<SkCanvas*>(c_canvas);
    canvas->translate((SkScalar)dx, (SkScalar)dy);
}

skiac_matrix* skiac_canvas_get_total_matrix(skiac_canvas* c_canvas)
{
    SkCanvas* canvas = reinterpret_cast<SkCanvas*>(c_canvas);
    SkMatrix* matrix = new SkMatrix();
    *matrix = canvas->getTotalMatrix();
    return reinterpret_cast<skiac_matrix*>(matrix);
}

void skiac_canvas_draw_path(skiac_canvas* c_canvas, skiac_path* c_path, skiac_paint* c_paint)
{
    SkCanvas* canvas = reinterpret_cast<SkCanvas*>(c_canvas);
    const SkPath* path =  reinterpret_cast<const SkPath*>(c_path);
    const SkPaint* paint = reinterpret_cast<const SkPaint*>(c_paint);
    canvas->drawPath(*path, *paint);
}

void skiac_canvas_draw_rect(skiac_canvas* c_canvas, double x, double y, double w, double h, skiac_paint* c_paint)
{
    SkCanvas* canvas = reinterpret_cast<SkCanvas*>(c_canvas);
    SkRect rect = SkRect::MakeXYWH( (SkScalar)x, (SkScalar)y, (SkScalar)w, (SkScalar)h );
    const SkPaint* paint = reinterpret_cast<const SkPaint*>(c_paint);
    canvas->drawRect(rect, *paint);
}

void skiac_canvas_draw_surface(skiac_canvas* c_canvas, skiac_surface* c_surface, double left, double top, uint8_t alpha, BlendMode blendMode)
{
    SkCanvas* canvas = reinterpret_cast<SkCanvas*>(c_canvas);
    SkSurface* surface = reinterpret_cast<SkSurface*>(c_surface);
    sk_sp<SkImage> image = surface->makeImageSnapshot();
    SkPaint paint;
    paint.setFilterQuality(SkFilterQuality::kLow_SkFilterQuality);
    paint.setAlpha(alpha);
    paint.setBlendMode(blendModes_[static_cast<int>(blendMode)]);
    canvas->drawImage(image, (SkScalar)left, (SkScalar)top, &paint);
}

void skiac_canvas_reset_matrix(skiac_canvas* c_canvas)
{
    SkCanvas* canvas = reinterpret_cast<SkCanvas*>(c_canvas);
    canvas->resetMatrix();
}

void skiac_canvas_clip_rect(skiac_canvas* c_canvas, const skia_rect* c_rect)
{
    SkCanvas* canvas = reinterpret_cast<SkCanvas*>(c_canvas);
    SkRect rect = SkRect::MakeLTRB(c_rect->left, c_rect->top, c_rect->right, c_rect->bottom);
    canvas->clipRect(rect, true);
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

skiac_matrix *skiac_matrix_create_inverse(skiac_matrix *c_mat)
{
    const auto mat = reinterpret_cast<SkMatrix*>(c_mat);
    SkMatrix* inverse = new SkMatrix();
    // TODO: check for non-invertability.
    auto res = mat->invert(inverse);
    (void)res;
    return reinterpret_cast<skiac_matrix*>(inverse);
}

skia_matrix skiac_matrix_get_data(skiac_matrix *c_mat)
{
    const auto mat = reinterpret_cast<SkMatrix*>(c_mat);
    skia_matrix raw_mat;
    raw_mat.a = (double)mat->getScaleX();
    raw_mat.b = (double)mat->getSkewY();
    raw_mat.c = (double)mat->getSkewX();
    raw_mat.d = (double)mat->getScaleY();
    raw_mat.e = (double)mat->getTranslateX();
    raw_mat.f = (double)mat->getTranslateY();

    return raw_mat;
}

void skiac_matrix_pre_translate(skiac_matrix *c_mat, double dx, double dy)
{
    auto mat = reinterpret_cast<SkMatrix*>(c_mat);
    mat->preTranslate((SkScalar)dx, (SkScalar)dy);
}

void skiac_matrix_pre_scale(skiac_matrix *c_mat, double sx, double sy)
{
    auto mat = reinterpret_cast<SkMatrix*>(c_mat);
    mat->preScale((SkScalar)sx, (SkScalar)sy);
}


void skiac_matrix_map_rect(skiac_matrix *c_mat, skia_rect *c_dst, const skia_rect* c_src)
{
    const auto mat = reinterpret_cast<SkMatrix*>(c_mat);
    SkRect src = SkRect::MakeLTRB(c_src->left, c_src->top, c_src->right, c_src->bottom);
    SkRect dst;
    mat->mapRect(&dst, src);
    *c_dst = { dst.fLeft, dst.fTop, dst.fRight, dst.fBottom };
}

void skiac_matrix_reset(skiac_matrix *c_mat)
{
    const auto mat = reinterpret_cast<SkMatrix*>(c_mat);
    mat->reset();
}

void skiac_matrix_destroy(skiac_matrix *c_mat)
{
    // SkMatrix is NOT ref counted
    SkMatrix* matrix = reinterpret_cast<SkMatrix*>(c_mat);
    delete matrix;
}

// Paint

skiac_paint* skiac_paint_create()
{
    SkPaint* paint = new SkPaint();
    return reinterpret_cast<skiac_paint*>(paint);
}

void skiac_paint_destroy(skiac_paint* c_paint)
{
    SkPaint* paint = reinterpret_cast<SkPaint*>(c_paint);

    // Setting these references to nullptr should decrement their ref count.
    paint->setShader(nullptr);
    paint->setPathEffect(nullptr);

    // SkPaint is not ref counted, so explicitly delete.
    delete paint;
}

void skiac_paint_set_color(skiac_paint* c_paint, uint8_t r, uint8_t g, uint8_t b, uint8_t a)
{
    SkPaint* paint = reinterpret_cast<SkPaint*>(c_paint);
    paint->setARGB(a, r, g, b);
}

void skiac_paint_set_alpha(skiac_paint* c_paint, uint8_t a)
{
    SkPaint* paint = reinterpret_cast<SkPaint*>(c_paint);
    paint->setAlpha(a);
}

void skiac_paint_set_anti_alias(skiac_paint* c_paint, bool aa)
{
    SkPaint* paint = reinterpret_cast<SkPaint*>(c_paint);
    paint->setAntiAlias(aa);
}

void skiac_paint_set_blend_mode(skiac_paint* c_paint, BlendMode blendMode)
{
    SkPaint* paint = reinterpret_cast<SkPaint*>(c_paint);
    paint->setBlendMode(blendModes_[static_cast<int>(blendMode)]);
}

void skiac_paint_set_shader(skiac_paint* c_paint, skiac_shader* c_shader)
{
    SkPaint* paint = reinterpret_cast<SkPaint*>(c_paint);
    sk_sp<SkShader> shader(reinterpret_cast<SkShader*>(c_shader));
    paint->setShader(shader);
}

void skiac_paint_set_style(skiac_paint* c_paint, PaintStyle style)
{
    SkPaint* paint = reinterpret_cast<SkPaint*>(c_paint);
    paint->setStyle(paintStyles_[static_cast<int>(style)]);
}

void skiac_paint_set_stroke_width(skiac_paint* c_paint, double width)
{
    SkPaint* paint = reinterpret_cast<SkPaint*>(c_paint);
    paint->setStrokeWidth((SkScalar)width);
}

void skiac_paint_set_stroke_cap(skiac_paint* c_paint, StrokeCap cap)
{
    SkPaint* paint = reinterpret_cast<SkPaint*>(c_paint);
    paint->setStrokeCap(strokeCaps_[static_cast<int>(cap)]);
}

void skiac_paint_set_stroke_join(skiac_paint* c_paint, StrokeJoin join)
{
    SkPaint* paint = reinterpret_cast<SkPaint*>(c_paint);
    paint->setStrokeJoin(strokeJoins_[static_cast<int>(join)]);
}

void skiac_paint_set_stroke_miter(skiac_paint* c_paint, SkScalar miter)
{
    SkPaint* paint = reinterpret_cast<SkPaint*>(c_paint);
    paint->setStrokeMiter(miter);
}

void skiac_paint_set_path_effect(skiac_paint* c_paint, skiac_path_effect* c_path_effect)
{
    SkPaint* paint = reinterpret_cast<SkPaint*>(c_paint);
    sk_sp<SkPathEffect> pathEffect(reinterpret_cast<SkPathEffect*>(c_path_effect));
    paint->setPathEffect(pathEffect);
}

// Path

skiac_path* skiac_path_create()
{
    return reinterpret_cast<skiac_path*>(new SkPath());
}

void skiac_path_destroy(skiac_path* c_path)
{
    // SkPath is NOT ref counted
    delete reinterpret_cast<SkPath*>(c_path);
}

void skiac_path_set_fill_type(skiac_path* c_path, FillType type)
{
    SkPath* path = reinterpret_cast<SkPath*>(c_path);
    path->setFillType((SkPath::FillType)type);
}

void skiac_path_move_to(skiac_path* c_path, double x, double y)
{
    SkPath* path = reinterpret_cast<SkPath*>(c_path);
    path->moveTo((SkScalar)x, (SkScalar)y);
}

void skiac_path_line_to(skiac_path* c_path, double x, double y)
{
    SkPath* path = reinterpret_cast<SkPath*>(c_path);
    path->lineTo((SkScalar)x, (SkScalar)y);
}

void skiac_path_cubic_to(skiac_path* c_path, double x1, double y1, double x2, double y2, double x3, double y3)
{
    SkPath* path = reinterpret_cast<SkPath*>(c_path);
    path->cubicTo((SkScalar)x1, (SkScalar)y1, (SkScalar)x2, (SkScalar)y2, (SkScalar)x3, (SkScalar)y3);
}

void skiac_path_close(skiac_path* c_path)
{
    SkPath* path = reinterpret_cast<SkPath*>(c_path);
    path->close();
}

// PathEffect

skiac_path_effect* skiac_path_effect_make_dash_path(const SkScalar* intervals, int count, SkScalar phase)
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
    const skia_point* c_points, const SkColor* colors, const SkScalar* positions,
    int count, TileMode tile_mode,
    uint32_t flags, skiac_matrix *c_mat)
{
    const SkPoint* points = reinterpret_cast<const SkPoint*>(c_points);
    const SkMatrix* matrix = reinterpret_cast<const SkMatrix*>(c_mat);

    SkShader* shader = SkGradientShader::MakeLinear(
        points, colors, positions, count, tileModes_[static_cast<int>(tile_mode)],
        flags, matrix
    ).release();
    shader->ref();

    return reinterpret_cast<skiac_shader*>(shader);
}

skiac_shader* skiac_shader_make_two_point_conical_gradient(
    const skia_point c_start_point, SkScalar start_radius,
    const skia_point c_end_point, SkScalar end_radius,
    const SkColor* colors, const SkScalar* positions,
    int count, TileMode tile_mode,
    uint32_t flags, skiac_matrix *c_mat)
{
    const SkPoint startPoint = { c_start_point.x, c_start_point.y };
    const SkPoint endPoint = { c_end_point.x, c_end_point.y };
    const SkMatrix* matrix = reinterpret_cast<const SkMatrix*>(c_mat);

    SkShader* shader = SkGradientShader::MakeTwoPointConical(
        startPoint, start_radius,
        endPoint, end_radius,
        colors, positions, count, tileModes_[static_cast<int>(tile_mode)],
        flags, matrix
    ).release();
    shader->ref();

    return reinterpret_cast<skiac_shader*>(shader);
}

skiac_shader* skiac_shader_make_from_surface_image(skiac_surface* c_surface, const skiac_matrix* c_matrix)
{
    SkSurface* surface = reinterpret_cast<SkSurface*>(c_surface);
    const SkMatrix* matrix = reinterpret_cast<const SkMatrix*>(c_matrix);

    sk_sp<SkImage> image = surface->makeImageSnapshot();
    SkShader* shader = image->makeShader(
        SkTileMode::kRepeat,
        SkTileMode::kRepeat,
        matrix
    ).release();
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
