#!/usr/bin/env bash

set -e

VERSION="0.14.0"

git clone https://github.com/RazrFalcon/resvg resvg-$VERSION
cd resvg-"$VERSION"
git checkout tags/v"$VERSION" -b temp-branch

mkdir -p .cargo
cargo vendor > .cargo/config

cd ..

env XZ_OPT="-9e" tar \
    --exclude=".git" \
    --exclude=".gitignore" \
    --exclude=".travis.yml" \
    --exclude="resvg-$VERSION/.github" \
    --exclude="resvg-$VERSION/version-bump.md" \
    --exclude="resvg-$VERSION/docs" \
    -cJf resvg-"$VERSION".tar.xz resvg-"$VERSION"

# Clean up.
rm -rf resvg-"$VERSION"

# Test our archive.
tar -xJf resvg-"$VERSION".tar.xz
cd resvg-"$VERSION"
cargo build --release --frozen

# Clean up again.
cd ..
rm -r resvg-"$VERSION"
