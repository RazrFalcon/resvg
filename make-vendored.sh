#!/usr/bin/env bash

set -e

VERSION="0.9.0"

git clone https://github.com/RazrFalcon/resvg resvg-$VERSION
cd resvg-"$VERSION"
git checkout tags/v"$VERSION" -b temp-branch

mkdir -p .cargo
cargo-vendor vendor --relative-path > .cargo/config

cd ..

env XZ_OPT="-9e" tar \
    --exclude=".git" \
    --exclude=".github" \
    --exclude=".gitignore" \
    --exclude=".travis.yml" \
    --exclude="version-bump.md" \
    --exclude="docs" \
    --exclude="benches" \
    --exclude="examples" \
    --exclude="testing-tools" \
    --exclude="capi/qtests" \
    -cJf resvg-"$VERSION".tar.xz resvg-"$VERSION"

# Clean up.
rm -rf resvg-"$VERSION"

# Test our archive.
tar -xJf resvg-"$VERSION".tar.xz
cd resvg-"$VERSION"
cargo build --verbose --release --frozen \
    --manifest-path tools/rendersvg/Cargo.toml --features "raqote-backend"

# Clean up again.
cd ..
rm -r resvg-"$VERSION"
