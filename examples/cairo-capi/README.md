A simple example that shows how to use *resvg* from GTK+ through C-API.

I'm not good with C and GTK+ so any suggestions are welcome.

## Run

```bash
# build C-API with a cairo backend first
cargo build --release --features "cairo-backend" --manifest-path ../../capi/Cargo.toml
make
LD_LIBRARY_PATH=../../target/debug ./example ../qt-demo/hello-resvg.svg
```

See [docs/build.md](../../docs/build.md) for details.
