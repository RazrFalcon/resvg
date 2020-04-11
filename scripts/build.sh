#!/bin/sh

export source_root="$1"
export output="$2"
export meson_build_root="$3"
export release_profile="$4"
export featureset="$5"

export CARGO_TARGET_DIR="$meson_build_root"/cargo-target
export CARGO_HOME="$meson_build_root"/cargo-home

if [ "$featureset" = "both" ]; then
    features="qt-backend cairo-backend"
elif [ "$featureset" = "qt" ]; then
    features="qt-backend"
else
    features="cairo-backend"
fi

if [ "$release_profile" = "debug" ]; then
    cargo build --manifest-path "$1"/Cargo.toml --features "$features"
    cargo build --manifest-path "$1"/capi/Cargo.toml --features "$features"
    cp "$CARGO_TARGET_DIR"/debug/libresvg.so "$2"
else
    cargo build --manifest-path "$1"/Cargo.toml --release --features "$features"
    cargo build --manifest-path "$1"/capi/Cargo.toml --release --features "$features"
    cp "$CARGO_TARGET_DIR"/debug/libresvg.so "$2"
fi