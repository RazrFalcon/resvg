# usvg
[![Crates.io](https://img.shields.io/crates/v/usvg.svg)](https://crates.io/crates/usvg)
[![Documentation](https://docs.rs/usvg/badge.svg)](https://docs.rs/usvg)

*usvg* (micro SVG) is an [SVG] simplification tool.

## Purpose

Imagine, that you have to extract some data from the [SVG] file, but your
library/framework/language doesn't have a good SVG library.
And all you need is paths data.

You can try to export it by yourself (how hard can it be, right).
All you need is an XML library (I'll hope that your language has one).
But soon you realize that paths data has a pretty complex format and a lot
of edge-cases. And we didn't mention attributes propagation, transforms,
visibility flags, attribute values validation, XML quirks, etc.
It will take a lot of time and code to implement this stuff correctly.

So, instead of creating a library that can be used from any language (impossible),
*usvg* takes a different approach. It converts an input SVG into an extremely
simple representation, which is still a valid SVG.
And now, all you need is an XML library with some small amount of code.

## Key features of the simplified SVG

- No basic shapes (rect, circle, etc). Only paths
- Simple paths:
  - Only *MoveTo*, *LineTo*, *CurveTo* and *ClosePath* will be produced
  - All path segments are in absolute coordinates
  - No implicit segment commands
  - All values are separated by a space
- All (supported) attributes are resolved. No implicit one
- `use` will be resolved
- Invisible elements will be removed
- Invalid elements (like `rect` with negative/zero size) will be removed
- Units (mm, em, etc.) will be resolved
- Comments will be removed
- DTD will be resolved
- CSS will be resolved
- `style` attribute will be resolved
- `inherit` attribute value will be resolved
- `currentColor` attribute value will be resolved
- Paint fallback will be resolved
- No `script` (simply ignoring it)

Full spec can be found [here](../docs/usvg_spec.adoc).

## Limitations

- Currently, its not lossless. Some SVG features isn't supported yet and will be ignored.
- CSS support is minimal.
- Only [static](http://www.w3.org/TR/SVG11/feature#SVG-static) SVG features,
  e.g. no: `a`, `view`, `cursor`, `script` and [animations](https://www.w3.org/TR/SVG/animate.html).
- Font-based elements are not supported.

## Dependency

The latest stable [Rust](https://www.rust-lang.org/).

## FAQ

### How to ensure that SVG is a valid "Micro" SVG?

You can't. The idea is that you should not store files produced by *usvg*.
You should use them immediately. Like an intermediate data.

## License

*usvg* is licensed under the [MPLv2.0](https://www.mozilla.org/en-US/MPL/).

[SVG]: https://en.wikipedia.org/wiki/Scalable_Vector_Graphics
