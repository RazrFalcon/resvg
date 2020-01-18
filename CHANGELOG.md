# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

This changelog also contains important changes in dependencies.

## [Unreleased]

## [0.9.0] - 2020-01-18
### Added
- `feConvolveMatrix`, `feMorphology`, `feDisplacementMap`, `feTurbulence`,
  `feDiffuseLighting` and `feSpecularLighting` support.
- `BackgroundImage`, `BackgroundAlpha`, `FillPaint` and `StrokePaint` support as a filter input.
- Load grayscale raster images.
- `enable-background` support.
- resvg/usvg can be built without text rendering support now.
- `OutputImage::make_vec` and `OutputImage::make_rgba_vec`.
- `feImage` with a reference to an internal element.

### Changed
- `feComposite` k1-4 coefficients can have any number now.
  This matches browsers behaviour.
- Use `flate2` instead of `libflate` for GZip decoding.
- (usvg) `fill` and `stroke` attributes will always be set for `path` now.
- (usvg) `g`, `path` and `image` can now be set inside `defs`. Required by `feImage`.
- (c-api) Rename `resvg_*_render_to_image` into `resvg_*_render_to_file`.

### Fixed
- (usvg) Transform processing during text-to-path conversion.
- `feComposite` with fully transparent region was producing an invalid result.
- Fallback to `matrix` in `feColorMatrix` when `type` is not set or invalid.
- ID preserving for `use` elements.
- `feFlood` with subregion and `primitiveUnits=objectBoundingBox`.
- (harfbuzz_rs) Memory leak.

## [0.8.0] - 2019-08-17
### Added
- A [Skia](https://skia.org/) backend thanks to
  [JaFenix](https://github.com/JaFenix).
- `feComponentTransfer` support.
- `feColorMatrix` support.
- A better CSS support.
- An `*.otf` fonts support.
- (usvg) `dx`, `dy` are supported inside `textPath` now.
- Use a box blur for `feGaussianBlur` with `stdDeviation`>=2.
  This is 4-8 times faster than IIR blur.
  Thanks to [Shnatsel](https://github.com/Shnatsel).

### Changed
- All backends are using Rust crates for raster images loading now.
- Use `pico-args` instead of `gumdrop` to reduced the build time of `tools/rendersvg`
  and `tools/usvg`.
- (usvg) The `xmlwriter` is used for SVG generation now.
  Almost 2x faster than generating an `svgdom`.
- (usvg) Optimize font database initialization. Almost 50% faster.
- Use a lower PNG compression ratio to speed up PNG generation.
  Depending on a backend and image can be 2-4x faster.
- `OutputImage::save` -> `OutputImage::save_png`.
- (usvg) `Path::segments` -> `Path::data`.
- Cairo backend compilation is 2x faster now due to overall changes.
- Performance improvements (Oxygen Icon theme SVG-to-PNG):
  - cairo-backend: 22% faster
  - qt-backend: 20% faster
  - raqote-backend: 34% faster

### Fixed
- (qt-api) A default font resolving.
- (usvg) `baseline-shift` processing inside `textPath`.
- (usvg) Remove all `tref` element children.
- (usvg) `tref` with `xml:space` resolving.
- (usvg) Ignore nested `tref`.
- (usvg) Ignore invalid `clipPath` children that were referenced via `use`.
- (usvg) `currentColor` will always fallback to black now.
  Previously, `stroke` was set to `none` which is incorrect.
- (usvg) `use` can reference an element inside a non-SVG element now.
- (usvg) Collect all styles for generic fonts and not only *Regular*.
- (usvg) Parse only presentation attributes from the `style` element and attribute.

### Removed
- (cairo-backend) `gdk-pixbuf` dependency.
- (qt-backend) JPEG image format plugin dependency.
- `svgdom` dependency.

## [0.7.0] - 2019-06-19
### Added
- New text layout implementation:
  - `textPath` support.
  - `writing-mode` support, aka vertical text.
  - [Text BIDI reordering](http://www.unicode.org/reports/tr9/).
  - Better text shaping.
  - `word-spacing` is supported for all backends now.
  - [`harfbuzz`](https://github.com/harfbuzz/harfbuzz) dependency.
  - Subscript, superscript offsets are extracted from font and not hardcoded now.
- `shape-rendering`, `text-rendering` and `image-rendering` support.
- The `arithmetic` operator for `feComposite`.
- (usvg) `--quiet` argument.
- (c-api) `resvg_get_image_bbox`.
- (qt-api) `ResvgRenderer::boundingBox`.
- (resvg) A [raqote](https://github.com/jrmuizel/raqote) backend thanks to
  [jrmuizel](https://github.com/jrmuizel). Still experimental.

### Changed
- Text will be converted into paths on the `usvg` side now.
- (resvg) Do not rescale images before rendering. This is faster and better.
- (usvg) An `image` element with a zero or negative size will be skipped now.
  Previously, a linked image size was used, which is incorrect.
- Geometry primitives (`Rect`, `Size`, etc) are immutable and always valid now.
- (usvg) The default `color-interpolation-filters` attribute will not be exported now.

### Removed
- (usvg) All text related structures and enums. Text will be converted into `Path` now.
- `InitObject` and `init()` because they are no longer needed.
- (c-api) `resvg_handle`, `resvg_init`, `resvg_destroy`.
- (c-api) `resvg_cairo_get_node_bbox` and `resvg_qt_get_node_bbox`.
  Use backend-independent `resvg_get_node_bbox` instead.
- (cairo-backend) `pango` dependency.
- (resvg) `Backend::calc_node_bbox`. Use `Node::calculate_bbox()` instead.

### Fixed
- `letter-spacing` on cursive scripts (like Arabic).
- (rctree) Prevent stack overflow on a huge, deeply nested SVG.
- (c-api) `resvg_is_image_empty` was always returning `false`.
- (resvg) Panic when `filter` with `objectBoudningBox` was set on an empty group.
- (usvg) `mask` with `objectBoundingBox` resolving.
- (usvg) `pattern`'s `viewBox` attribute resolving via `href`.
- (roxmltree) Namespace resolving.

## [0.6.1] - 2019-03-16
### Fixed
- (usvg) `transform` multiplication.
- (usvg) `use` inside `clipPath` resolving.

## [0.6.0] - 2019-03-16
### Added
- Nested `baseline-shift` support.
- (qt-api) `renderToImage`.
- (usvg) A better algorithm for unused defs (`defs` element children, like gradients) removal.
- (usvg) `Error::InvalidSize`.
- (c-api) `RESVG_ERROR_INVALID_SIZE`.

### Changed
- (usvg) A major rewrite.
- `baseline-shift` with `sub`, `super` and percent values calculation.
- Marker resolving moved completely to `usvg`.
- If an SVG doesn't have a valid size than an error will occur.
  Previously, an empty tree was produced.
- (qt-api) `render` methods are `const` now.
- (usvg) Disable default attributes exporting.

### Removed
- (usvg) Marker element and attributes. Markers will be resolved just like `use` now.

### Fixed
- (resvg) During the `tspan` rendering, the `text` bbox will be used instead
  of the `tspan` bbox itself. This is the correct behaviour by the SVG spec.
- (cairo-backend) `font-family` parsing.
- (usvg) `filter:none` processing.
- (usvg) `text` inside `text` processing.
- (usvg) Endless loop during `use` resolving.
- (usvg) Endless loop when SVG has indirect recursive `xlink:href` links.
- (usvg) Endless loop when SVG has recursive `marker-*` links.
- (usvg) Panic during `use` resolving.
- (usvg) Panic during inherited attributes resolving.
- (usvg) Groups regrouping.
- (usvg) `dx`/`dy` processing on `text`.
- (usvg) `textAnchor` resolving.
- (usvg) Ignore `fill-rule` on `text`.
- (svgtypes) Style with comments parsing.
- (roxmltree) Namespaces resolving.

## [0.5.0] - 2019-01-04
### Added
- `marker` support.
- Partial `baseline-shift` support.
- `letter-spacing` support.
- (qt-backend) `word-spacing` support.
  Does not work on the cairo backend.
- tools/explorer-thumbnailer
- tools/kde-dolphin-thumbnailer

### Fixed
- Object bounding box calculation.
- Pattern scaling.
- Nested `objectBoundigBox` support.
- (usvg) `color` on `use` resolving.
- (usvg) `offset` attribute resolving inside the `stop` element.
- (usvg) Ungrouping of groups with non-inheritable attributes.
- (usvg) `rotate` attribute resolving.
- (usvg) Paths without stroke and fill will no longer be removed.
  Required for a proper bbox resolving.
- (usvg) Coordinates resolving when units are `userSpaceOnUse`.
- (usvg) Groups regrouping. Caused an incorrect rendering of `clipPath`
  that had `filter` on a child.
- (usvg) Style attributes resolving on the root `svg` element.
- (usvg) `SmoothCurveTo` and `SmoothQuadratic` conversion.
- (usvg) `symbol` resolving.
- (cairo-backend) Font ascent calculation.
- (qt-backend) Stroking of LineTo specified as CurveTo.
- (svgdom) `stroke-miterlimit` attribute parsing.
- (svgdom) `length` and `number` attribute types parsing.
- (svgdom) `offset` attribute parsing.
- (svgdom) IRI resolving order when SVG has duplicated ID's.

## [0.4.0] - 2018-12-13
### Added
- (resvg) Initial filters support.
- (resvg) Nested `clipPath` and `mask` support.
- (resvg) MSVC support.
- (rendersvg) `font-family`, `font-size` and `languages` to args.
- (usvg) `systemLanguage` attribute support.
- (usvg) Default font family and size is configurable now.
- (c-api) `RESVG_ERROR_PARSING_FAILED`.
- (c-api) `font_family`, `font_size` and `languages` to `resvg_options`.
- (qt-api) `ResvgRenderer::setDevicePixelRatio`.

### Changed
- (rendersvg) Use `gumdrop` instead of `getopts`.
- (c-api) Qt wrapper is header-only now.

### Fixed
- (cairo-backend) Text layout.
- (cairo-backend) Rendering of a zero length subpath with a square cap.
- (qt-backend) Transform retrieving via Qt bindings.
- (resvg) Recursive SVG images via `image` tag.
- (resvg) Bbox calculation of the text with rotate.
- (resvg) Invisible elements processing.
- (qt-api) SVG from QByteArray loading when data is invalid.
- (usvg) `display` attribute processing.
- (usvg) Recursive `mask` resolving.
- (usvg) `inherit` attribute value resolving.
- (svgdom) XML namespaces resolving.

### Removed
- (rendersvg) `failure` dependency.

## [0.3.0] - 2018-05-23
### Added
- (c-api) `resvg_is_image_empty`.
- (c-api) `resvg_error` enum.
- (c-api) Qt wrapper.
- (resvg) Advanced text layout support (lists of x, y, dx, dy and rotate).
- (resvg) SVG support for `image` element.
- (usvg) `symbol` element support.
- (usvg) Nested `svg` elements support.
- (usvg) Paint fallback resolving.
- (usvg) Bbox validation for shapes that use painting servers.
- (svgdom) Elements from ENTITY resolving.

### Changed
- (c-api) `resvg_parse_tree_from_file`, `resvg_parse_tree_from_data`
  `resvg_cairo_render_to_image` and `resvg_qt_render_to_image`
  will return an error code now.
- (cairo-backend) Use `gdk-pixbuf` crate instead of `image`.
- (resvg) `Render::render_to_image` and `Render::render_node_to_image` will return
  `Option` and not `Result` now.
- (resvg) New geometry primitives implementation.
- (resvg) Rename `render_*` modules to `backend_`.
- (rendersvg) Use `getopts` instead of `clap` to reduce the executable size.
- (svgtypes) `StreamExt::parse_iri` and `StreamExt::parse_func_iri` will parse
  not only well-formed data now.

### Fixed
- (qt-backend) Gradient with `objectBoundingBox` rendering.
- (qt-backend) Text bounding box detection during the rendering.
- (cairo-backend) `image` element clipping.
- (cairo-backend) Layers management.
- (c-api) `resvg_get_node_transform` will return a correct transform now.
- (resvg) `text-decoration` thickness.
- (resvg) `pattern` scaling.
- (resvg) `image` without size rendering.
- (usvg) Panic during `visibility` resolving.
- (usvg) Gradients with one stop resolving.
- (usvg) `use` attributes resolving.
- (usvg) `clipPath` and `mask` attributes resolving.
- (usvg) `offset` attribute in `stop` element resolving.
- (usvg) Incorrect `font-size` attribute resolving.
- (usvg) Gradient stops resolving.
- (usvg) `switch` element resolving.
- (svgdom) Mixed `xml:space` processing.
- (svgtypes) `Paint::from_span` poor performance.

### Removed
- (c-api) `resvg_error_msg_destroy`.
- (resvg) `parse_rtree_*` methods. Use `usvg::Tree::from_` instead.
- (resvg) `Error`.

## [0.2.0] - 2018-04-24
### Added
- (svg) Partial `clipPath` support.
- (svg) Partial `mask` support.
- (svg) Partial `pattern` support.
- (svg) `preserveAspectRatio` support.
- (svg) Check that an external image is PNG or JPEG.
- (rendersvg) Added `--query-all` and `--export-id` arguments to render SVG items by ID.
- (rendersvg) Added `--perf` argument for a simple performance stats.

### Changed
- (resvg) API is completely new.

### Fixed
- `font-size` attribute inheritance during `use` resolving.

[Unreleased]: https://github.com/RazrFalcon/resvg/compare/v0.9.0...HEAD
[0.9.0]: https://github.com/RazrFalcon/resvg/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/RazrFalcon/resvg/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/RazrFalcon/resvg/compare/v0.6.1...v0.7.0
[0.6.1]: https://github.com/RazrFalcon/resvg/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/RazrFalcon/resvg/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/RazrFalcon/resvg/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/RazrFalcon/resvg/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/RazrFalcon/resvg/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/RazrFalcon/resvg/compare/v0.1.0...v0.2.0
