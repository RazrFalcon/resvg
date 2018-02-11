#include <QMessageBox>
#include <QTimer>
#include <QSettings>
#include <QFileInfo>
#include <QFileDialog>
#include <QDir>
#include <QScreen>
#include <QDebug>

#include "settingsdialog.h"
#include "process.h"

#include "mainwindow.h"
#include "ui_mainwindow.h"

MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent)
    , ui(new Ui::MainWindow)
{
    ui->setupUi(this);

    m_render.setScale(qApp->screens().first()->devicePixelRatio());

    adjustSize();

    m_imgViews.insert(ImageType::Chrome, ui->imgViewChrome);
    m_imgViews.insert(ImageType::ResvgCairo, ui->imgViewResvgCairo);
    m_imgViews.insert(ImageType::ResvgQt, ui->imgViewResvgQt);
    m_imgViews.insert(ImageType::Inkscape, ui->imgViewInkscape);
    m_imgViews.insert(ImageType::Rsvg, ui->imgViewRsvg);
    m_imgViews.insert(ImageType::QtSvg, ui->imgViewQtSvg);

    m_diffViews.insert(ImageType::ResvgCairo, ui->imgViewResvgCairoDiff);
    m_diffViews.insert(ImageType::ResvgQt, ui->imgViewResvgQtDiff);
    m_diffViews.insert(ImageType::Inkscape, ui->imgViewInkscapeDiff);
    m_diffViews.insert(ImageType::Rsvg, ui->imgViewRsvgDiff);
    m_diffViews.insert(ImageType::QtSvg, ui->imgViewQtSvgDiff);

    m_diffLabels.insert(ImageType::ResvgCairo, ui->lblResvgCairoDiff);
    m_diffLabels.insert(ImageType::ResvgQt, ui->lblResvgQtDiff);
    m_diffLabels.insert(ImageType::Inkscape, ui->lblInkscapeDiff);
    m_diffLabels.insert(ImageType::Rsvg, ui->lblRsvgDiff);
    m_diffLabels.insert(ImageType::QtSvg, ui->lblQtSvgDiff);

    connect(&m_render, &Render::imageReady, this, &MainWindow::onImageReady);
    connect(&m_render, &Render::diffReady, this, &MainWindow::onDiffReady);
    connect(&m_render, &Render::diffStats, this, &MainWindow::onDiffStats);
    connect(&m_render, &Render::warning, this, &MainWindow::onRenderWarning);
    connect(&m_render, &Render::error, this, &MainWindow::onRenderError);
    connect(&m_render, &Render::finished, this, &MainWindow::onRenderFinished);

    initDefaultSettings();

    // TODO: check that convertors exists

    QTimer::singleShot(5, this, &MainWindow::onStart);
}

MainWindow::~MainWindow()
{
    delete ui;
}

void MainWindow::initDefaultSettings()
{
    QSettings settings;
    if (!settings.contains("ResvgPath")) {
        settings.setValue("ResvgBuild", "debug");
        settings.setValue("ResvgPath", QString(SRCDIR) + "../../tools/rendersvg/target/debug/rendersvg");
        settings.setValue("InkscapePath", "inkscape");
        settings.setValue("RsvgPath", "rsvg-convert");

        m_render.loadSettings();
    }
}

void MainWindow::setGuiEnabled(bool flag)
{
    ui->btnOpen->setEnabled(flag);
    ui->btnSettings->setEnabled(flag);
    ui->cmbBoxFiles->setEnabled(flag);
}

void MainWindow::onStart()
{
    QSettings settings;

    const QString path = settings.value("Path").toString();

    if (!path.isEmpty() && QFileInfo::exists(path) && QFileInfo(path).isDir()) {
        loadImageList(path);
    }
}

void MainWindow::on_btnOpen_clicked()
{
    QSettings settings;

    QString dir = settings.value("Path", QDir::homePath()).toString();
    if (!QFileInfo::exists(dir)) {
        dir = QDir::homePath();
    }

    const QString path = QFileDialog::getExistingDirectory(this, "Select Folder", dir);
    if (path.isEmpty()) {
        return;
    }

    settings.setValue("Path", path);

    loadImageList(path);
}

void MainWindow::loadImageList(const QString &path)
{
    ui->cmbBoxFiles->clear();

    const auto list = QDir(path).entryInfoList({ "*.svg" }, QDir::Files, QDir::Name);
    for (const auto info : list) {
        ui->cmbBoxFiles->addItem(info.completeBaseName(), info.absoluteFilePath());
    }

    if (ui->cmbBoxFiles->count() != 0) {
        const QString currPath = ui->cmbBoxFiles->currentData().toString();
        loadImage(currPath);
    }

    ui->lineEdit->setText(path);

    ui->cmbBoxFiles->setFocus();
}

void MainWindow::on_cmbBoxFiles_currentIndexChanged(int)
{
    const QString currPath = ui->cmbBoxFiles->currentData().toString();
    loadImage(currPath);
}

void MainWindow::loadImage(const QString &path)
{
    setAnimationEnabled(true);
    resetImages();

    m_render.render(path);

    setGuiEnabled(false);
}

void MainWindow::setAnimationEnabled(bool flag)
{
    for (ImageView *view : m_imgViews) {
        view->setAnimationEnabled(flag);
    }

    for (ImageView *view : m_diffViews) {
        view->setAnimationEnabled(flag);
    }
}

void MainWindow::resetImages()
{
    for (ImageView *view : m_imgViews) {
        view->resetImage();
    }

    for (ImageView *view : m_diffViews) {
        view->resetImage();
    }
}

void MainWindow::setDiffText(QLabel *lbl, uint diff, float percent) const
{
    lbl->setText(QString::number(diff) + "/" + QString::number(percent, 'f', 2) + "%");
}

void MainWindow::onImageReady(const ImageType type, const QImage &img)
{
    Q_ASSERT(!img.isNull());

    const auto view = m_imgViews.value(type);
    view->setAnimationEnabled(false);
    view->setImage(img);
}

void MainWindow::onDiffReady(const ImageType type, const QImage &img)
{
    const auto view = m_diffViews.value(type);
    view->setAnimationEnabled(false);
    view->setImage(img);
}

void MainWindow::onDiffStats(const ImageType type, const uint value, const float percent)
{
    setDiffText(m_diffLabels.value(type), value, percent);
}

void MainWindow::onRenderWarning(const QString &msg)
{
    QMessageBox::warning(this, "Warning", msg);
}

void MainWindow::onRenderError(const QString &msg)
{
    QMessageBox::critical(this, "Error", msg);
}

void MainWindow::onRenderFinished()
{
    setGuiEnabled(true);
    ui->cmbBoxFiles->setFocus();

    setAnimationEnabled(false);
}

void MainWindow::on_btnSettings_clicked()
{
    SettingsDialog diag(this);
    if (diag.exec()) {
        m_render.loadSettings();
    }
}
