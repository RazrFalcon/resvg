C interface for *resvg*.

## Build

```bash
# Build with a Qt backend
cargo build --release --features="qt-backend"
# or with a cairo backend
cargo build --release --features="cairo-backend"
# or with both.
cargo build --release --features="qt-backend cairo-backend"
```

See [doc/build.md](../doc/build.md) for details.

This will build a dynamic library. There is no point in building a static
library since it still will depend on Qt/cairo.
