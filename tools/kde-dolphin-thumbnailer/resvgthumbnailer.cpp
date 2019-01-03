#include "resvgthumbnailer.h"

#include <ResvgQt.h>

#include <QPainter>

extern "C"
{
    Q_DECL_EXPORT ThumbCreator *new_creator()
    {
        return new ResvgThumbnailer;
    }
}

bool ResvgThumbnailer::create(const QString& path, int width, int heigth, QImage& img)
{
    ResvgRenderer renderer(path);
    if (!renderer.isValid() || renderer.isEmpty()) {
        return false;
    }

    const double ratio = static_cast<double>(renderer.defaultSize().height()) /
                         static_cast<double>(renderer.defaultSize().width());
    if (width < heigth)
        heigth = qRound(ratio * width);
    else
        width = qRound(heigth / ratio);

    QImage previewImage(width, heigth, QImage::Format_ARGB32_Premultiplied);
    previewImage.fill(Qt::transparent);

    QPainter p(&previewImage);
    renderer.render(&p);
    p.end();

    img = previewImage;
    return true;
}

ThumbCreator::Flags ResvgThumbnailer::flags() const
{
    return (Flags)(None);
}
