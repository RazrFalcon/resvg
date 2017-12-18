#pragma once

#include <QWidget>
#include <QBasicTimer>

class ImageView : public QWidget
{
    Q_OBJECT

public:
    explicit ImageView(QWidget *parent = nullptr);

    void setAnimationEnabled(bool flag);

    void setImage(const QImage &img);
    void resetImage();

protected:
    void paintEvent(QPaintEvent *) override;
    void timerEvent(QTimerEvent *event) override;

private:
    const qreal m_scale;

    QBasicTimer m_timer;
    int m_angle = 0;
    QImage m_img;
};
