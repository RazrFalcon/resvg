QT      += core gui widgets svg
TARGET   = resvg-vs-qtsvg
TEMPLATE = app

SOURCES += \
    main.cpp \
    svgview.cpp \
    mainwindow.cpp \
    ../../capi/qt-wrapper/ResvgQt.cpp

HEADERS += \
    svgview.h \
    mainwindow.h

FORMS += \
    mainwindow.ui

RESOURCES += \
    icons.qrc

CONFIG(release, debug|release): LIBS += -L$$PWD/../../target/release/ -lresvg
else:CONFIG(debug, debug|release): LIBS += -L$$PWD/../../target/debug/ -lresvg

INCLUDEPATH += $$PWD/../../capi/include
DEPENDPATH += $$PWD/../../capi/include

INCLUDEPATH += $$PWD/../../capi/qt-wrapper
