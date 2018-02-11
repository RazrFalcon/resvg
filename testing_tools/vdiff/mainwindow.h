#pragma once

#include <QMainWindow>

#include "render.h"

namespace Ui {
class MainWindow;
}

class QLabel;

class ImageView;

class MainWindow : public QMainWindow
{
    Q_OBJECT

public:
    explicit MainWindow(QWidget *parent = 0);
    ~MainWindow();

private:
    void initDefaultSettings();
    void setGuiEnabled(bool flag);
    void loadImageList(const QString &path);
    void resetImages();
    void loadImage(const QString &path);
    void setAnimationEnabled(bool flag);

    void setDiffText(QLabel *lbl, uint diff, float percent) const;

private slots:
    void onStart();
    void on_btnOpen_clicked();
    void on_cmbBoxFiles_currentIndexChanged(int index);
    void onImageReady(const ImageType type, const QImage &img);
    void onDiffReady(const ImageType type, const QImage &img);
    void onDiffStats(const ImageType type, const uint value, const float percent);
    void onRenderWarning(const QString &msg);
    void onRenderError(const QString &msg);
    void onRenderFinished();

    void on_btnSettings_clicked();

private:
    Ui::MainWindow * const ui;

    QHash<ImageType, ImageView*> m_imgViews;
    QHash<ImageType, ImageView*> m_diffViews;
    QHash<ImageType, QLabel*> m_diffLabels;

    Render m_render;
};
