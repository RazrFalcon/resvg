A simple example that shows how to use *resvg* from GTK+ through C-API.

I'm not good with C and GTK+ so any suggestions are welcome.

## Run

```sh
# build C-API first
cargo build --release --manifest-path ../../c-api/Cargo.toml
make
LD_LIBRARY_PATH=../../target/release ./example image.svg
```
