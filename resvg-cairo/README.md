# cairo backend for resvg

`resvg` backend implementation using the [cairo] library.

## Build

This backend uses the [gtk-rs](https://gtk-rs.org/) project.

Building on Linux should work out of the box, but others OS'es are more complex.

Note: we are not using the `gdk-pixbuf` crate for raster images, because it's way too heavy.

### on Windows using MSYS2

Install `stable-x86_64-pc-windows-gnu` [Rust] target.

Build using the MSYS2 shell:

```sh
pacman -S mingw-w64-x86_64-cairo
rustup default stable-x86_64-pc-windows-gnu

cargo build --release
```

You can use i686 target in the same way.

### on Linux

Install `cairo` and `harfbuzz` using your distributive's package manager.

On Ubuntu you can install them via:

```
sudo apt install libcairo2-dev libharfbuzz-dev
```

Build `resvg`:

```sh
cargo build --release
```

### on macOS

Using [homebrew](https://brew.sh):

```sh
brew install cairo

cargo build --release
```

## Runtime dependencies

`resvg-cairo` depends only on cairo. `pango` is not required.

On Linux, `harfbuzz` is also required.

## Running resvg CLI

```sh
cargo run --release -- in.svg out.png
```

The resulting binary can be found at: `target/release/resvg-cairo`

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


[cairo]: https://www.cairographics.org/
[Rust]: https://www.rust-lang.org/tools/install
