#include <QMessageBox>
#include <QGuiApplication>
#include <QScreen>
#include <QElapsedTimer>
#include <QTextLayout>
#include <QPainter>
#include <QFileInfo>
#include <QMimeData>
#include <QDebug>

#define RESVG_QT_BACKEND

extern "C" {
#include <resvg.h>
}

#include "svgview.h"

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
{
    setAcceptDrops(true);
    setMinimumSize(10, 10);
}

SvgView::~SvgView()
{
    if (m_rtree) {
        resvg_rtree_destroy(m_rtree);
    }

    if (m_opt) {
        delete m_opt;
    }
}

void SvgView::init()
{
    resvg_init_log();
}

void SvgView::setRenderToImage(bool flag)
{
    m_isRenderToImage = flag;

    if (!flag) {
        m_pix = QPixmap();
        update();
        return;
    }

    if (!m_rtree) {
        return;
    }

    const auto *screen = qApp->screens().first();
    const auto ratio = screen->devicePixelRatio();

    auto size = resvg_get_image_size(m_rtree);
    size.width *= ratio;
    size.height *= ratio;

    QImage img(size.width, size.height, QImage::Format_ARGB32_Premultiplied);
    img.fill(Qt::transparent);


    QPainter p;
    p.begin(&img);
    p.setRenderHint(QPainter::Antialiasing);
    resvg_qt_render_to_canvas(m_rtree, m_opt, size, &p);
    p.end();

    img.setDevicePixelRatio(ratio);

    m_pix = QPixmap::fromImage(img);
    update();
}

void SvgView::setFitToView(bool flag)
{
    m_isFitToView = flag;
    update();
}

void SvgView::setZoom(float zoom)
{
    m_zoom = zoom;
    update();
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

void SvgView::loadData(const QByteArray &ba)
{
    if (m_rtree) {
        resvg_rtree_destroy(m_rtree);
    }

    const auto *screen = qApp->screens().first();
    const double dpi = screen->logicalDotsPerInch() * screen->devicePixelRatio();

    m_opt = new resvg_options;
    resvg_init_options(m_opt);
    m_opt->dpi = dpi;

    char *err = nullptr;
    m_rtree = resvg_parse_rtree_from_data(ba.constData(), m_opt, &err);
    if (!m_rtree) {
        m_opt = nullptr;
        emit loadError(QString::fromUtf8(err));
        resvg_error_msg_destroy(err);
    }

    update();
}

void SvgView::loadFile(const QString &path)
{
    if (m_rtree) {
        resvg_rtree_destroy(m_rtree);
    }

    const auto *screen = qApp->screens().first();
    const double dpi = screen->logicalDotsPerInch() * screen->devicePixelRatio();

    m_opt = new resvg_options;
    resvg_init_options(m_opt);
    m_opt->dpi = dpi;

    char *err = nullptr;
    std::string utf8Path = path.toUtf8().constData();
    m_rtree = resvg_parse_rtree_from_file(utf8Path.c_str(), m_opt, &err);
    if (!m_rtree) {
        m_opt = nullptr;
        emit loadError(QString::fromUtf8(err));
        resvg_error_msg_destroy(err);
    }

    update();
}

void SvgView::paintEvent(QPaintEvent *e)
{
    if (!m_rtree) {
        QPainter p(this);
        p.drawText(rect(), Qt::AlignCenter, "Drop an SVG image here.");

        QFrame::paintEvent(e);
        return;
    }

    QElapsedTimer timer;
    timer.start();

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

    QRect imgRect;
    if (m_pix.isNull()) {
        p.setRenderHint(QPainter::Antialiasing);

        double x = r.x();
        double y = r.y();
        double img_width, img_height;
        if (m_isFitToView) {
            img_width = r.width();
            img_height = r.height();
        } else {
            const auto size = resvg_get_image_size(m_rtree);
            img_width = size.width * m_zoom;
            img_height = size.height * m_zoom;

            x = (r.width() - img_width)/2;
            y = (r.height() - img_height)/2;
        }

        p.translate(x, y);
        resvg_size s { (uint)img_width, (uint)img_height };
        resvg_qt_render_to_canvas(m_rtree, m_opt, s, &p);
        p.setTransform(QTransform());

        imgRect = QRect(x, y, img_width, img_height);
    } else {
        const auto ratio = m_pix.devicePixelRatio();

        double x = (r.width() - m_pix.width() / ratio)/2;
        double y = (r.height() - m_pix.height() / ratio)/2;

        p.drawPixmap(x, y, m_pix);

        imgRect = QRect(x, y, m_pix.width() / ratio, m_pix.height() / ratio);
    }

    emit renderTime(timer.nsecsElapsed());

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
