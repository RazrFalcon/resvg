A simple example that shows how to use *svgr* through C API to render on a Cairo context.

## Run

```bash
cargo build --manifest-path ../../Cargo.toml
make
LD_LIBRARY_PATH=../../../../target/debug ./example image.svg image.png
```
