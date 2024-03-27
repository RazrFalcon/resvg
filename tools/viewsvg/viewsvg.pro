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

CONFIG(release, debug|release): LIBS += -L$$PWD/../../target/release/ -lsvgr
else:CONFIG(debug, debug|release): LIBS += -L$$PWD/../../target/debug/ -lsvgr

windows:LIBS += -lWs2_32 -lAdvapi32 -lBcrypt -lUserenv -lNtdll

INCLUDEPATH += $$PWD/../../crates/c-api
DEPENDPATH += $$PWD/../../crates/c-api
