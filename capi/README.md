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

See [BUILD.adoc](../BUILD.adoc) for details.

This will build a dynamic library. There is no point in building the static
library since it will depend on Qt/cairo anyway.

## Examples

A usage example with a *cairo* backend can be found at [examples/cairo-capi](../examples/cairo-capi).

A usage example with a *qt* backend can be found in the [examples/qt-demo](../examples/qt-demo) app.
