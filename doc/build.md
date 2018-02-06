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

Also, you can build *resvg* without any backends if you what too.

### Qt backend

Qt backend requires only `QtCore` and `QtGui` libraries.
And the JPEG image format plugin (eg. `plugins/imageformats/libqjpeg.so`).

Technically, any Qt 5 version should work, but we support only Qt >= 5.6.

### cairo backend

We use `pango` for text rendering, so you have to install/build it too
with a `pangocairo` library.

## Windows

1. [Install Rust](https://www.rust-lang.org/en-US/install.html) with a
`stable-i686-pc-windows-gnu` target.
1. Install [MSYS2](http://www.msys2.org/).

### Qt backend

Only MinGW 32bit version is supported. MSVS should work too, but it is not tested.

Install Qt MinGW 32bit using an
[official installer](http://download.qt.io/official_releases/online_installers/qt-unified-windows-x86-online.exe).

In the MSYS2 Shell:
```bash
# We use Qt 5.9.3 for example.

# Prepare PATH.
export PATH="/c/Qt/5.9.3/mingw53_32/bin:/c/Qt/Tools/mingw530_32/bin:/c/Users/$USER/.cargo/bin:$PATH"

# Build.
QT_DIR=/c/Qt/5.9.3/mingw53_32 cargo.exe build --release --features "qt-backend"
```

### cairo backend

Install GTK+ dependencies using MSYS2 as explained
[here](http://gtk-rs.org/docs/requirements.html#windows).

We no need the whole GTK+, so we can install only `pango`, which will install
`cairo` too.

Then run this command in the MSYS2 MinGW Shell:
```
cargo.exe build --release --features "cairo-backend"
```

## Linux

### Qt backend

Install Qt 5 using your distributive package manager.

```
cargo build --release --features "qt-backend"
```

### cairo backend

Install `cairo` and `pango`(with `pangocairo`) using your distributive package manager.

For Ubuntu you need only `libpango1.0-dev`.

```
cargo build --release --features "cairo-backend"
```

## macOS

[Install Rust](https://www.rust-lang.org/en-US/install.html).

### Qt backend

Install Qt using an
[official installer](http://download.qt.io/official_releases/online_installers/qt-unified-mac-x64-online.dmg).

```
QT_DIR=/Users/$USER/Qt/5.9.3/clang_64 cargo build --release --features "qt-backend"
```

### cairo backend

Not supported.
