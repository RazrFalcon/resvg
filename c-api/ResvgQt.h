/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

/**
 * @file ResvgQt.h
 *
 * An idiomatic Qt API for resvg.
 */

#ifndef RESVG_QT_H
#define RESVG_QT_H

#define RESVG_QT_MAJOR_VERSION 0
#define RESVG_QT_MINOR_VERSION 11
#define RESVG_QT_PATCH_VERSION 0
#define RESVG_QT_VERSION "0.11.0"

#include <QDebug>
#include <QFile>
#include <QGuiApplication>
#include <QImage>
#include <QRectF>
#include <QScopedPointer>
#include <QScreen>
#include <QString>
#include <QTransform>

#include <resvg.h>

namespace ResvgPrivate {

class Data
{
public:
    ~Data()
    {
        clear();
    }

    void reset()
    {
        clear();
    }

    resvg_render_tree *tree = nullptr;
    QRectF viewBox;
    QString errMsg;

private:
    void clear()
    {
        // No need to deallocate opt.font_family, because it is a constant.

        if (tree) {
            resvg_tree_destroy(tree);
            tree = nullptr;
        }

        viewBox = QRectF();
        errMsg = QString();
    }
};

static QString errorToString(const int err)
{
    switch (err) {
        case RESVG_OK :
            return QString();
        case RESVG_ERROR_NOT_AN_UTF8_STR :
            return QLatin1String("The SVG content has not an UTF-8 encoding.");
        case RESVG_ERROR_FILE_OPEN_FAILED :
            return QLatin1String("Failed to read the file.");
        case RESVG_ERROR_INVALID_FILE_SUFFIX :
            return QLatin1String("Invalid file suffix.");
        case RESVG_ERROR_MALFORMED_GZIP :
            return QLatin1String("Not a GZip compressed data.");
        case RESVG_ERROR_INVALID_SIZE :
            return QLatin1String("SVG doesn't have a valid size.");
        case RESVG_ERROR_PARSING_FAILED :
            return QLatin1String("Failed to parse an SVG data.");
    }

    Q_UNREACHABLE();
}

} //ResvgPrivate

/**
 * @brief SVG parsing options.
 */
class ResvgOptions {
public:
    /**
     * @brief Constructs a new options set.
     */
    ResvgOptions()
        : d(resvg_options_create())
    {
        // Do not set the default font via QFont::family()
        // because it will return a dummy one on Windows.
        // See https://github.com/RazrFalcon/resvg/issues/159

        setLanguages({ QLocale().bcp47Name() });
    }

    /**
     * @brief Sets an SVG image path.
     *
     * Used to resolve relative image paths.
     *
     * Default: not set
     */
    void setFilePath(const QString &path)
    {
        if (path.isEmpty()) {
            resvg_options_set_file_path(d, nullptr);
        } else {
            auto pathC = path.toUtf8();
            pathC.append('\0');
            resvg_options_set_file_path(d, pathC.constData());
        }
    }

    /**
     * @brief Sets the target DPI.
     *
     * Impact units conversion.
     *
     * Default: 96
     */
    void setDpi(const double dpi)
    {
        resvg_options_set_dpi(d, dpi);
    }

    /**
     * @brief Sets the default font family.
     *
     * Will be used when no `font-family` attribute is set in the SVG.
     *
     * Default: Times New Roman
     */
    void setFontFamily(const QString &family)
    {
        if (family.isEmpty()) {
            return;
        }

        auto familyC = family.toUtf8();
        familyC.append('\0');
        resvg_options_set_font_family(d, familyC.constData());
    }

    /**
     * @brief Sets the default font size.
     *
     * Will be used when no `font-size` attribute is set in the SVG.
     *
     * Default: 12
     */
    void setFontSize(const double size)
    {
        resvg_options_set_font_size(d, size);
    }

    /**
     * @brief Sets a list of languages.
     *
     * Will be used to resolve a `systemLanguage` conditional attribute.
     *
     * Example: en, en-US.
     *
     * Default: en
     */
    void setLanguages(const QStringList &languages)
    {
        if (languages.isEmpty()) {
            resvg_options_set_languages(d, nullptr);
        } else {
            auto languagesC = languages.join(',').toUtf8();
            languagesC.append('\0');
            resvg_options_set_languages(d, languagesC.constData());
        }
    }

    /**
     * @brief Sets the default shape rendering method.
     *
     * Will be used when an SVG element's `shape-rendering` property is set to `auto`.
     *
     * Default: `RESVG_SHAPE_RENDERING_GEOMETRIC_PRECISION`
     */
    void setShapeRenderingMode(const resvg_shape_rendering mode)
    {
        resvg_options_set_shape_rendering_mode(d, mode);
    }

    /**
     * @brief Sets the default text rendering method.
     *
     * Will be used when an SVG element's `text-rendering` property is set to `auto`.
     *
     * Default: `RESVG_TEXT_RENDERING_OPTIMIZE_LEGIBILITY`
     */
    void setTextRenderingMode(const resvg_text_rendering mode)
    {
        resvg_options_set_text_rendering_mode(d, mode);
    }

    /**
     * @brief Sets the default image rendering method.
     *
     * Will be used when an SVG element's `image-rendering` property is set to `auto`.
     *
     * Default: `RESVG_IMAGE_RENDERING_OPTIMIZE_QUALITY`
     */
    void setImageRenderingMode(const resvg_image_rendering mode)
    {
        resvg_options_set_image_rendering_mode(d, mode);
    }

    /**
     * @brief Keep named groups.
     *
     * If set to `true`, all non-empty groups with `id` attribute will not be removed.
     *
     * Default: false
     */
    void setKeepNamedGroups(const bool keep)
    {
        resvg_options_set_keep_named_groups(d, keep);
    }

    /**
     * @brief Loads a font data into the internal fonts database.
     *
     * Prints a warning into the log when the data is not a valid TrueType font.
     */
    void loadFontData(const QByteArray &data)
    {
        resvg_options_load_font_data(d, data.constData(), data.size());
    }

    /**
     * @brief Loads a font file into the internal fonts database.
     *
     * Prints a warning into the log when the data is not a valid TrueType font.
     */
    bool loadFontFile(const QString &path)
    {
        auto pathC = path.toUtf8();
        pathC.append('\0');
        return resvg_options_load_font_file(d, pathC.constData());
    }

    /**
     * @brief Loads system fonts into the internal fonts database.
     *
     * This method is very IO intensive.
     *
     * This method should be executed only once per #resvg_options.
     *
     * The system scanning is not perfect, so some fonts may be omitted.
     * Please send a bug report in this case.
     *
     * Prints warnings into the log.
     */
    void loadSystemFonts()
    {
        resvg_options_load_system_fonts(d);
    }

    /**
     * @brief Destructs options.
     */
    ~ResvgOptions()
    {
        resvg_options_destroy(d);
    }

    friend class ResvgRenderer;

private:
    resvg_options * const d;
};

/**
 * @brief QSvgRenderer-like wrapper for resvg.
 */
class ResvgRenderer {
public:
    /**
     * @brief Constructs a new renderer.
     */
    ResvgRenderer()
        : d(new ResvgPrivate::Data())
    {
    }

    /**
     * @brief Constructs a new renderer and loads the contents of the SVG(Z) file.
     */
    ResvgRenderer(const QString &filePath, const ResvgOptions &opt)
        : d(new ResvgPrivate::Data())
    {
        load(filePath, opt);
    }

    /**
     * @brief Constructs a new renderer and loads the SVG data.
     */
    ResvgRenderer(const QByteArray &data, const ResvgOptions &opt)
        : d(new ResvgPrivate::Data())
    {
        load(data, opt);
    }

    /**
     * @brief Loads the contents of the SVG(Z) file.
     */
    bool load(const QString &filePath, const ResvgOptions &opt)
    {
        // Check for Qt resource path.
        if (filePath.startsWith(QLatin1String(":/"))) {
            QFile file(filePath);
            if (file.open(QFile::ReadOnly)) {
                return load(file.readAll(), opt);
            } else {
                return false;
            }
        }

        d->reset();

        auto filePathC = filePath.toUtf8();
        filePathC.append('\0');
//        resvg_options_set_file_path(opt.d, filePathC.constData());

        const auto err = resvg_parse_tree_from_file(filePathC.constData(), opt.d, &d->tree);
        if (err != RESVG_OK) {
            d->errMsg = ResvgPrivate::errorToString(err);
            return false;
        }

        const auto r = resvg_get_image_viewbox(d->tree);
        d->viewBox = QRectF(r.x, r.y, r.width, r.height);

        return true;
    }

    /**
     * @brief Loads the SVG data.
     */
    bool load(const QByteArray &data, const ResvgOptions &opt)
    {
        d->reset();

        const auto err = resvg_parse_tree_from_data(data.constData(), data.size(), opt.d, &d->tree);
        if (err != RESVG_OK) {
            d->errMsg = ResvgPrivate::errorToString(err);
            return false;
        }

        const auto r = resvg_get_image_viewbox(d->tree);
        d->viewBox = QRectF(r.x, r.y, r.width, r.height);

        return true;
    }

    /**
     * @brief Returns \b true if the file or data were loaded successful.
     */
    bool isValid() const
    {
        return d->tree;
    }

    /**
     * @brief Returns an underling error when #isValid is \b false.
     */
    QString errorString() const
    {
        return d->errMsg;
    }

    /**
     * @brief Checks that underling tree has any nodes.
     *
     * #ResvgRenderer and #ResvgRenderer constructors
     * will set an error only if a file does not exist or it has a non-UTF-8 encoding.
     * All other errors will result in an empty tree with a 100x100px size.
     *
     * @return Returns \b true if tree has any nodes.
     */
    bool isEmpty() const
    {
        if (d->tree)
            return !resvg_is_image_empty(d->tree);
        else
            return true;
    }

    /**
     * @brief Returns an SVG size.
     */
    QSize defaultSize() const
    {
        return defaultSizeF().toSize();
    }

    /**
     * @brief Returns an SVG size.
     */
    QSizeF defaultSizeF() const
    {
        if (d->tree)
            return d->viewBox.size();
        else
            return QSizeF();
    }

    /**
     * @brief Returns an SVG viewbox.
     */
    QRect viewBox() const
    {
        return viewBoxF().toRect();
    }

    /**
     * @brief Returns an SVG viewbox.
     */
    QRectF viewBoxF() const
    {
        if (d->tree)
            return d->viewBox;
        else
            return QRectF();
    }

    /**
     * @brief Returns bounding rectangle of the item with the given \b id.
     *        The transformation matrix of parent elements is not affecting
     *        the bounds of the element.
     */
    QRectF boundsOnElement(const QString &id) const
    {
        if (!d->tree)
            return QRectF();

        const auto utf8Str = id.toUtf8();
        const auto rawId = utf8Str.constData();
        resvg_rect bbox;
        if (resvg_get_node_bbox(d->tree, rawId, &bbox))
            return QRectF(bbox.x, bbox.y, bbox.width, bbox.height);

        return QRectF();
    }

    /**
     * @brief Returns bounding rectangle of a whole image.
     */
    QRectF boundingBox() const
    {
        if (!d->tree)
            return QRectF();

        resvg_rect bbox;
        if (resvg_get_image_bbox(d->tree, &bbox))
            return QRectF(bbox.x, bbox.y, bbox.width, bbox.height);

        return QRectF();
    }

    /**
     * @brief Returns \b true if element with such an ID exists.
     */
    bool elementExists(const QString &id) const
    {
        if (!d->tree)
            return false;

        const auto utf8Str = id.toUtf8();
        const auto rawId = utf8Str.constData();
        return resvg_node_exists(d->tree, rawId);
    }

    /**
     * @brief Returns element's transform.
     */
    QTransform transformForElement(const QString &id) const
    {
        if (!d->tree)
            return QTransform();

        const auto utf8Str = id.toUtf8();
        const auto rawId = utf8Str.constData();
        resvg_transform ts;
        if (resvg_get_node_transform(d->tree, rawId, &ts))
            return QTransform(ts.a, ts.b, ts.c, ts.d, ts.e, ts.f);

        return QTransform();
    }

    // TODO: render node

    /**
     * @brief Renders the SVG data to \b QImage with a specified \b size.
     *
     * If \b size is not set, the \b defaultSize() will be used.
     */
    QImage renderToImage(const QSize &size = QSize()) const
    {
        resvg_fit_to fit_to = { RESVG_FIT_TO_ORIGINAL, 1 };
        if (size.isValid()) {
            // TODO: support height too.
            fit_to.type = RESVG_FIT_TO_WIDTH;
            fit_to.value = size.width();
        }

        const auto resvg_img = resvg_render(d->tree, fit_to, nullptr);
        if (!resvg_img) {
            return QImage();
        }

        QImage qImg(resvg_image_get_width(resvg_img),
                    resvg_image_get_height(resvg_img),
                    QImage::Format_ARGB32);

        size_t len;
        auto img_data = resvg_image_get_data(resvg_img, &len);
        memcpy(qImg.bits(), img_data, len);

        resvg_image_destroy(resvg_img);

        // std::move is required to call inplace version of rgbSwapped().
        return std::move(qImg).rgbSwapped();
    }

    /**
     * @brief Initializes the library log.
     *
     * Use it if you want to see any warnings.
     *
     * Must be called only once.
     *
     * All warnings will be printed to the \b stderr.
     */
    static void initLog()
    {
        resvg_init_log();
    }

private:
    QScopedPointer<ResvgPrivate::Data> d;
};

#endif // RESVG_QT_H
