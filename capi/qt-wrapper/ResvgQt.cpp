/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#include <ResvgQt.h>

extern "C" {
#define RESVG_QT_BACKEND
#include <resvg.h>
}

#include <QGuiApplication>
#include <QScreen>
#include <QPainter>
#include <QFile>
#include <QDebug>


static void initOptions(resvg_options &opt)
{
    resvg_init_options(&opt);

    const auto screens = qApp->screens();
    if (!screens.isEmpty()) {
        const auto screen = screens.at(0);
        opt.dpi = screen->logicalDotsPerInch() * screen->devicePixelRatio();
    }
}

class ResvgRendererPrivate
{
public:
    ~ResvgRendererPrivate()
    {
        reset();
    }

    void reset()
    {
        if (tree) {
            resvg_tree_destroy(tree);
            tree = nullptr;
        }

        if (opt.path) {
            delete[] opt.path; // do not use free() because was allocated via qstrdup()
            opt.path = NULL;
        }

        initOptions(opt);
        viewBox = QRectF();
        errMsg = QString();
    }

    resvg_render_tree *tree = nullptr;
    resvg_options opt;
    QRectF viewBox;
    QString errMsg;
};

static QString errorToString(const int err)
{
    switch (err) {
        case RESVG_OK :
            return QString(); break;
        case RESVG_ERROR_NOT_AN_UTF8_STR :
            return QLatin1Literal("The SVG content has not an UTF-8 encoding."); break;
        case RESVG_ERROR_FILE_OPEN_FAILED :
            return QLatin1Literal("Failed to open the file."); break;
        case RESVG_ERROR_FILE_WRITE_FAILED :
            return QLatin1Literal("Failed to write to the file."); break;
        case RESVG_ERROR_INVALID_FILE_SUFFIX :
            return QLatin1Literal("Invalid file suffix."); break;
        case RESVG_ERROR_MALFORMED_GZIP :
            return QLatin1Literal("Not a GZip compressed data."); break;
        case RESVG_ERROR_PARSING_FAILED :
            return QLatin1Literal("Failed to parse an SVG data."); break;
        case RESVG_ERROR_NO_CANVAS :
            return QLatin1Literal("Failed to allocate the canvas."); break;
    }

    Q_UNREACHABLE();
}

ResvgRenderer::ResvgRenderer()
    : d(new ResvgRendererPrivate())
{
}

ResvgRenderer::ResvgRenderer(const QString &filePath)
    : d(new ResvgRendererPrivate())
{
    load(filePath);
}

ResvgRenderer::ResvgRenderer(const QByteArray &data)
    : d(new ResvgRendererPrivate())
{
    load(data);
}

ResvgRenderer::~ResvgRenderer() {}

bool ResvgRenderer::load(const QString &filePath)
{
    // Check for Qt resource path.
    if (filePath.startsWith(QLatin1String(":/"))) {
        QFile file(filePath);
        if (file.open(QFile::ReadOnly)) {
            return load(file.readAll());
        } else {
            return false;
        }
    }

    d->reset();

    const auto utf8Str = filePath.toUtf8();
    const auto rawFilePath = utf8Str.constData();
    d->opt.path = qstrdup(rawFilePath);

    const auto err = resvg_parse_tree_from_file(rawFilePath, &d->opt, &d->tree);
    if (err != RESVG_OK) {
        d->errMsg = errorToString(err);
        return false;
    }

    const auto r = resvg_get_image_viewbox(d->tree);
    d->viewBox = QRectF(r.x, r.y, r.width, r.height);

    return true;
}

bool ResvgRenderer::load(const QByteArray &data)
{
    d->reset();

    const auto err = resvg_parse_tree_from_data(data.constData(), data.size(), &d->opt, &d->tree);
    if (err != RESVG_OK) {
        d->errMsg = errorToString(err);
        return false;
    }

    const auto r = resvg_get_image_viewbox(d->tree);
    d->viewBox = QRectF(r.x, r.y, r.width, r.height);

    return true;
}

bool ResvgRenderer::isValid() const
{
    return d->tree;
}

QString ResvgRenderer::errorString() const
{
    return d->errMsg;
}

bool ResvgRenderer::isEmpty() const
{
    if (d->tree)
        return !resvg_is_image_empty(d->tree);
    else
        return true;
}

QSize ResvgRenderer::defaultSize() const
{
    return defaultSizeF().toSize();
}

QSizeF ResvgRenderer::defaultSizeF() const
{
    if (d->tree)
        return d->viewBox.size();
    else
        return QSizeF();
}

QRect ResvgRenderer::viewBox() const
{
    return viewBoxF().toRect();
}

QRectF ResvgRenderer::viewBoxF() const
{
    if (d->tree)
        return d->viewBox;
    else
        return QRectF();
}

QRectF ResvgRenderer::boundsOnElement(const QString &id) const
{
    if (d->tree) {
        const auto utf8Str = id.toUtf8();
        const auto rawId = utf8Str.constData();
        resvg_rect bbox;
        if (resvg_qt_get_node_bbox(d->tree, &d->opt, rawId, &bbox)) {
            return QRectF(bbox.x, bbox.y, bbox.height, bbox.width);
        }
    }

    return QRectF();
}

bool ResvgRenderer::elementExists(const QString &id) const
{
    if (d->tree) {
        const auto utf8Str = id.toUtf8();
        const auto rawId = utf8Str.constData();
        return resvg_node_exists(d->tree, rawId);
    }

    return false;
}

QTransform ResvgRenderer::transformForElement(const QString &id) const
{
    if (d->tree) {
        const auto utf8Str = id.toUtf8();
        const auto rawId = utf8Str.constData();
        resvg_transform ts;
        if (resvg_get_node_transform(d->tree, rawId, &ts)) {
            return QTransform(ts.a, ts.b, ts.c, ts.d, ts.e, ts.f);
        }
    }

    return QTransform();
}

void ResvgRenderer::render(QPainter *p)
{
    render(p, QRectF());
}

void ResvgRenderer::render(QPainter *p, const QRectF &bounds)
{
    if (!d->tree)
        return;

    const auto r = bounds.isValid() ? bounds : p->viewport();

    p->save();
    p->setRenderHint(QPainter::Antialiasing);

    const double sx = (double)r.width() / d->viewBox.width();
    const double sy = (double)r.height() / d->viewBox.height();

    p->setTransform(QTransform(sx, 0, 0, sy, r.x(), r.y()), true);

    resvg_size imgSize { (uint)d->viewBox.width(), (uint)d->viewBox.height() };
    resvg_qt_render_to_canvas(d->tree, &d->opt, imgSize, p);

    p->restore();
}

void ResvgRenderer::render(QPainter *p, const QString &elementId, const QRectF &bounds)
{
    if (!d->tree)
        return;

    const auto utf8Str = elementId.toUtf8();
    const auto rawId = utf8Str.constData();

    resvg_rect bbox;
    if (!resvg_qt_get_node_bbox(d->tree, &d->opt, rawId, &bbox)) {
        qWarning() << QString(QStringLiteral("Element '%1' has no bounding box.")).arg(elementId);
        return;
    }

    p->save();
    p->setRenderHint(QPainter::Antialiasing);

    const auto r = bounds.isValid() ? bounds : p->viewport();

    const double sx = (double)r.width() / bbox.width;
    const double sy = (double)r.height() / bbox.height;
    p->setTransform(QTransform(sx, 0, 0, sy, bounds.x(), bounds.y()), true);

    resvg_size imgSize { (uint)bbox.width, (uint)bbox.height };
    resvg_qt_render_to_canvas_by_id(d->tree, &d->opt, imgSize, rawId, p);

    p->restore();
}

void ResvgRenderer::initLog()
{
    resvg_init_log();
}
