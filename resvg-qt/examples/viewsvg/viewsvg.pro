QT += core gui widgets

TARGET = viewsvg
TEMPLATE = app
CONFIG += C++11 debug_and_release

SOURCES += \
    main.cpp \
    mainwindow.cpp \
    svgview.cpp

HEADERS += \
    mainwindow.h \
    svgview.h

FORMS += \
    mainwindow.ui

LIBS += -lresvg_qt

BASEDIR = $$absolute_path(../../target)

QMAKE_LFLAGS_RELEASE += -L$$absolute_path(release,$$BASEDIR)
QMAKE_LFLAGS_DEBUG += -L$$absolute_path(debug,$$BASEDIR)

CONFIG(release, debug|release) {
    QMAKE_RPATHDIR += $$absolute_path(release,$$BASEDIR)
}
else:CONFIG(debug, debug|release) {
    QMAKE_RPATHDIR += $$absolute_path(debug,$$BASEDIR)
}

INCLUDEPATH += $$absolute_path(../../c-api)
DEPENDPATH += $$absolute_path(../../c-api)
