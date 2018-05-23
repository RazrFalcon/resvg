#pragma once

#include <QFrame>
#include <QSvgRenderer>
#include <QMutex>

#include <ResvgQt.h>

enum class RenderBackend
{
    Resvg,
    QtSvg,
};

class SvgViewWorker : public QObject
{
    Q_OBJECT

public:
    SvgViewWorker(QObject *parent = nullptr);

    QRect viewBox() const;

public slots:
    void loadData(const QByteArray &data);
    void loadFile(const QString &path);
    void render(const QSize &viewSize, RenderBackend backend);

signals:
    void rendered(QImage);
    void errorMsg(QString);

private:
    const float m_dpiRatio;
    mutable QMutex m_mutex;
    ResvgRenderer m_renderer;
    QSvgRenderer m_qtRenderer;
};

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

    void setFitToView(bool flag);
    void setBackgound(Backgound backgound);
    void setDrawImageBorder(bool flag);
    void setBackend(RenderBackend backend);

    void loadData(const QByteArray &data);
    void loadFile(const QString &path);

signals:
    void loadError(QString);

protected:
    void paintEvent(QPaintEvent *);
    void dragEnterEvent(QDragEnterEvent *event);
    void dragMoveEvent(QDragMoveEvent *event);
    void dropEvent(QDropEvent *event);
    void resizeEvent(QResizeEvent *);

private:
    void requestUpdate();

private slots:
    void onRendered(const QImage &img);

private:
    const QImage m_checkboardImg;
    SvgViewWorker * const m_worker;

    QString m_path;
    RenderBackend m_backend = RenderBackend::Resvg;
    float m_dpiRatio = 1.0;
    bool m_isFitToView = true;
    Backgound m_backgound = Backgound::CheckBoard;
    bool m_isDrawImageBorder = false;
    QImage m_img;
};
