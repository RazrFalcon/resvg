QT += winextras

TARGET = SVGThumbnailExtension
TEMPLATE = lib

SOURCES += \
    src/thumbnailprovider.cpp \
    src/main.cpp \
    src/classfactory.cpp

HEADERS +=\
    src/thumbnailprovider.h \
    src/classfactory.h \
    src/common.h

DEF_FILE += \
    src/thumbnailprovider.def

CONFIG(release, debug|release): LIBS += -L$$PWD/../../resvg-qt/target/release/ -lresvg_qt
else:CONFIG(debug, debug|release): LIBS += -L$$PWD/../../resvg-qt/target/debug/ -lresvg_qt

INCLUDEPATH += $$PWD/../../resvg-qt/c-api
DEPENDPATH += $$PWD/../../resvg-qt/c-api

LIBS += \
    shlwapi.lib \
    advapi32.lib \
    gdiplus.lib \
    Userenv.lib \
    Ws2_32.lib
