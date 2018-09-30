# How to build resvg

## General

Currently, *resvg* supports only two backend: Qt and cairo.
You can build them separately or together.

To enable a backend use the `--features` option of the `cargo`:

```bash
# Build with a Qt backend
cargo build --release --features="qt-backend"
# or with a cairo backend
cargo build --release --features="cairo-backend"
# or with both.
cargo build --release --features="qt-backend cairo-backend"
```

### Rust

The library requires Rust >= 1.22.

### Qt backend

Qt backend requires only `QtCore` and `QtGui` libraries.
And the JPEG image format plugin (eg. `plugins/imageformats/libqjpeg.(dll/so/dylib)`).

Technically, any Qt 5 version should work, but we only support Qt >= 5.6.

### cairo backend

We are using `pango` for text rendering, so you have to build it too.
With a `pangocairo` library (part of the `pango`).

## Windows

1. [Install Rust](https://www.rust-lang.org/en-US/install.html) with a
`stable-i686-pc-windows-gnu` target. MSVS is not supported.
1. Install [MSYS2](http://www.msys2.org/).

### Qt backend

Only MinGW 32bit version is supported.

Install Qt MinGW 32bit using an
[official installer](http://download.qt.io/official_releases/online_installers/qt-unified-windows-x86-online.exe).

In the MSYS2 Shell:
```bash
# We are using Qt 5.9.3 for example.

# Prepare PATH.
export PATH="/c/Qt/5.9.3/mingw53_32/bin:/c/Qt/Tools/mingw530_32/bin:/c/Users/$USER/.cargo/bin:$PATH"

# Build.
QT_DIR=/c/Qt/5.9.3/mingw53_32 cargo.exe build --release --features "qt-backend"
```

### cairo backend

Install GTK+ dependencies using MSYS2 as explained
[here](http://gtk-rs.org/docs/requirements.html#windows).

We do not need the whole GTK+, so we can install only `pango` (which will install
`cairo` too) and `gdk-pixbuf2`:

```bash
pacman -S mingw-w64-i686-pango mingw-w64-i686-gdk-pixbuf2
```

Then we can build *resvg*:

```bash
cargo.exe build --release --features "cairo-backend"
```

## Linux

[Install Rust](https://www.rust-lang.org/en-US/install.html).

### Qt backend

Install Qt 5 using your distributive package manager.

```bash
cargo build --release --features "qt-backend"
```

If you don't want to use a system Qt you can alter it with the `PKG_CONFIG_PATH` variable.

```bash
PKG_CONFIG_PATH='/path_to_qt/lib/pkgconfig' cargo build --release --features "qt-backend"
```

### cairo backend

Install `cairo`, `pango`(with `pangocairo`) and `gdk-pixbuf` using your distributive's package manager.

For Ubuntu its `libpango1.0-dev` and `libgdk-pixbuf2.0-dev`.

```bash
cargo build --release --features "cairo-backend"
```

## macOS

[Install Rust](https://www.rust-lang.org/en-US/install.html).

### Qt backend

Using [homebrew](https://brew.sh/):

```bash
brew install qt

QT_DIR=/usr/local/opt/qt cargo build --release --features "qt-backend"
```

Or an
[official installer](http://download.qt.io/official_releases/online_installers/qt-unified-mac-x64-online.dmg):

```bash
QT_DIR=/Users/$USER/Qt/5.9.3/clang_64 cargo build --release --features "qt-backend"
```

### cairo backend

Using [homebrew](https://brew.sh/):

```bash
brew install gdk-pixbuf pango cairo

cargo build --release --features "cairo-backend"
```
