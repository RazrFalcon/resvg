#!/usr/bin/env bash

# WARNING: run only in a freshly cloned repo

set -e

VERSION="0.10.0"

if [ "$1" = "" ]; then
    echo "Usage: $0 backend"
    exit
fi

BACKEND="$1"
VERSIONED_DIR="resvg-$BACKEND-$VERSION"

# Temporary rename the backend dir.
mv "resvg-$BACKEND" "$VERSIONED_DIR"

cd "$VERSIONED_DIR"
mkdir -p .cargo
cargo vendor > .cargo/config
cd ..

env XZ_OPT="-9e" tar -cJvf "$VERSIONED_DIR".tar.xz "$VERSIONED_DIR"

# Rename back.
mv "$VERSIONED_DIR" "resvg-$BACKEND"

# Test our archive.
tar -xJf "$VERSIONED_DIR".tar.xz
cd "$VERSIONED_DIR"
cargo build --release --frozen --offline
cd ..
rm -r "$VERSIONED_DIR"
