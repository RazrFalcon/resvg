#pragma once

#include <QFrame>

struct resvg_render_tree;

class SvgView : public QFrame
{
    Q_OBJECT

public:
    enum class Backgound
    {
        None,
        White,
        CheckBoard,
    };

    explicit SvgView(QWidget *parent = nullptr);
    ~SvgView();

    static void init();

    void setRenderToImage(bool flag);
    void setFitToView(bool flag);
    void setZoom(float zoom);
    void setBackgound(Backgound backgound);
    void setDrawImageBorder(bool flag);

    void loadData(const QByteArray &ba);
    void loadFile(const QString &path);

signals:
    void renderTime(qint64);
    void loadError(QString);

protected:
    void paintEvent(QPaintEvent *);
    void dragEnterEvent(QDragEnterEvent *event);
    void dragMoveEvent(QDragMoveEvent *event);
    void dropEvent(QDropEvent *event);

private:
    const QImage m_checkboardImg;

    resvg_render_tree *m_rtree = nullptr;

    bool m_isFitToView = true;
    float m_zoom = 1.0f;
    Backgound m_backgound = Backgound::CheckBoard;
    bool m_isDrawImageBorder = false;

    bool m_isRenderToImage = false;
    QPixmap m_pix;

};
