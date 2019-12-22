#include <QGuiApplication>
#include <QImage>
#include <QPainter>
#include <QImageWriter>
#include <QDebug>

#include "qt_capi.hpp"

#define IMAGE_CAST reinterpret_cast<QImage*>(c_img)
#define PAINTER_CAST reinterpret_cast<QPainter*>(c_p)
#define PATH_CAST reinterpret_cast<QPainterPath*>(c_pp)
#define TRANSFORM_CAST reinterpret_cast<QTransform*>(c_ts)
#define PEN_CAST reinterpret_cast<QPen*>(c_pen)
#define BRUSH_CAST reinterpret_cast<QBrush*>(c_brush)
#define LG_CAST reinterpret_cast<QLinearGradient*>(c_lg)
#define RG_CAST reinterpret_cast<QRadialGradient*>(c_rg)

extern "C" {

// QImage

qtc_qimage * qtc_qimage_create_rgba_premultiplied(uint32_t width, uint32_t height)
{
    QImage *img = new QImage(width, height, QImage::Format_ARGB32_Premultiplied);

    if (img->isNull()) {
        return 0;
    }

    return reinterpret_cast<qtc_qimage*>(img);
}

qtc_qimage * qtc_qimage_create_rgba(uint32_t width, uint32_t height)
{
    QImage *img = new QImage(width, height, QImage::Format_ARGB32);

    if (img->isNull()) {
        return 0;
    }

    return reinterpret_cast<qtc_qimage*>(img);
}

uint8_t* qtc_qimage_get_data(qtc_qimage *c_img)
{
    return IMAGE_CAST->bits();
}

uint32_t qtc_qimage_get_size_in_bytes(qtc_qimage *c_img)
{
#if QT_VERSION >= QT_VERSION_CHECK(5,10,0)
    return IMAGE_CAST->sizeInBytes();
#else
    return IMAGE_CAST->byteCount();
#endif
}

qtc_qimage* qtc_qimage_resize(qtc_qimage *c_img, uint32_t width, uint32_t height, AspectRatioMode ratio,
                              bool smoothTransformation)
{
    const auto mode = smoothTransformation ? Qt::SmoothTransformation : Qt::FastTransformation;
    const QImage rImg = IMAGE_CAST->scaled(width, height, Qt::AspectRatioMode(ratio), mode);
    return reinterpret_cast<qtc_qimage*>(new QImage(rImg));
}

qtc_qimage* qtc_qimage_copy(qtc_qimage *c_img, uint32_t x, uint32_t y, uint32_t width, uint32_t height)
{
    const QImage rImg = IMAGE_CAST->copy(x, y, width, height);
    return reinterpret_cast<qtc_qimage*>(new QImage(rImg));
}

void qtc_qimage_fill(qtc_qimage *c_img, uint8_t r, uint8_t g, uint8_t b, uint8_t a)
{
    IMAGE_CAST->fill(QColor(r, g, b, a));
}

qtc_qimage* qtc_qimage_to_rgba(qtc_qimage *c_img)
{
    const QImage rImg = IMAGE_CAST->convertToFormat(QImage::Format_ARGB32);
    return reinterpret_cast<qtc_qimage*>(new QImage(rImg));
}

uint32_t qtc_qimage_get_width(qtc_qimage *c_img)
{
    return IMAGE_CAST->width();
}

uint32_t qtc_qimage_get_height(qtc_qimage *c_img)
{
    return IMAGE_CAST->height();
}

bool qtc_qimage_save(qtc_qimage *c_img, const char *path)
{
    QImageWriter writer(QString::fromUtf8(path));
    writer.setCompression(20); // Use a lower ratio to speed up compression.
    return writer.write(*IMAGE_CAST);
}

void qtc_qimage_destroy(qtc_qimage *c_img)
{
    delete IMAGE_CAST;
}

// QPainter

qtc_qpainter *qtc_qpainter_create(qtc_qimage *c_img)
{
    auto p = new QPainter();

    p->begin(IMAGE_CAST);
    p->setPen(Qt::NoPen);
    p->setBrush(Qt::NoBrush);
    p->setRenderHint(QPainter::Antialiasing, true);
    p->setRenderHint(QPainter::SmoothPixmapTransform, true);

    return reinterpret_cast<qtc_qpainter*>(p);
}

void qtc_qpainter_set_antialiasing(qtc_qpainter *c_p, bool flag)
{
    PAINTER_CAST->setRenderHint(QPainter::Antialiasing, flag);
}

void qtc_qpainter_set_smooth_pixmap_transform(qtc_qpainter *c_p, bool flag)
{
    PAINTER_CAST->setRenderHint(QPainter::SmoothPixmapTransform, flag);
}

void qtc_qpainter_set_pen(qtc_qpainter *c_p, qtc_qpen *c_pen)
{
    PAINTER_CAST->setPen(*PEN_CAST);
}

void qtc_qpainter_reset_pen(qtc_qpainter *c_p)
{
    PAINTER_CAST->setPen(Qt::NoPen);
}

void qtc_qpainter_set_brush(qtc_qpainter *c_p, qtc_qbrush *c_brush)
{
    PAINTER_CAST->setBrush(*BRUSH_CAST);
}

void qtc_qpainter_reset_brush(qtc_qpainter *c_p)
{
    PAINTER_CAST->setBrush(Qt::NoBrush);
}

void qtc_qpainter_set_opacity(qtc_qpainter *c_p, double opacity)
{
    PAINTER_CAST->setOpacity(opacity);
}

void qtc_qpainter_draw_path(qtc_qpainter *c_p, qtc_qpainterpath *c_pp)
{
    PAINTER_CAST->drawPath(*PATH_CAST);
}

void qtc_qpainter_draw_image(qtc_qpainter *c_p, double x, double y, qtc_qimage *c_img)
{
    PAINTER_CAST->drawImage(QPointF(x, y), *IMAGE_CAST);
}

void qtc_qpainter_draw_image_rect(qtc_qpainter *c_p, double x, double y, double w, double h, qtc_qimage *c_img)
{
    PAINTER_CAST->drawImage(QRectF(x, y, w, h), *IMAGE_CAST);
}

void qtc_qpainter_draw_text(qtc_qpainter *c_p, double x, double y, const char *c_text)
{
    auto p = PAINTER_CAST;

    const QString text = QString::fromUtf8(c_text);

    QPainterPath path;
    path.addText(QPointF(x, y + p->fontMetrics().ascent()), p->font(), text);
    p->drawPath(path);
}

void qtc_qpainter_draw_rect(qtc_qpainter *c_p, double x, double y, double w, double h)
{
    PAINTER_CAST->drawRect(QRectF(x, y, w, h));
}

void qtc_qpainter_translate(qtc_qpainter *c_p, double tx, double ty)
{
    PAINTER_CAST->translate(tx, ty);
}

void qtc_qpainter_scale(qtc_qpainter *c_p, double sx, double sy)
{
    PAINTER_CAST->scale(sx, sy);
}

qtc_qtransform *qtc_qpainter_get_transform(qtc_qpainter *c_p)
{
    const auto ts = PAINTER_CAST->transform();

    return reinterpret_cast<qtc_qtransform*>(new QTransform(ts));
}

void qtc_qpainter_set_transform(qtc_qpainter *c_p, qtc_qtransform *c_ts, bool combine)
{
    PAINTER_CAST->setTransform(*TRANSFORM_CAST, combine);
}

void qtc_qpainter_set_clip_rect(qtc_qpainter *c_p, double x, double y, double w, double h)
{
    PAINTER_CAST->setClipRect(QRectF(x, y, w, h));
}

void qtc_qpainter_set_clip_path(qtc_qpainter *c_p, qtc_qpainterpath *c_pp)
{
    PAINTER_CAST->setClipPath(*PATH_CAST);
}

void qtc_qpainter_reset_clip_path(qtc_qpainter *c_p)
{
    PAINTER_CAST->setClipPath(QPainterPath(), Qt::NoClip);
}

void qtc_qpainter_set_composition_mode(qtc_qpainter *c_p, CompositionMode mode)
{
    PAINTER_CAST->setCompositionMode(QPainter::CompositionMode(mode));
}

void qtc_qpainter_end(qtc_qpainter *c_p)
{
    PAINTER_CAST->end();
}

void qtc_qpainter_destroy(qtc_qpainter *c_p)
{
    delete PAINTER_CAST;
}

// QPainterPath

qtc_qpainterpath *qtc_qpainterpath_create()
{
    return reinterpret_cast<qtc_qpainterpath*>(new QPainterPath());
}

void qtc_qpainterpath_move_to(qtc_qpainterpath *c_pp, double x, double y)
{
    PATH_CAST->moveTo(x, y);
}

void qtc_qpainterpath_line_to(qtc_qpainterpath *c_pp, double x, double y)
{
    PATH_CAST->lineTo(x, y);
}

void qtc_qpainterpath_curve_to(qtc_qpainterpath *c_pp, double x1, double y1, double x2, double y2,
                               double x, double y)
{
    PATH_CAST->cubicTo(x1, y1, x2, y2, x, y);
}

void qtc_qpainterpath_close_path(qtc_qpainterpath *c_pp)
{
    PATH_CAST->closeSubpath();
}

void qtc_qpainterpath_set_fill_rule(qtc_qpainterpath *c_pp, FillRule rule)
{
    PATH_CAST->setFillRule(Qt::FillRule(rule));
}

void qtc_qpainterpath_destroy(qtc_qpainterpath *c_pp)
{
    delete PATH_CAST;
}

// QTransform

qtc_qtransform *qtc_qtransform_create()
{
    return reinterpret_cast<qtc_qtransform*>(new QTransform());
}

qtc_qtransform *qtc_qtransform_create_from(double a, double b, double c, double d, double e, double f)
{
    return reinterpret_cast<qtc_qtransform*>(new QTransform(a, b, c, d, e, f));
}

qtc_transform qtc_qtransform_get_data(qtc_qtransform *c_ts)
{
    const auto ts = TRANSFORM_CAST;
    qtc_transform raw_ts;
    raw_ts.a = ts->m11();
    raw_ts.b = ts->m12();
    raw_ts.c = ts->m21();
    raw_ts.d = ts->m22();
    raw_ts.e = ts->m31();
    raw_ts.f = ts->m32();

    return raw_ts;
}

void qtc_qtransform_destroy(qtc_qtransform *c_ts)
{
    delete TRANSFORM_CAST;
}

// QPen

qtc_qpen *qtc_qpen_create()
{
    return reinterpret_cast<qtc_qpen*>(new QPen());
}

void qtc_qpen_destroy(qtc_qpen *c_pen)
{
    delete PEN_CAST;
}

void qtc_qpen_set_color(qtc_qpen *c_pen, uint8_t r, uint8_t g, uint8_t b, uint8_t a)
{
    PEN_CAST->setColor(QColor(r, g, b, a));
}

void qtc_qpen_set_brush(qtc_qpen *c_pen, qtc_qbrush *c_brush)
{
    PEN_CAST->setBrush(*BRUSH_CAST);
}

void qtc_qpen_set_line_cap(qtc_qpen *c_pen, PenCapStyle s)
{
    PEN_CAST->setCapStyle(Qt::PenCapStyle(s));
}

void qtc_qpen_set_line_join(qtc_qpen *c_pen, PenJoinStyle s)
{
    PEN_CAST->setJoinStyle(Qt::PenJoinStyle(s));
}

void qtc_qpen_set_width(qtc_qpen *c_pen, double width)
{
    PEN_CAST->setWidthF(width);
}

void qtc_qpen_set_miter_limit(qtc_qpen *c_pen, double limit)
{
    PEN_CAST->setMiterLimit(limit);
}

void qtc_qpen_set_dash_offset(qtc_qpen *c_pen, double offset)
{
    qreal w = PEN_CAST->widthF();
    if (w == 0) {
        w = 1;
    }

    PEN_CAST->setDashOffset(offset / w);
}

void qtc_qpen_set_dash_array(qtc_qpen *c_pen, const double *array, int len)
{
    QVector<double> dashes;
    dashes.reserve(len);

    qreal w = PEN_CAST->widthF();
    if (qFuzzyIsNull(w)) {
        w = 1;
    }

    for (int i = 0; i < len; ++i) {
        dashes << array[i] / w;
    }

    PEN_CAST->setDashPattern(dashes);
}

// QBrush

qtc_qbrush *qtc_qbrush_create()
{
    return reinterpret_cast<qtc_qbrush*>(new QBrush(Qt::SolidPattern));
}

void qtc_qbrush_set_color(qtc_qbrush *c_brush, uint8_t r, uint8_t g, uint8_t b, uint8_t a)
{
    BRUSH_CAST->setColor(QColor(r, g, b, a));
}

void qtc_qbrush_set_linear_gradient(qtc_qbrush *c_brush, qtc_qlineargradient *c_lg)
{
    *BRUSH_CAST = QBrush(*LG_CAST);
}

void qtc_qbrush_set_radial_gradient(qtc_qbrush *c_brush, qtc_qradialgradient *c_rg)
{
    *BRUSH_CAST = QBrush(*RG_CAST);
}

void qtc_qbrush_set_pattern(qtc_qbrush *c_brush, qtc_qimage *c_img)
{
    BRUSH_CAST->setTextureImage(*IMAGE_CAST);
}

void qtc_qbrush_set_transform(qtc_qbrush *c_brush, qtc_qtransform *c_ts)
{
    BRUSH_CAST->setTransform(*TRANSFORM_CAST);
}

void qtc_qbrush_destroy(qtc_qbrush *c_brush)
{
    delete BRUSH_CAST;
}

// QLinearGradient

qtc_qlineargradient *qtc_qlineargradient_create(double x1, double y1, double x2, double y2)
{
    auto lg = new QLinearGradient(x1, y1, x2, y2);
    lg->setInterpolationMode(QGradient::ComponentInterpolation);
    return reinterpret_cast<qtc_qlineargradient*>(lg);
}

void qtc_qlineargradient_set_color_at(qtc_qlineargradient *c_lg, double offset,
                                      uint8_t r, uint8_t g, uint8_t b, uint8_t a)
{
    LG_CAST->setColorAt(offset, QColor(r, g, b, a));
}

void qtc_qlineargradient_set_spread(qtc_qlineargradient *c_lg, Spread s)
{
    LG_CAST->setSpread(QGradient::Spread(s));
}

void qtc_qlineargradient_destroy(qtc_qlineargradient *c_lg)
{
    delete LG_CAST;
}

// QRadialGradient

qtc_qradialgradient *qtc_qradialgradient_create(double cx, double cy, double fx, double fy, double r)
{
    auto rg = new QRadialGradient(cx, cy, r, fx, fy);
    rg->setInterpolationMode(QGradient::ComponentInterpolation);
    return reinterpret_cast<qtc_qradialgradient*>(rg);
}

void qtc_qradialgradient_set_color_at(qtc_qradialgradient *c_rg, double offset,
                                      uint8_t r, uint8_t g, uint8_t b, uint8_t a)
{
    RG_CAST->setColorAt(offset, QColor(r, g, b, a));
}

void qtc_qradialgradient_set_spread(qtc_qradialgradient *c_rg, Spread s)
{
    RG_CAST->setSpread(QGradient::Spread(s));
}

void qtc_qradialgradient_destroy(qtc_qradialgradient *c_rg)
{
    delete RG_CAST;
}

}
