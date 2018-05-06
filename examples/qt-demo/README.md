A simple demo with a Qt backend.

Shows how to use the *resvg* C-API.

## Dependencies

- Qt >= 5.6

## Run

```bash
# build C-API first
cargo build --release --features "qt-backend" --manifest-path ../../capi/Cargo.toml
# build demo
qmake
make
LD_LIBRARY_PATH=../../target/release ./demo
```

See [docs/build.md](../../docs/build.md) for details.
