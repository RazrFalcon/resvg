A simple example that shows how to use *resvg* from GTK+ through C-API.

I'm not good with C and GTK+ so any suggestions are welcome.

## Run

```bash
# build C-API with a cairo backend first
cd ../../capi
cargo build --features="cairo-backend"
cd ../examples/cairo-capi
make
LD_LIBRARY_PATH=../../capi/target/debug ./example ../../demo/Ghostscript_Tiger.svg
```

See [doc/build.md](../../../doc/build.md) for details.
