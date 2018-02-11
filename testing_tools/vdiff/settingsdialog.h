#pragma once

#include <QDialog>

namespace Ui {
class SettingsDialog;
}

class SettingsDialog : public QDialog
{
    Q_OBJECT

public:
    explicit SettingsDialog(QWidget *parent = 0);
    ~SettingsDialog();

private:
    void loadSettings();

private slots:
    void on_buttonBox_accepted();
    void on_btnSelectInkscape_clicked();
    void on_btnSelectRsvg_clicked();

private:
    Ui::SettingsDialog * const ui;
};
