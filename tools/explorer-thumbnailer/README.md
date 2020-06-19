# explorer-thumbnailer

An SVG thumbnails generator for the Windows Explorer.

## Dependencies

- Windows 64bit >= Vista
- MSVC >= 2015
- Qt >= 5.6 built with MSVC

## Build

```batch
"C:\Program Files (x86)\Microsoft Visual Studio 14.0\VC\vcvarsall.bat" amd64
set PATH=%userprofile%\.cargo\bin;C:\Qt\5.12.0\msvc2015_64\bin;%PATH%
rem build C-API first
set QT_DIR=C:\Qt\5.12.0\msvc2015_64
cargo build --release --manifest-path ../../resvg-qt/c-api/Cargo.toml
rem build thumbnailer
qmake
nmake
rem prepare files for installer
windeployqt --no-translations release\SVGThumbnailExtension.dll
```

## Origin

This project is based on
[SVG Viewer Extension for Windows Explorer](https://github.com/maphew/svg-explorer-extension).
Unlike the original, it uses *resvg* with Qt5 instead of Qt4 SVG module.

Also, it contains some code refactoring and a new installer.

## Licencse

This project is licensed under the LGPL-3.0, just as an original project.
