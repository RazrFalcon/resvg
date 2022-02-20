# C API for resvg

## Build

```sh
cargo build --release
```

This will produce dynamic and static C libraries that can be found at `../target/release`.

## Header generation

The `resvg.h` is generated via [cbindgen](https://github.com/eqrion/cbindgen)
and then manually edited a bit.
