#include <QMessageBox>
#include <QGuiApplication>
#include <QScreen>
#include <QElapsedTimer>
#include <QTextLayout>
#include <QPainter>
#include <QFileInfo>
#include <QMimeData>
#include <QTimer>
#include <QThread>
#include <QDebug>

#include "svgview.h"

SvgViewWorker::SvgViewWorker(QObject *parent)
    : QObject(parent)
    , m_dpiRatio(qApp->screens().first()->devicePixelRatio())
{
}

QRect SvgViewWorker::viewBox() const
{
    QMutexLocker lock(&m_mutex);
    return m_renderer.viewBox();
}

void SvgViewWorker::loadData(const QByteArray &data)
{
    QMutexLocker lock(&m_mutex);

    m_renderer.load(data);
    if (!m_renderer.isValid()) {
        emit errorMsg(m_renderer.errorString());
    }

    m_qtRenderer.load(data);
}

void SvgViewWorker::loadFile(const QString &path)
{
    QMutexLocker lock(&m_mutex);

    m_renderer.load(path);
    if (!m_renderer.isValid()) {
        emit errorMsg(m_renderer.errorString());
    }

    m_qtRenderer.load(path);
}

void SvgViewWorker::render(const QSize &viewSize, RenderBackend backend)
{
    Q_ASSERT(QThread::currentThread() != qApp->thread());

    QMutexLocker lock(&m_mutex);

    if (backend == RenderBackend::Resvg) {
        if (m_renderer.isEmpty()) {
            return;
        }

        const auto s = m_renderer.defaultSize().scaled(viewSize, Qt::KeepAspectRatio);
        QImage img(s * m_dpiRatio, QImage::Format_ARGB32_Premultiplied);
        img.fill(Qt::transparent);

        QPainter p;
        p.begin(&img);
        p.setRenderHint(QPainter::Antialiasing);
        m_renderer.render(&p);
        p.end();

        img.setDevicePixelRatio(m_dpiRatio);

        emit rendered(img);
    } else {
        if (!m_qtRenderer.isValid()) {
            return;
        }

        const auto s = m_qtRenderer.defaultSize().scaled(viewSize, Qt::KeepAspectRatio);
        QImage img(s * m_dpiRatio, QImage::Format_ARGB32_Premultiplied);
        img.fill(Qt::transparent);

        QPainter p;
        p.begin(&img);
        p.setRenderHint(QPainter::Antialiasing);
        m_qtRenderer.render(&p);
        p.end();

        img.setDevicePixelRatio(m_dpiRatio);

        emit rendered(img);
    }
}

static QImage genCheckedTexture()
{
    int l = 20;

    QImage pix = QImage(l, l, QImage::Format_RGB32);
    int b = pix.width() / 2.0;
    pix.fill(QColor("#c0c0c0"));

    QPainter p;
    p.begin(&pix);
    p.fillRect(QRect(0,0,b,b), QColor("#808080"));
    p.fillRect(QRect(b,b,b,b), QColor("#808080"));
    p.end();

    return pix;
}

SvgView::SvgView(QWidget *parent)
    : QFrame(parent)
    , m_checkboardImg(genCheckedTexture())
    , m_worker(new SvgViewWorker())
{
    setAcceptDrops(true);
    setMinimumSize(10, 10);

    QThread *th = new QThread(this);
    m_worker->moveToThread(th);
    th->start();

    const auto *screen = qApp->screens().first();
    m_dpiRatio = screen->devicePixelRatio();

    connect(m_worker, &SvgViewWorker::rendered, this, &SvgView::onRendered);
}

SvgView::~SvgView()
{
    QThread *th = m_worker->thread();
    th->quit();
    th->wait(10000);
    delete m_worker;
}

void SvgView::init()
{
    ResvgRenderer::initLog();
}

void SvgView::setFitToView(bool flag)
{
    m_isFitToView = flag;
    requestUpdate();
}

void SvgView::setBackgound(SvgView::Backgound backgound)
{
    m_backgound = backgound;
    update();
}

void SvgView::setDrawImageBorder(bool flag)
{
    m_isDrawImageBorder = flag;
    update();
}

void SvgView::setBackend(RenderBackend backend)
{
    m_backend = backend;
    requestUpdate();
}

void SvgView::loadData(const QByteArray &ba)
{
    m_worker->loadData(ba);
    requestUpdate();
}

void SvgView::loadFile(const QString &path)
{
    m_worker->loadFile(path);
    requestUpdate();
}

void SvgView::paintEvent(QPaintEvent *e)
{
    if (m_img.isNull()) {
        QPainter p(this);
        p.drawText(rect(), Qt::AlignCenter, "Drop an SVG image here.");

        QFrame::paintEvent(e);
        return;
    }

    QPainter p(this);
    const auto r = contentsRect();
    p.setClipRect(r);

    switch (m_backgound) {
        case Backgound::None : break;
        case Backgound::White : {
            p.fillRect(contentsRect(), Qt::white);
        } break;
        case Backgound::CheckBoard : {
            p.fillRect(contentsRect(), QBrush(m_checkboardImg));
        } break;
    }

    const QRect imgRect(0, 0, m_img.width() / m_dpiRatio, m_img.height() / m_dpiRatio);

    p.translate(r.x() + (r.width() - imgRect.width())/ 2,
                r.y() + (r.height() - imgRect.height()) / 2);

    p.drawImage(0, 0, m_img);

    if (m_isDrawImageBorder) {
        p.setRenderHint(QPainter::Antialiasing, false);
        p.setPen(Qt::green);
        p.setBrush(Qt::NoBrush);
        p.drawRect(imgRect);
    }

    QFrame::paintEvent(e);
}

void SvgView::dragEnterEvent(QDragEnterEvent *event)
{
    event->accept();
}

void SvgView::dragMoveEvent(QDragMoveEvent *event)
{
    event->accept();
}

void SvgView::dropEvent(QDropEvent *event)
{
    const QMimeData *mime = event->mimeData();
    if (!mime->hasUrls()) {
        event->ignore();
        return;
    }

    for (const QUrl &url : mime->urls()) {
        if (!url.isLocalFile()) {
            continue;
        }

        QString path = url.toLocalFile();
        QFileInfo fi = QFileInfo(path);
        if (fi.isSymLink()) {
            continue;
        }

        if (fi.isFile()) {
            QString suffix = QFileInfo(path).suffix().toLower();
            if (suffix == "svg" || suffix == "svgz") {
                loadFile(path);
            } else {
                QMessageBox::warning(this, tr("Warning"),
                                     tr("You can drop only SVG and SVGZ files."));
            }
        }
    }

    event->acceptProposedAction();
}

void SvgView::resizeEvent(QResizeEvent *)
{
    requestUpdate();
}

void SvgView::requestUpdate()
{
    const auto s = m_isFitToView ? size() : m_worker->viewBox().size();

    // Run method in the m_worker thread scope.
    QTimer::singleShot(1, m_worker, [=](){
        m_worker->render(s, m_backend);
    });
}

void SvgView::onRendered(const QImage &img)
{
    m_img = img;
    update();
}

