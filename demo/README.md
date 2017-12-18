A simple demo with a Qt backend.

Shows how to use the *libresvg* C-API.

## Dependencies

- Qt >= 5.6

## Run

```bash
# build C-API first
cd ../capi
cargo build --release --features "qt-backend"
# build demo
cd ../demo
qmake
make
LD_LIBRARY_PATH=../capi/target/release ./demo
```

See [doc/build.md](../doc/build.md) for details.