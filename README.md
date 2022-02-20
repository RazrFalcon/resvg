## resvg
![Build Status](https://github.com/RazrFalcon/resvg/workflows/Rust/badge.svg)
[![Crates.io](https://img.shields.io/crates/v/resvg.svg)](https://crates.io/crates/resvg)
[![Documentation](https://docs.rs/resvg/badge.svg)](https://docs.rs/resvg)
[![Rust 1.51+](https://img.shields.io/badge/rust-1.51+-orange.svg)](https://www.rust-lang.org)

*resvg* is an [SVG](https://en.wikipedia.org/wiki/Scalable_Vector_Graphics) rendering library.

It can be used as a Rust library, as a C library and as a CLI application to render static SVG files.

The core idea is to make a fast, small, portable SVG library with an aim to support the whole SVG spec.

## Features

### Designed for edge-cases

SVG is a very complicated format with a large specification (SVG 1.1 is almost 900 pages).
You basically need a web browser to handle all of it. But the truth is that even browsers
fail at this (see [SVG support](https://github.com/RazrFalcon/resvg#svg-support)).
Yes, unlike `resvg`, browser do support dynamic SVG features like animations and scripting.
But using a browser to render SVG _correctly_ is sadly not an option.

To prove its correctness, `resvg` has a vast test suite which includes almost 1500 tests.
And this is only SVG-to-PNG regression tests. This doesn't include tests in `resvg` dependencies.
And the best thing is that `resvg` test suite is available to everyone. It's not tied to `resvg`
in any way. Which should help people who plan to develop their own SVG libraries.

### Safety

It's hard not to mention safety when we talk about Rust and processing of a random input.
And we're talking not only about SVG/XML here, but also about CSS, TTF, PNG, JPEG, GIF and GZIP.

While `resvg` is not the only SVG library written in Rust, it's the only one that
is written completely in Rust. There are no non-Rust code in the final binary.

Moreover, there are almost no unsafe code either. Yes, some dependencies still have some unsafe
and fonts memory-mapping is inherently unsafe.
But it's still the best you can get in terms of memory safety.

And not memory safety alone. `resvg` has extensive checks to prevent endless loops (freezes)
and stack overflows (via recursion).

### Zero bloat

Right now, a `resvg` CLI application is less than 3MB and doesn't require any external dependencies.
There is nothing in the binary that is not needed for rendering SVG files.

### Portable

Right now, `resvg` is guarantee to work everywhere were you can compile the Rust itself,
including WASM. There are some rough edges with obscure CPU architectures and
mobile OSes (mainly system fonts loading). But it should be pretty painless otherwise.

### SVG preprocessing

Another major difference from other SVG rendering libraries is that in `resvg`
SVG parsing and rendering are two completely separate steps.
Moreover, they are two separate libraries altogether: `resvg` and [usvg].
Meaning you can easily write your own renderer on top of `usvg` using any 2D library of your liking.
Or you can work with a preprocessed and simplified SVG data called [Micro SVG](./docs/usvg_spec.adoc)
in which case you could avoid dealing with most of SVG complexity.

### Performance

Comparing performance between different SVG rendering libraries is like comparing
apples and oranges. Everyone has a very different set of supported features,
implementation languages, build flags, etc.
But since `resvg` is written in Rust and uses [tiny-skia] for rendering - it's pretty fast.
Moreover, it still has a lot of room for improvement.

### Reproducibility

Since `resvg` doesn't rely on any system libraries it allows us to have reproducible results
on all supported platforms. Meaning if you render an SVG file on x86 Windows and then render it
on ARM macOS - the produced image will be identical. Each pixel would have exactly the same value.

## Limitations

- No animations<br>
  There are no plans on implementing them.
- No native text rendering<br>
  Because resvg doesn't rely on any system libraries we cannot use native text rendering either.
  On the other hand, native text rendering is optimized for small horizontal text, which is not
  that common is SVG.
- Unicode-only<br>
  It's the 21th century. Text files not in UTF-8 encoding are no longer relevant.

## SVG support

`resvg` is aiming to support only the [static](http://www.w3.org/TR/SVG11/feature#SVG-static)
SVG subset; e.g. no `a`, `script`, `view` or `cursor` elements, no events and no animations.

[SVG 2](https://www.w3.org/TR/SVG2/) support is in progress.
You can check for issues with the
[svg2 tag](https://github.com/RazrFalcon/resvg/issues?q=is%3Aissue+is%3Aopen+label%3Asvg2)
or our [SVG 2 changelog](https://github.com/RazrFalcon/resvg/blob/master/docs/svg2-changelog.md).

[SVG Tiny 1.2](https://www.w3.org/TR/SVGTiny12/) is not supported and not planned.

Results of the [resvg test suite](./tests/README.md):

![](./.github/chart.svg)

SVG 2 only results:

![](./.github/chart-svg2.svg)

You can find a complete table of supported features
[here](https://razrfalcon.github.io/resvg-test-suite/svg-support-table.html).
It also includes alternative libraries.

We're not testing against all SVG libraries because many of them are pretty bad.
If your library is not on the list it probably doesn't pass even the 25% mark.
Such libraries are: wxSvg, LunaSVG and nanosvg.

## resvg project

There is a subtle difference between resvg as a _library_ and resvg as a _project_.
While most users will interact only with the resvg library, it's just a tip of an iceberg.
There are a lot of libraries that I had to write to make resvg possible.
Here are some of them:

- resvg - the actual SVG renderer
- [usvg] - an SVG preprocessor/simplifier
- [tiny-skia] - a [Skia](https://github.com/google/skia) subset ported to Rust
- [rustybuzz] - a [harfbuzz](https://github.com/harfbuzz/harfbuzz) subset ported to Rust
- [ttf-parser] - a TrueType/OpenType font parser
- [fontdb] - a simple, in-memory font database with CSS-like queries
- [roxmltree] + [xmlparser] - an XML parsing libraries
- [simplecss] - a pretty decent CSS 2 parser and selector
- [pico-args] - an absolutely minimal, but surprisingly popular command-line arguments parser

So while the resvg _library_ is deceptively small (around 2500 LOC), the resvg _project_
is nearing 75'000 LOC. Which is not that much considering how much resvg does.
It's definitely the smallest option out there.

## License

`resvg` project is licensed under the [MPLv2.0](https://www.mozilla.org/en-US/MPL/).

[usvg]: https://github.com/RazrFalcon/resvg/tree/master/usvg
[rustybuzz]: https://github.com/RazrFalcon/rustybuzz
[tiny-skia]: https://github.com/RazrFalcon/tiny-skia
[ttf-parser]: https://github.com/RazrFalcon/ttf-parser
[roxmltree]: https://github.com/RazrFalcon/roxmltree
[xmlparser]: https://github.com/RazrFalcon/xmlparser
[simplecss]: https://github.com/RazrFalcon/simplecss
[fontdb]: https://github.com/RazrFalcon/fontdb
[pico-args]: https://github.com/RazrFalcon/pico-args
