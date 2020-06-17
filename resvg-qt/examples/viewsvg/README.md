# viewsvg

A simple SVG viewer using resvg-qt.

## Dependencies

- Qt >= 5.6

## Build

Note: make sure you have read the parent readme.

```sh
# build C-API first
cargo build --release --manifest-path ../../c-api/Cargo.toml
# build viewsvg
qmake
make
# run
./viewsvg
```
