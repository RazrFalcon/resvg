# Skia backend for resvg

`resvg` backend implementation using the [Skia] library.

## Build

The Skia build process is not trivial, therefore we are not building it automatically.
A caller should built it manually and set corespondent environment variables:

- `SKIA_DIR` should point to a Skia directory that contains the Skia `include` directory.
- `SKIA_LIB_DIR` should point to a Skia directory that contains `skia.dll`.

Also, Skia doesn't have a stable API, therefore we can support only a fixed version.
Right now it is m76.

### on Windows using MSVC

Install:

- `stable-x86_64-pc-windows-msvc` [Rust] target.
- Skia itself (we assume that you have already built one).

`SKIA_DIR` should point to a Skia directory that contains the Skia `include` directory.
`SKIA_LIB_DIR` should point to a Skia directory that contains `skia.dll`.

Build using `x64 Native Tools Command Prompt for VS 2017` shell:

```batch
set PATH=%userprofile%\.cargo\bin;%PATH%
set SKIA_DIR=path
set SKIA_LIB_DIR=path
rustup.exe default stable-x86_64-pc-windows-msvc

cargo.exe build --release
```

### on Linux

We assume that you have already built Skia itself.

Install `harfbuzz` using your distributive's package manager.

On Ubuntu you can install it via:

```sh
sudo apt install libharfbuzz-dev
```

Build `resvg`:

```sh
SKIA_DIR=path SKIA_LIB_DIR=path cargo build --release
```

### on macOS

We assume that you have already built Skia itself.

```sh
SKIA_DIR=path SKIA_LIB_DIR=path cargo build --release
```

## Runtime dependencies

`resvg-skia` depends only on Skia itself.

On Linux, `harfbuzz` is also required.

## Running resvg CLI

```sh
cargo run --release -- in.svg out.png
```

The resulting binary can be found at: `target/release/resvg-skia`

## Running examples

A simple SVG to PNG converter:

```sh
cargo run --example minimal -- in.svg out.png
```

Render image using a manually constructed SVG render tree:

```sh
cargo run --example custom_rtree
```

Draw bounding boxes around all shapes on input SVG:

```sh
cargo run --example draw_bboxes -- bboxes.svg bboxes.png -z 4
```

## License

[MPLv2.0](https://www.mozilla.org/en-US/MPL/).


[Skia]: https://skia.org/
[Rust]: https://www.rust-lang.org/tools/install
