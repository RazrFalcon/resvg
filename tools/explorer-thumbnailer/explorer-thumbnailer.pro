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

CONFIG(release, debug|release): LIBS += -L$$PWD/../../target/release/ -lresvg
else:CONFIG(debug, debug|release): LIBS += -L$$PWD/../../target/debug/ -lresvg

INCLUDEPATH += $$PWD/../../capi/include
DEPENDPATH += $$PWD/../../capi/include

LIBS += \
    shlwapi.lib \
    advapi32.lib \
    gdiplus.lib
