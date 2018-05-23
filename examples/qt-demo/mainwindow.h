#pragma once

#include <QMainWindow>

namespace Ui {
class MainWindow;
}

class MainWindow : public QMainWindow
{
    Q_OBJECT

public:
    explicit MainWindow(QWidget *parent = 0);
    ~MainWindow();

private slots:
    void onStart();
    void on_rBtnFitSize_toggled(bool checked);
    void on_cmbBoxBackground_currentIndexChanged(int index);
    void on_chBoxDrawBorder_toggled(bool checked);
    void on_rBtnRenderViaResvg_toggled(bool checked);

private:
    Ui::MainWindow *ui;
};
