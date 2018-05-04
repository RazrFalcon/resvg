#include "mainwindow.h"
#include "ui_mainwindow.h"

MainWindow::MainWindow(QWidget *parent) :
    QMainWindow(parent),
    ui(new Ui::MainWindow)
{
    ui->setupUi(this);

    ResvgRenderer::initLog();

    ui->svgViewBasic->setRenderMode(RenderMode::Basic);
    ui->resvgViewBasic->setRenderMode(RenderMode::Basic);
    ui->resvgViewBasic->setRenderType(RenderType::Resvg);

    ui->svgViewBasicWithBounds->setRenderMode(RenderMode::BasicWithBounds);
    ui->resvgViewBasicWithBounds->setRenderMode(RenderMode::BasicWithBounds);
    ui->resvgViewBasicWithBounds->setRenderType(RenderType::Resvg);

    ui->svgViewById->setRenderMode(RenderMode::ById);
    ui->resvgViewById->setRenderMode(RenderMode::ById);
    ui->resvgViewById->setRenderType(RenderType::Resvg);

    ui->svgViewByIdWithBounds->setRenderMode(RenderMode::ByIdWithBounds);
    ui->resvgViewByIdWithBounds->setRenderMode(RenderMode::ByIdWithBounds);
    ui->resvgViewByIdWithBounds->setRenderType(RenderType::Resvg);
}

MainWindow::~MainWindow()
{
    delete ui;
}
