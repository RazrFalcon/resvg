## resvg [![Build Status]](https://travis-ci.org/RazrFalcon/resvg)

[Build Status]: https://travis-ci.org/RazrFalcon/resvg.svg?branch=master

*resvg* is an [SVG](https://en.wikipedia.org/wiki/Scalable_Vector_Graphics) rendering library.

## Purpose

*resvg* can be used to render SVG files based on a
[static](http://www.w3.org/TR/SVG11/feature#SVG-static)
[SVG Full 1.1](https://www.w3.org/TR/SVG/Overview.html) subset, excluding
[fonts support](https://www.w3.org/TR/SVG11/feature#Font).
In simple terms: no animations, scripting and embedded fonts.

The core idea is to make a fast, portable, small, multiple backend library
designed for edge-cases.

It can be used as a simple SVG to PNG converter
and as an embeddable library to paint SVG on an application native canvas.

## Why a new library?

*resvg* is trying to compete with [librsvg], [QtSvg]
and [Inkscape] (only as a CLI SVG to PNG converter).

One of the main difference from other rendering libraries is that *resvg* does a lot
of preprocessing before rendering. It converts shapes to paths, resolves attributes,
removes groups, removes invisible elements, fixes a lot of issues in malformed SVG files
and only then starts the rendering. So it's very easy to implement a new rendering backend.

More details [here](doc/svgdom.adoc).

### resvg vs librsvg

*librsvg* is the main competitor to the *resvg*. And even though that *librsvg* itself is being
rewritten in Rust, as *resvg*, the architecture of the library is completely different:

- *librsvg*, currently, is heavily tied to [cairo] library, unlike *resvg*
- *librsvg* is heavily tied to [GNOME] which makes it painful to distribute outside the Linux ecosystem
- *librsvg* doesn't really preprocess input files, rendering them as is
- *librsvg* has a minimal support of the edge-cases, which leads to rendering errors

### resvg vs Inkscape

Inkscape is often used to convert SVG to PNG, but it's not an actual competitor to *resvg*,
because it's still a complete SVG editor, not a tiny library.
Also, it's very slow.
But it has the best SVG support amongst other.

### resvg vs QtSvg

Without a doubt, [QtSvg] is heavily used in [Qt] applications.
But [QtSvg] itself is very limited. It officially supports only a tiny portion
of the SVG Tiny 1.2 subset. In simple terms - it correctly renders only primitive SVG images.

## SVG support

Test results using [testing_tools/images](testing_tools/images):

[![Chart]](https://htmlpreview.github.io/?https://github.com/RazrFalcon/resvg/blob/master/doc/support-chart.html)

[Chart]: doc/support-chart.png

You can see chart's underling data
[here](https://htmlpreview.github.io/?https://github.com/RazrFalcon/resvg/blob/master/doc/support-table-checked.html).

You can find a complete table of supported features [here](doc/support-table.adoc).
It also includes alternative libraries.

TL;DR

- no `filter`
- no `mask`
- no `marker`
- no `symbol`

## Backends

*resvg* supports [Qt] and [cairo] backends.

## Build

See [doc/build.md](doc/build.md) for details.

## License

*resvg* is licensed under the [MPLv2.0](https://www.mozilla.org/en-US/MPL/).


[Inkscape]: https://www.inkscape.org
[librsvg]: https://wiki.gnome.org/action/show/Projects/LibRsvg
[QtSvg]: https://doc.qt.io/qt-5/qtsvg-index.html

[cairo]: https://www.cairographics.org/
[Qt]: https://www.qt.io/
[Skia]: https://skia.org/

[GNOME]: https://www.gnome.org/
