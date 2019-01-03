# viewsvg

A simple SVG viewer using Qt and *resvg* C-API.

## Dependencies

- Qt >= 5.6

## Run

```bash
# build C-API first
cargo build --release --features "qt-backend" --manifest-path ../../capi/Cargo.toml
# build viewsvg
qmake
make
# run
LD_LIBRARY_PATH=../../target/release ./viewsvg
```

See [BUILD.adoc](../../BUILD.adoc) for details.
