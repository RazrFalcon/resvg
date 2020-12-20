QT += core gui widgets

TARGET = viewsvg
TEMPLATE = app
CONFIG += c++11

SOURCES += \
    main.cpp \
    mainwindow.cpp \
    svgview.cpp

HEADERS += \
    mainwindow.h \
    svgview.h

FORMS += \
    mainwindow.ui

CONFIG(release, debug|release): LIBS += -L$$PWD/../../target/release/ -lresvg
else:CONFIG(debug, debug|release): LIBS += -L$$PWD/../../target/debug/ -lresvg

windows:LIBS += -lWs2_32 -lAdvapi32 -lUserenv

INCLUDEPATH += $$PWD/../../c-api
DEPENDPATH += $$PWD/../../c-api
