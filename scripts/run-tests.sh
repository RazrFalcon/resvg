#!/bin/bash

# exit on error, verbose
set -ev

WORK_DIR="."
PKG_DIR="$TRAVIS_BUILD_DIR"

# if a local run
if [ -z "$TRAVIS_BUILD_DIR" ]; then
    PKG_DIR=$(pwd)"/.."
    LOCAL_TEST=true
    WORK_DIR="/tmp/"
fi

# test qt backend
cd "$PKG_DIR"/tools/rendersvg
cargo build --verbose --features="qt-backend"
cd "$PKG_DIR"/tests/regression
if [ -z "$LOCAL_TEST" ]; then
    export QT_QPA_PLATFORM=offscreen
    sudo ln -s /usr/share/fonts /opt/qt56/lib/fonts
fi
mkdir -p "$WORK_DIR"/workdir-qt
cargo run --release -- --workdir="$WORK_DIR"/workdir-qt --backend=qt --use-prev-commit

# test cairo backend
cd "$PKG_DIR"/tools/rendersvg
cargo build --verbose --features="cairo-backend"
cd "$PKG_DIR"/tests/regression
mkdir -p "$WORK_DIR"/workdir-cairo
cargo run --release -- --workdir="$WORK_DIR"/workdir-cairo --backend=cairo --use-prev-commit

# try to build with all backends
cd "$PKG_DIR"/tools/rendersvg
cargo build --verbose --features="cairo-backend qt-backend"

# build demo
#
# build C-API for demo
cd "$PKG_DIR"/capi
cargo build --verbose --features="qt-backend"
#
cd "$PKG_DIR"/demo
QT_SELECT=5 qmake CONFIG+=debug
make

# build cairo C example
#
# build C-API for cairo-capi
cd "$PKG_DIR"/capi
cargo build --verbose --features="cairo-backend"
#
cd "$PKG_DIR"/examples/cairo-capi
make

# build cairo-rs example
cd "$PKG_DIR"/examples/cairo-rs
cargo build
