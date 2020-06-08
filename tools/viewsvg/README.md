# viewsvg

A simple SVG viewer using Qt and *resvg* C-API.

## Dependencies

- Qt >= 5.6

## Build

To build a release version:
```bash
# build C-API first
cargo build --release --features "qt-backend" --manifest-path ../../capi/Cargo.toml
# build viewsvg
qmake
make
# run
./viewsvg
```

Or, to build in debug mode:
```bash
cargo build --debug --features "qt-backend" --manifest-path ../../capi/Cargo.toml
qmake
make debug
./viewsvg
```

See [BUILD.adoc](../../BUILD.adoc) for details.
