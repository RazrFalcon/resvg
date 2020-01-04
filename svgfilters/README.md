## svgfilters
[![Crates.io](https://img.shields.io/crates/v/svgfilters.svg)](https://crates.io/crates/svgfilters)
[![Documentation](https://docs.rs/svgfilters/badge.svg)](https://docs.rs/svgfilters)

`svgfilters` provides low-level [SVG filters](https://www.w3.org/TR/SVG11/filters.html)
implementation.

`svgfilters` doesn't implement the whole filters workflow, just operations on raster images.
Filter region calculation, image colors (un)premultiplication, input validation,
filter primitives order, transformations, etc. should be implemented by the caller.

### Implemented filters

- [feColorMatrix](https://www.w3.org/TR/SVG11/filters.html#feColorMatrixElement)
- [feComponentTransfer](https://www.w3.org/TR/SVG11/filters.html#feComponentTransferElement)
- [feComposite](https://www.w3.org/TR/SVG11/filters.html#feCompositeElement)
  Only the arithmetic operator is supported since other one are pretty common
  and should be implemented by the 2D library itself.
- [feConvolveMatrix](https://www.w3.org/TR/SVG11/filters.html#feConvolveMatrixElement)
- [feDiffuseLighting](https://www.w3.org/TR/SVG11/filters.html#feDiffuseLightingElement)
- [feDisplacementMap](https://www.w3.org/TR/SVG11/filters.html#feDisplacementMapElement)
- [feGaussianBlur](https://www.w3.org/TR/SVG11/filters.html#feGaussianBlurElement)
  Box blur and IIR blur variants are available.
- [feMorphology](https://www.w3.org/TR/SVG11/filters.html#feMorphologyElement)
- [feSpecularLighting](https://www.w3.org/TR/SVG11/filters.html#feSpecularLightingElement)
- [feTurbulence](https://www.w3.org/TR/SVG11/filters.html#feTurbulenceElement)

### Unimplemented filters

- [feFlood](https://www.w3.org/TR/SVG11/filters.html#feFloodElement),
  because it's just a simple fill.
- [feImage](https://www.w3.org/TR/SVG11/filters.html#feImageElement),
  because it can be implemented only by a caller.
- [feTile](https://www.w3.org/TR/SVG11/filters.html#feTileElement),
  because it's basically a fill with pattern.
- [feMerge](https://www.w3.org/TR/SVG11/filters.html#feMergeElement),
  because it's just a layer compositing and a 2D library will be faster.
- [feOffset](https://www.w3.org/TR/SVG11/filters.html#feOffsetElement),
  because it's just a layer compositing with offset.

### Performance

The library isn't well optimized yet, but it's mostly allocation free.
Some methods will allocate necessary, temporary buffers which will be reflected in the documentation.
But majority of methods will work on provided buffers.

### License

*svgfilters* is licensed under the [MPLv2.0](https://www.mozilla.org/en-US/MPL/).
