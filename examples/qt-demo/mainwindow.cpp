#include <QMessageBox>
#include <QTimer>
#include <QFile>
#include <QDebug>

#include "mainwindow.h"
#include "ui_mainwindow.h"

MainWindow::MainWindow(QWidget *parent) :
    QMainWindow(parent),
    ui(new Ui::MainWindow)
{
    ui->setupUi(this);

    SvgView::init();

    ui->cmbBoxBackground->setCurrentIndex(1);

    connect(ui->svgView, &SvgView::loadError, this, [this](const QString &msg){
        QMessageBox::critical(this, "Error", msg);
    });

    QTimer::singleShot(5, this, &MainWindow::onStart);
}

MainWindow::~MainWindow()
{
    delete ui;
}

void MainWindow::onStart()
{
    QFile file(":/hello-resvg.svg");
    file.open(QFile::ReadOnly);

    const QByteArray ba = file.readAll();
    ui->svgView->loadData(ba);
}

void MainWindow::on_rBtnRenderViaResvg_toggled(bool checked)
{
    ui->svgView->setBackend(checked ? RenderBackend::Resvg : RenderBackend::QtSvg);
}

void MainWindow::on_rBtnFitSize_toggled(bool checked)
{
    ui->svgView->setFitToView(checked);
}

void MainWindow::on_cmbBoxBackground_currentIndexChanged(int index)
{
    ui->svgView->setBackgound(SvgView::Backgound(index));
}

void MainWindow::on_chBoxDrawBorder_toggled(bool checked)
{
    ui->svgView->setDrawImageBorder(checked);
}
