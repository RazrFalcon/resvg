# raqote backend for resvg

`resvg` backend implementation using the [raqote] library.

**Warning**: the `raqote` library is still in development and pretty unstable.
You should prefer other backends.

This backend intentionally doesn't provide a C API.

## Build

Right now, this is the only backend that uses Rust-based 2D library,
therefore building process is fairly straightforward.

Sadly, you still need a C++ compiler to build [harfbuzz](https://github.com/harfbuzz/harfbuzz).

### on Windows using MSVC

Install `stable-x86_64-pc-windows-msvc` [Rust] target.

Build using `x64 Native Tools Command Prompt for VS 2017` shell:

```batch
set PATH=%userprofile%\.cargo\bin;%PATH%
rustup.exe default stable-x86_64-pc-windows-msvc

cargo.exe build --release
```

### on Windows using MSYS2

Install `stable-x86_64-pc-windows-gnu` [Rust] target.
And then:

```sh
pacman -S mingw-w64-x86_64-gcc
rustup default stable-x86_64-pc-windows-gnu

cargo build --release
```

You can use i686 target in the same way.

### on Linux

Install `harfbuzz` using your distributive's package manager.

On Ubuntu you can install it via:

```sh
sudo apt install libharfbuzz-dev
```

Build `resvg`:

```sh
cargo build --release
```

### on macOS

```sh
cargo build --release
```

## Runtime dependencies

`harfbuzz` on Linux. On other OS'es it will be built statically.

## Running resvg CLI

```sh
cargo run --release -- in.svg out.png
```

The resulting binary can be found at: `target/release/resvg-raqote`

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


[raqote]: https://github.com/jrmuizel/raqote
[Rust]: https://www.rust-lang.org/tools/install
