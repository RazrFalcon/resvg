QT      += core gui widgets svg concurrent

TARGET   = vdiff
TEMPLATE = app

CONFIG += C++11

SOURCES  += \
    imageview.cpp \
    main.cpp \
    mainwindow.cpp \
    process.cpp \
    render.cpp \
    settingsdialog.cpp

HEADERS  += \
    imageview.h \
    mainwindow.h \
    process.h \
    render.h \
    settingsdialog.h \
    either.h

FORMS    += \
    mainwindow.ui \
    settingsdialog.ui

DEFINES += SRCDIR=\\\"$$PWD/\\\"
