/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

/**
 * @file ResvgQt.h
 *
 * Qt wrapper for resvg C-API
 */

#ifndef RESVGQT_H
#define RESVGQT_H

#define RESVG_QT_BACKEND

extern "C" {
#include <resvg.h>
}

#include <QString>
#include <QScopedPointer>
#include <QRectF>
#include <QTransform>
#include <QGuiApplication>
#include <QScreen>
#include <QPainter>
#include <QFile>
#include <QDebug>

namespace ResvgPrivate {

static const char* toCStr(const QString &text)
{
    const auto utf8 = text.toUtf8();
    const auto data = utf8.constData();
    return qstrdup(data);
}

class Data
{
public:
    Data()
    {
        init();
    }

    ~Data()
    {
        clear();
    }

    void reset()
    {
        clear();
        init();
    }

    resvg_render_tree *tree = nullptr;
    resvg_options opt;
    qreal scaleFactor = 1.0;
    QRectF viewBox;
    QString errMsg;

private:
    void init()
    {
        resvg_init_options(&opt);

        QFont font;
        opt.font_family = toCStr(font.family());
        opt.font_size = font.pointSize();

        opt.languages = toCStr(QLocale().bcp47Name());

        opt.dpi = 96 * scaleFactor;
    }

    void clear()
    {
        if (tree) {
            resvg_tree_destroy(tree);
            tree = nullptr;
        }

        if (opt.path) {
            delete[] opt.path; // do not use free() because was allocated via qstrdup()
            opt.path = NULL;
        }

        if (opt.font_family) {
            delete[] opt.font_family; // do not use free() because was allocated via qstrdup()
            opt.font_family = NULL;
        }

        if (opt.languages) {
            delete[] opt.languages; // do not use free() because was allocated via qstrdup()
            opt.languages = NULL;
        }

        viewBox = QRectF();
        errMsg = QString();
    }
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

} //ResvgPrivate

/**
 * @brief QSvgRenderer-like wrapper for resvg C-API
 */
class ResvgRenderer {
public:
    /**
     * @brief Constructs a new renderer.
     */
    ResvgRenderer();

    /**
     * @brief Constructs a new renderer and loads the contents of the SVG(Z) file.
     */
    ResvgRenderer(const QString &filePath);

    /**
     * @brief Constructs a new renderer and loads the SVG data.
     */
    ResvgRenderer(const QByteArray &data);

    /**
     * @brief Destructs the renderer.
     */
    ~ResvgRenderer();

    /**
     * @brief Loads the contents of the SVG(Z) file.
     */
    bool load(const QString &filePath);

    /**
     * @brief Loads the SVG data.
     */
    bool load(const QByteArray &data);

    /**
     * @brief Returns \b true if the file or data were loaded successful.
     */
    bool isValid() const;

    /**
     * @brief Returns an underling error when #isValid is \b false.
     */
    QString errorString() const;

    /**
     * @brief Checks that underling tree has any nodes.
     *
     * #ResvgRenderer and #ResvgRenderer constructors
     * will set an error only if a file does not exist or it has a non-UTF-8 encoding.
     * All other errors will result in an empty tree with a 100x100px size.
     *
     * @return Returns \b true if tree has any nodes.
     */
    bool isEmpty() const;

    /**
     * @brief Returns an SVG size.
     */
    QSize defaultSize() const;

    /**
     * @brief Returns an SVG size.
     */
    QSizeF defaultSizeF() const;

    /**
     * @brief Returns an SVG viewbox.
     */
    QRect viewBox() const;

    /**
     * @brief Returns an SVG viewbox.
     */
    QRectF viewBoxF() const;

    /**
     * @brief Returns bounding rectangle of the item with the given \b id.
     *        The transformation matrix of parent elements is not affecting
     *        the bounds of the element.
     */
    QRectF boundsOnElement(const QString &id) const;

    /**
     * @brief Returns \b true if element with such an ID exists.
     */
    bool elementExists(const QString &id) const;

    /**
     * @brief Returns element's transform.
     */
    QTransform transformForElement(const QString &id) const;

    /**
     * @brief Sets the device pixel ratio for the image.
     */
    void setDevicePixelRatio(qreal scaleFactor);

    /**
     * @brief Renders the SVG data to canvas.
     */
    void render(QPainter *p);

    /**
     * @brief Renders the SVG data to canvas with the specified \b bounds.
     *
     * If the bounding rectangle is not specified
     * the SVG file is mapped to the whole paint device.
     */
    void render(QPainter *p, const QRectF &bounds);

    /**
     * @brief Renders the given element with \b elementId on the specified \b bounds.
     *
     * If the bounding rectangle is not specified
     * the SVG element is mapped to the whole paint device.
     */
    void render(QPainter *p, const QString &elementId,
                const QRectF &bounds = QRectF());

    /**
     * @brief Initializes the library log.
     *
     * Use it if you want to see any warnings.
     *
     * Must be called only once.
     *
     * All warnings will be printed to the \b stderr.
     */
    static void initLog();

private:
    QScopedPointer<ResvgPrivate::Data> d;
};

// Implementation.

inline ResvgRenderer::ResvgRenderer()
    : d(new ResvgPrivate::Data())
{
}

inline ResvgRenderer::ResvgRenderer(const QString &filePath)
    : d(new ResvgPrivate::Data())
{
    load(filePath);
}

inline ResvgRenderer::ResvgRenderer(const QByteArray &data)
    : d(new ResvgPrivate::Data())
{
    load(data);
}

inline ResvgRenderer::~ResvgRenderer() {}

inline bool ResvgRenderer::load(const QString &filePath)
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

    d->opt.path = ResvgPrivate::toCStr(filePath);

    const auto err = resvg_parse_tree_from_file(d->opt.path, &d->opt, &d->tree);
    if (err != RESVG_OK) {
        d->errMsg = ResvgPrivate::errorToString(err);
        return false;
    }

    const auto r = resvg_get_image_viewbox(d->tree);
    d->viewBox = QRectF(r.x, r.y, r.width, r.height);

    return true;
}

inline bool ResvgRenderer::load(const QByteArray &data)
{
    d->reset();

    const auto err = resvg_parse_tree_from_data(data.constData(), data.size(), &d->opt, &d->tree);
    if (err != RESVG_OK) {
        d->errMsg = ResvgPrivate::errorToString(err);
        return false;
    }

    const auto r = resvg_get_image_viewbox(d->tree);
    d->viewBox = QRectF(r.x, r.y, r.width, r.height);

    return true;
}

inline bool ResvgRenderer::isValid() const
{
    return d->tree;
}

inline QString ResvgRenderer::errorString() const
{
    return d->errMsg;
}

inline bool ResvgRenderer::isEmpty() const
{
    if (d->tree)
        return !resvg_is_image_empty(d->tree);
    else
        return true;
}

inline QSize ResvgRenderer::defaultSize() const
{
    return defaultSizeF().toSize();
}

inline QSizeF ResvgRenderer::defaultSizeF() const
{
    if (d->tree)
        return d->viewBox.size();
    else
        return QSizeF();
}

inline QRect ResvgRenderer::viewBox() const
{
    return viewBoxF().toRect();
}

inline QRectF ResvgRenderer::viewBoxF() const
{
    if (d->tree)
        return d->viewBox;
    else
        return QRectF();
}

inline QRectF ResvgRenderer::boundsOnElement(const QString &id) const
{
    if (!d->tree)
        return QRectF();

    const auto utf8Str = id.toUtf8();
    const auto rawId = utf8Str.constData();
    resvg_rect bbox;
    if (resvg_qt_get_node_bbox(d->tree, &d->opt, rawId, &bbox)) {
        return QRectF(bbox.x, bbox.y, bbox.height, bbox.width);
    }

    return QRectF();
}

inline bool ResvgRenderer::elementExists(const QString &id) const
{
    if (!d->tree)
        return false;

    const auto utf8Str = id.toUtf8();
    const auto rawId = utf8Str.constData();
    return resvg_node_exists(d->tree, rawId);
}

inline QTransform ResvgRenderer::transformForElement(const QString &id) const
{
    if (!d->tree)
        return QTransform();

    const auto utf8Str = id.toUtf8();
    const auto rawId = utf8Str.constData();
    resvg_transform ts;
    if (resvg_get_node_transform(d->tree, rawId, &ts)) {
        return QTransform(ts.a, ts.b, ts.c, ts.d, ts.e, ts.f);
    }

    return QTransform();
}

inline void ResvgRenderer::setDevicePixelRatio(qreal scaleFactor)
{
    d->scaleFactor = scaleFactor;
}

inline void ResvgRenderer::render(QPainter *p)
{
    render(p, QRectF());
}

inline void ResvgRenderer::render(QPainter *p, const QRectF &bounds)
{
    if (!d->tree)
        return;

    if (bounds.isValid()) {
        Q_UNIMPLEMENTED();
        return;
    }

    p->save();
    p->setRenderHint(QPainter::Antialiasing);

    const auto r = p->viewport();
    resvg_size imgSize { (uint)r.width(), (uint)r.height() };
    resvg_qt_render_to_canvas(d->tree, &d->opt, imgSize, p);

    p->restore();
}

inline void ResvgRenderer::render(QPainter *p, const QString &elementId, const QRectF &bounds)
{
    Q_UNUSED(p) Q_UNUSED(elementId) Q_UNUSED(bounds)
    Q_UNIMPLEMENTED();
}

inline void ResvgRenderer::initLog()
{
    resvg_init_log();
}

#undef RESVG_QT_BACKEND

#endif // RESVGQT_H
