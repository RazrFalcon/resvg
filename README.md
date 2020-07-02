## resvg
[![Build Status](https://travis-ci.org/RazrFalcon/resvg.svg?branch=master)](https://travis-ci.org/RazrFalcon/resvg)
[![Crates.io](https://img.shields.io/crates/v/resvg.svg)](https://crates.io/crates/resvg)
[![Documentation](https://docs.rs/resvg/badge.svg)](https://docs.rs/resvg)

*resvg* is an [SVG](https://en.wikipedia.org/wiki/Scalable_Vector_Graphics) rendering library.

## Purpose

`resvg` can be used as a Rust library, a C library and as a CLI application
to render SVG files based on a
[static](http://www.w3.org/TR/SVG11/feature#SVG-static)
[SVG Full 1.1](https://www.w3.org/TR/SVG11/) subset.

The core idea is to make a fast, small, portable SVG library designed for edge-cases.
Right now, a `resvg` CLI application is less than 4MiB and doesn't require any external dependencies.

At the moment, there are no production-ready 2D rendering libraries for Rust.
Because of that, `resvg` relies on [Skia].

Another major difference from other SVG rendering libraries is that `resvg` does a lot
of preprocessing before rendering. It converts an input SVG into a simplified one
called [Micro SVG](./docs/usvg_spec.adoc) and only then it begins rendering.
So it's very easy to implement a new rendering backend.
But we officially support only the Skia one.
And you can also access *Micro SVG* as XML directly via the [usvg](./usvg) tool.

## SVG support

`resvg` is aiming to support only the [static](http://www.w3.org/TR/SVG11/feature#SVG-static)
SVG subset; e.g. no `a`, `script`, `view` or `cursor` elements, no events and no animations.

[SVG Tiny 1.2](https://www.w3.org/TR/SVGTiny12/) and [SVG 2.0](https://www.w3.org/TR/SVG2/)
are not supported and not planned.

Results of the [resvg test suite](./tests/README.md):

![](./.github/chart.svg)

You can find a complete table of supported features
[here](https://razrfalcon.github.io/resvg-test-suite/svg-support-table.html).
It also includes alternative libraries.

## Performance

Comparing performance between different SVG rendering libraries is like comparing
apples and oranges. Everyone has a very different set of supported features,
implementation languages, build flags, etc.
But since `resvg` is written in Rust and uses [Skia] for rendering - it's pretty fast.

## Building

Despite being a Rust library, `resvg` depends on [Skia] and [harfbuzz],
therefore you will need a modern C++ compiler. But in most cases the compilation
process should be as easy as:

```
cargo build --release
```

which will produce binaries that doesn't require any external dependencies.

And while we can leave `harfbuzz` compilation to Cargo, Skia is more troublesome.
Mainly because it
[requires](https://skia.org/user/build#compilers)
`clang` and no other compilers.

By default, `resvg` uses it's own Skia bindings called
[tiny-skia](https://github.com/RazrFalcon/tiny-skia). Which supports only `clang` too.
See the [Build with embedded Skia](https://github.com/RazrFalcon/tiny-skia#build-with-embedded-skia)
section for details. And yes, you can use your own Skia build too.

Also, we do not support 32-bit builds and MINGW target.

### Linux specific

By default, the system `harfbuzz` library will be linked on Linux and BSD.
To force static linking, set the `HARFBUZZ_SYS_NO_PKG_CONFIG=1` environment variable.

## Safety

Since `resvg` depends on C++ libraries, it's inherently unsafe in Rust terms.
Despite of that, most of the dependencies are actually fully safe.
The main exceptions are [Skia] and [harfbuzz] bindings, and files memory mapping.

## License

`resvg` project is licensed under the [MPLv2.0](https://www.mozilla.org/en-US/MPL/).


[Skia]: https://skia.org/
[harfbuzz]: https://github.com/harfbuzz/harfbuzz
