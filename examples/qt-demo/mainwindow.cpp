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

    for (int i = 25; i <= 500; i += 25) {
        ui->cmbBoxZoom->addItem(QString("%1%").arg(i), i);
    }
    ui->cmbBoxZoom->setCurrentIndex(3);

    ui->cmbBoxBackground->setCurrentIndex(2);


    connect(ui->svgView, &SvgView::renderTime, this, [this](qint64 ns){
        ui->lblTime->setText(QString::number(ns / 1000000.0, 'f', 2) + "ms");
    });

    connect(ui->svgView, &SvgView::loadError, this, [this](const QString &msg){
        QMessageBox::critical(this, "Error", msg);
    });

    updateState();

    QTimer::singleShot(5, this, &MainWindow::onStart);
}

MainWindow::~MainWindow()
{
    delete ui;
}

void MainWindow::onStart()
{
    QFile file(":/Ghostscript_Tiger.svg");
    file.open(QFile::ReadOnly);

    const QByteArray ba = file.readAll();
    ui->svgView->loadData(ba);
}

void MainWindow::on_rBtnRenderToImage_toggled(bool checked)
{
    ui->svgView->setRenderToImage(checked);
    updateState();
}

void MainWindow::on_rBtnFitSize_toggled(bool checked)
{
    ui->svgView->setFitToView(checked);
    updateState();
}

void MainWindow::on_cmbBoxZoom_currentIndexChanged(int /*index*/)
{
    const auto zoom = ui->cmbBoxZoom->currentData().toInt();
    ui->svgView->setZoom(zoom / 100.0f);
}

void MainWindow::on_cmbBoxBackground_currentIndexChanged(int index)
{
    ui->svgView->setBackgound(SvgView::Backgound(index));
}

void MainWindow::on_chBoxDrawBorder_toggled(bool checked)
{
    ui->svgView->setDrawImageBorder(checked);
}

void MainWindow::updateState()
{
    ui->widgetSize->setEnabled(ui->rBtnRenderToWidget->isChecked());
    ui->cmbBoxZoom->setEnabled(ui->rBtnOrigSize->isChecked() && ui->widgetSize->isEnabled());
}
