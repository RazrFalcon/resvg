#include "svgrthumbnailer.h"

#include <QPainter>

extern "C"
{
    Q_DECL_EXPORT ThumbCreator *new_creator()
    {
        return new ResvgThumbnailer;
    }
}

ResvgThumbnailer::ResvgThumbnailer()
{
    m_opt.loadSystemFonts();
}

bool ResvgThumbnailer::create(const QString& path, int width, int heigth, QImage& img)
{
    ResvgRenderer renderer(path, m_opt);
    if (!renderer.isValid() || renderer.isEmpty()) {
        return false;
    }

    img = renderer.renderToImage(QSize(width, heigth) * img.devicePixelRatio());
    return true;
}

ThumbCreator::Flags ResvgThumbnailer::flags() const
{
    return (Flags)(None);
}
