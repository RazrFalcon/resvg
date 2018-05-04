QT       += testlib

TARGET    = tst_resvgqt
CONFIG   += console
CONFIG   -= app_bundle

TEMPLATE = app

QMAKE_CXXFLAGS += -Wextra -Wpedantic

QMAKE_CXXFLAGS += -fsanitize=address
QMAKE_LFLAGS += -fsanitize=address

SOURCES += \
    tst_resvgqt.cpp \
    $$PWD/../qt-wrapper/ResvgQt.cpp

DEFINES += SRCDIR=\\\"$$PWD\\\"

CONFIG(release, debug|release): LIBS += -L$$PWD/../../target/release/ -lresvg
else:CONFIG(debug, debug|release): LIBS += -L$$PWD/../../target/debug/ -lresvg

INCLUDEPATH += $$PWD/../include
DEPENDPATH += $$PWD/../include

INCLUDEPATH += $$PWD/../qt-wrapper
