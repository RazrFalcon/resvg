## resvg
[![Build Status](https://travis-ci.org/RazrFalcon/resvg.svg?branch=master)](https://travis-ci.org/RazrFalcon/resvg)
[![Crates.io](https://img.shields.io/crates/v/resvg.svg)](https://crates.io/crates/resvg)
[![Documentation](https://docs.rs/resvg/badge.svg)](https://docs.rs/resvg)

*resvg* is an [SVG](https://en.wikipedia.org/wiki/Scalable_Vector_Graphics) rendering library.

## Purpose

*resvg* can be used as:

- a Rust library
- a C library (see [capi](./capi))
- a CLI application (see [tools/rendersvg](./tools/rendersvg))

to render SVG files based on a
[static](http://www.w3.org/TR/SVG11/feature#SVG-static)
[SVG Full 1.1](https://www.w3.org/TR/SVG11/) subset to raster images or
to a backend's canvas (e.g. to a QWidget via QPainter).

The core idea is to make a fast, small, portable, multiple-backend SVG library
designed for edge-cases.

Another major difference from other SVG rendering libraries is that *resvg* does a lot
of preprocessing before rendering. It converts an input SVG into a simplified one
called [Micro SVG](./docs/usvg_spec.adoc) and only then it begins rendering.
So it's very easy to implement a new rendering backend.
And you can also access *Micro SVG* as XML directly via [usvg](./tools/usvg) tool.

## SVG support

*resvg* is aiming to support only the [static](http://www.w3.org/TR/SVG11/feature#SVG-static)
SVG subset; e.g. no `a`, `script`, `view` or `cursor` elements, no events and no animations.

[SVG Tiny 1.2](https://www.w3.org/TR/SVGTiny12/) and [SVG 2.0](https://www.w3.org/TR/SVG2/)
are not supported and not planned.

Results of the [resvg test suite](./svg-tests/README.md):

![](./.github/chart.svg)

You can find a complete table of supported features
[here](https://razrfalcon.github.io/resvg-test-suite/svg-support-table.html).
It also includes alternative libraries.

## Rendering backends

At the moment, there are no production-ready 2D rendering libraries for Rust,
therefore we have to delegate the rendering itself to external libraries.

Right now, resvg supports four backends: [cairo], [Qt], [Skia] and [raqote].

All of them support exactly the same set of features and should produce roughly the same images
(see the [Testing](#testing) section for details).
While raqote is the most experimental and unstable of them, but the only one written in Rust.

From the performance perspective, Skia is the fastest one.
cairo and Qt performance heavily depends on SVG content, but roughly the same
(cairo has a faster shapes rendering, while Qt has a faster layers compositing).
And raqote is the slowest one.

## Performance

Comparing performance between different SVG rendering libraries is like comparing
apples and oranges. Everyone has a very different set of supported features,
implementation languages, build flags, etc. You should do the benchmarks by yourself,
on your images.

## Project structure

- `resvg` – rendering backends implementation
  - [`usvg`](./usvg) – an SVG simplification tool
    - [`roxmltree`](https://github.com/RazrFalcon/roxmltree) – a DOM-like XML tree
      - [`xmlparser`](https://github.com/RazrFalcon/xmlparser) – an XML parser
    - [`svgtypes`](https://github.com/RazrFalcon/svgtypes) – SVG types parser and writer
    - [`simplecss`](https://github.com/RazrFalcon/simplecss) – a simple CSS2 parser
    - [`ttf-parser`](https://github.com/RazrFalcon/ttf-parser) – a TrueType/OpenType parser
    - [`xmlwriter`](https://github.com/RazrFalcon/xmlwriter) – a simple XML writer
    - [`rctree`](https://github.com/RazrFalcon/rctree) – a DOM-like tree
  - [`resvg-qt`](./bindings/resvg-qt) – minimal bindings to [Qt]
  - [`resvg-skia`](./bindings/resvg-skia) – minimal bindings to [Skia]
  - [`svgfilters`](./svgfilters) – a collection of SVG filters

All other dependencies aren't written by me for this project.

## Directory structure

- `benches` - basic benchmarks for rendering operations
- `bindings` – minimal bindings to Qt and Skia used by *resvg*
- `capi` – C interface for *resvg*
- `docs` – basic documentation
- `examples` – usage examples for *resvg* as a library
- `src` – source code
- `svg-tests` - a collection of SVG files for testing
- `svgfilters` - SVG filters implementation
- `testing-tools` – scripts used for testing
- `tools` – useful tools
- `usvg` – an SVG simplification library used by *resvg*

## Safety

- The library must not panic. Any panic should be considered a critical bug and should be reported.
  There are only a few methods that can produce a panic.
- The core library structure (see above) does not use any `unsafe`,
  but since all backends are implemented via FFI, we are stuck with `unsafe` anyway.
  Also, `usvg` uses unsafe for fonts memory mapping.

## Testing

We are using regression testing to test *resvg*.

Basically, we will download a previous
*resvg* version and check that the new one produces the same results
(excluding the expected changes).

The downside of this method is that you need a network connection.
On the other hand, we have 4 backends and each of them will produce slightly different results
since there is no single correct 2D rendering technique. Bézier curves flattening,
gradients rendering, bitmaps scaling, anti-aliasing - they are all backend-specific.<br/>
Not to mention the text rendering. We don't use the system fonts rendering, but a list of available,
default fonts will still affect the results.

So a regression testing looks like the best choice between manual testing
and overcomplicated automatic one. And before each release I'm testing all files manually anyway.

See [testing-tools/regression/README.md](./testing-tools/regression/README.md) for more details.

Also, the test files itself are located at the `svg-tests` directory.

## License

*resvg* is licensed under the [MPLv2.0](https://www.mozilla.org/en-US/MPL/).


[Inkscape]: https://www.inkscape.org
[librsvg]: https://wiki.gnome.org/action/show/Projects/LibRsvg
[QtSvg]: https://doc.qt.io/qt-5/qtsvg-index.html

[cairo]: https://www.cairographics.org/
[Qt]: https://www.qt.io/
[Skia]: https://skia.org/
[raqote]: https://github.com/jrmuizel/raqote

[GNOME]: https://www.gnome.org/
