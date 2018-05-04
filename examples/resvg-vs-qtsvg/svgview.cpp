#include <QPainter>
#include <QFile>
#include <QDebug>

#include "svgview.h"

SvgView::SvgView(QWidget *parent)
    : QWidget(parent)
{
    setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Expanding);

    m_svgRender.load(QLatin1Literal(":/test.svg"));

    m_resvgRender.load(QLatin1Literal(":/test.svg"));
    Q_ASSERT(m_resvgRender.isValid());
    Q_ASSERT(!m_resvgRender.isEmpty());
}

template<class T>
static void renderVia(QPainter &p, RenderMode mode, const QSize &size, T &r)
{
    switch (mode) {
        case RenderMode::Basic : {
            r.render(&p);
        } break;

        case RenderMode::BasicWithBounds : {
            r.render(&p, QRect(10, 20, size.width()/2, size.height()/2));
        } break;

        case RenderMode::ById : {
            r.render(&p, "circle1");
        } break;

        case RenderMode::ByIdWithBounds : {
            r.render(&p, "circle1", QRect(10, 20, size.width()/2, size.height()/2));
        } break;
    }
}

void SvgView::paintEvent(QPaintEvent *)
{
    QPainter p(this);
    p.fillRect(rect(), Qt::white);

    p.translate(10, 20);
    p.scale(1.25, 0.75);

    if (m_rendererType == RenderType::QtSvg) {
        renderVia(p, m_mode, size(), m_svgRender);
    } else {
        renderVia(p, m_mode, size(), m_resvgRender);
    }
}
