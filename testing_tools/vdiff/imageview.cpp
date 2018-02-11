#include <QGuiApplication>
#include <QScreen>
#include <QPainter>
#include <QTimerEvent>

#include "imageview.h"

ImageView::ImageView(QWidget *parent)
    : QWidget(parent)
    , m_scale(qApp->screens().first()->devicePixelRatio())
{
}

void ImageView::setAnimationEnabled(bool flag)
{
    if (flag) {
        m_angle = 0;
        m_timer.start(100, this);
    } else {
        m_timer.stop();
    }

    update();
}

void ImageView::setImage(const QImage &img)
{
    m_img = img;
    m_img.setDevicePixelRatio(m_scale);
    update();
}

void ImageView::resetImage()
{
    setImage(QImage());
}

void ImageView::paintEvent(QPaintEvent *)
{
    QPainter p(this);

    p.fillRect(rect(), Qt::white);

    if (!m_img.isNull()) {
        p.drawImage(0, 0, m_img);
    }

    if (m_timer.isActive()) {
        int outerRadius = height() * 0.1;
        int innerRadius = outerRadius * 0.45;

        int capsuleHeight = outerRadius - innerRadius;
        int capsuleWidth  = capsuleHeight * .35;
        int capsuleRadius = capsuleWidth / 2;

        for (int i = 0; i < 12; ++i) {
            QColor color = Qt::black;
            color.setAlphaF(1.0f - (i / 12.0f));
            p.setRenderHint(QPainter::Antialiasing);
            p.setPen(Qt::NoPen);
            p.setBrush(color);
            p.save();
            p.translate(width()/2, height()/2);
            p.rotate(m_angle - i * 30.0f);
            p.drawRoundedRect(-capsuleWidth * 0.5, -(innerRadius + capsuleHeight), capsuleWidth,
                               capsuleHeight, capsuleRadius, capsuleRadius);
            p.restore();
        }
    }
}

void ImageView::timerEvent(QTimerEvent *event)
{
    if (event->timerId() == m_timer.timerId()) {
        m_angle = (m_angle + 30) % 360;
        update();
    } else {
        QWidget::timerEvent(event);
    }
}
