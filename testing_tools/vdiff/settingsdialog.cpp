#include <QSettings>
#include <QFileDialog>
#include <QDebug>

#include "settingsdialog.h"
#include "ui_settingsdialog.h"

SettingsDialog::SettingsDialog(QWidget *parent)
    : QDialog(parent)
    , ui(new Ui::SettingsDialog)
{
    ui->setupUi(this);

    loadSettings();
    adjustSize();
}

SettingsDialog::~SettingsDialog()
{
    delete ui;
}

void SettingsDialog::loadSettings()
{
    QSettings settings;
    ui->rBtnRelease->setChecked(settings.value("ResvgBuild").toString() == "release");

    ui->lineEditInkscape->setText(settings.value("InkscapePath").toString());
    ui->lineEditRsvg->setText(settings.value("RsvgPath").toString());
}

void SettingsDialog::on_buttonBox_accepted()
{
    QSettings settings;

    QString resvgBuild = ui->rBtnDebug->isChecked() ? "debug" : "release";
    settings.setValue("ResvgBuild", resvgBuild);

    QString resvgPath = QString(SRCDIR) + "../../tools/rendersvg/target/%1/rendersvg";

    settings.setValue("ResvgPath", resvgPath.arg(resvgBuild));
    settings.setValue("InkscapePath", ui->lineEditInkscape->text());
    settings.setValue("RsvgPath", ui->lineEditRsvg->text());
}

void SettingsDialog::on_btnSelectInkscape_clicked()
{
    const QString path = QFileDialog::getOpenFileName(this, "Inkscape exe path");
    if (!path.isEmpty()) {
        ui->lineEditInkscape->setText(path);
    }
}

void SettingsDialog::on_btnSelectRsvg_clicked()
{
    const QString path = QFileDialog::getOpenFileName(this, "rsvg-convert exe path");
    if (!path.isEmpty()) {
        ui->lineEditRsvg->setText(path);
    }
}
