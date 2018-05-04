QT += core gui widgets

TARGET = demo
TEMPLATE = app
CONFIG += C++11

SOURCES += \
    main.cpp \
    mainwindow.cpp \
    svgview.cpp \
    ../../capi/qt-wrapper/ResvgQt.cpp

HEADERS += \
    mainwindow.h \
    svgview.h

FORMS += \
    mainwindow.ui

RESOURCES += \
    resources.qrc

CONFIG(release, debug|release): LIBS += -L$$PWD/../../target/release/ -lresvg
else:CONFIG(debug, debug|release): LIBS += -L$$PWD/../../target/debug/ -lresvg

INCLUDEPATH += $$PWD/../../capi/include
DEPENDPATH += $$PWD/../../capi/include

INCLUDEPATH += $$PWD/../../capi/qt-wrapper
