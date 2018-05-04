#pragma once

#include <QWidget>
#include <QSvgRenderer>

#include <ResvgQt.h>

enum class RenderMode {
    Basic,
    BasicWithBounds,
    ById,
    ByIdWithBounds,
};

enum class RenderType {
    Resvg,
    QtSvg,
};

class SvgView : public QWidget
{
    Q_OBJECT

public:
    explicit SvgView(QWidget *parent = nullptr);

    void setRenderMode(RenderMode mode) { m_mode = mode; }
    void setRenderType(RenderType type) { m_rendererType = type; }

protected:
    void paintEvent(QPaintEvent *);

private:
    RenderMode m_mode = RenderMode::Basic;
    RenderType m_rendererType = RenderType::QtSvg;
    QSvgRenderer m_svgRender;
    ResvgRenderer m_resvgRender;
};
