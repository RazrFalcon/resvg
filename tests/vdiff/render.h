#pragma once

#include <QObject>
#include <QFutureWatcher>
#include <QImage>

#include "either.h"

enum class ImageType
{
    Chrome,
    ResvgCairo,
    ResvgQt,
    Inkscape,
    Rsvg,
    QtSvg,
};

struct RenderData
{
    ImageType type;
    int viewSize;
    QString imgPath;
    QString convPath;
};

struct RenderOutput
{
    ImageType type;
    QImage img;
};

struct DiffData
{
    ImageType type;
    QImage img1;
    QImage img2;
};

struct DiffOutput
{
    ImageType type;
    uint value;
    float percent;
    QImage img;
};

using RenderResult = Either<RenderOutput, QString>;

Q_DECLARE_METATYPE(RenderOutput)
Q_DECLARE_METATYPE(DiffOutput)

class Render : public QObject
{
    Q_OBJECT

public:
    explicit Render(QObject *parent = nullptr);

    void setScale(qreal s);

    void render(const QString &path);

    void loadSettings();

    static QString imageTypeName(const ImageType t);

signals:
    void imageReady(ImageType, QImage);
    void diffReady(ImageType, QImage);
    void diffStats(ImageType, uint, float);
    void warning(QString);
    void error(QString);
    void finished();

private:
    void renderImages();

    static QImage loadImage(const QString &path);
    static QImage renderViaChrome(const RenderData &data);
    static QImage renderViaResvg(const RenderData &data);
    static QImage renderViaInkscape(const RenderData &data);
    static QImage renderViaRsvg(const RenderData &data);
    static QImage renderViaQtSvg(const RenderData &data);
    static RenderResult renderImage(const RenderData &data);
    static DiffOutput diffImage(const DiffData &data);

private slots:
    void onImageRendered(const int idx);
    void onImagesRendered();
    void onDiffResult(const int idx);
    void onDiffFinished();

private:
    int m_viewSize = 300;
    QFutureWatcher<RenderResult> m_watcher1;
    QFutureWatcher<DiffOutput> m_watcher2;
    QString m_imgPath;
    QHash<ImageType, QImage> m_imgs;

    struct Converters {
        QString resvg;
        QString inkscape;
        QString rsvg;
    } m_converters;
};

Q_DECL_PURE_FUNCTION inline uint qHash(const ImageType &key, uint seed = 0)
{ return qHash((uint)key, seed); }

QDebug operator<<(QDebug dbg, const ImageType &t);
