# Qt backend for resvg

`resvg` backend implementation using the [Qt] library.

## Build

*resvg* is a [Rust] library, therefore you should install it first.

Since there are no Qt binding for Rust yet, we are using our own.
This complicates the build process a bit.

On Windows and macOS, the build process consists of setting the `QT_DIR` environment variable and
running `cargo build`. On Linux we are using `pkg-config` instead.

### on Windows using MSVC

Install:

- `stable-x86_64-pc-windows-msvc` [Rust] target.
- Qt built with MSVC.

Build using `x64 Native Tools Command Prompt for VS 2017` shell:

```batch
set PATH=%userprofile%\.cargo\bin;%PATH%
set QT_DIR=C:\Qt\5.12.0\msvc2017_64
rustup.exe default stable-x86_64-pc-windows-msvc

cargo.exe build --release
```

Instead of `msvc2017_64` you can use any other Qt MSVC build. Even 32-bit one.
We are using Qt 5.12.0 just for example.

### on Windows using MinGW

Install:

- `stable-x86_64-pc-windows-gnu` [Rust] target.
- Qt built with MinGW 64-bit + mingw bundled with Qt.

Build using `cmd.exe`:

```batch
set PATH=C:\Qt\5.12.0\mingw73_64\bin;C:\Qt\Tools\mingw730_64\bin;%userprofile%\.cargo\bin;%PATH%
set QT_DIR=C:\Qt\5.12.0\mingw73_64
rustup.exe default stable-x86_64-pc-windows-gnu

cargo.exe build --release
```

Instead of `mingw73_64` you can use any other Qt mingw build.
We are using Qt 5.12.0 just for example.

### on Linux

Install Qt and `harfbuzz` using your distributive's package manager.

On Ubuntu you can install them via:

```
sudo apt install qtbase5-dev libharfbuzz-dev
```

Build `resvg`:

```sh
cargo build --release
```

If you don't want to use the system Qt, you can alter it with the `PKG_CONFIG_PATH` variable.

```sh
PKG_CONFIG_PATH='/path_to_qt/lib/pkgconfig' cargo build --release
```

### on macOS

Using an [official Qt installer](http://download.qt.io/official_releases/online_installers/qt-unified-mac-x64-online.dmg):

```sh
QT_DIR=/Users/$USER/Qt/5.12.0/clang_64 cargo build --release
```

or [homebrew](https://brew.sh):

```sh
brew install qt

QT_DIR=/usr/local/opt/qt cargo build --release
```

We are using Qt 5.12.0 just for example.

## Runtime dependencies

`resvg-qt` depends only on QtCore and QtGui libraries and imageformats/qjpeg plugin.

Technically, any Qt 5 version should work, but we only support Qt >= 5.6.

## Running examples

Note: we assume that you have already set up the `QT_DIR` variable or pkg-config,
as was described above.

A simple SVG to PNG converter:

```sh
cargo run --example minimal -- in.svg out.png
```

Render image using a manually constructed SVG render tree:

```sh
cargo run --example custom_rtree
```

Draw bounding boxes around all shapes on input SVG:

```sh
cargo run --example draw_bboxes -- bboxes.svg bboxes.png -z 4
```

## License

[MPLv2.0](https://www.mozilla.org/en-US/MPL/).


[Qt]: https://www.qt.io/
[Rust]: https://www.rust-lang.org/tools/install
