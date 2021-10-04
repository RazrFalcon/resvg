# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

This changelog also contains important changes in dependencies.

## [Unreleased]

## [0.19.0] - 2021-10-04
### Added
- Better text-on-path converter accuracy by accounting the current transform.

### Changed
- `usvg::NodeExt::abs_transform` includes current node transform now.
- Improved turbulence filter performance. Thanks to [akindle](https://github.com/akindle).
- Multiple dependencies updated.

## [0.18.0] - 2021-09-12
### Added
- `filter` build feature. Enabled by default.
- `usvg::PathBbox` and `resvg_path_bbox` (to C API).

### Changed
- (usvg) All filter related types are under the `filter` module now.
- (usvg) Remove `Fe` prefix from all filter types.
- (c-api) `resvg_get_node_bbox` returns `resvg_path_bbox` now.

### Fixed
- Horizontal and vertical lines processing.
- C API building without the `text` feature.

## [0.17.0] - 2021-09-04
### Added
- `tiny-skia` updated with support of images larger than 8000x8000 pixels.
- `feDropShadow` support. SVG2
- [`<filter-value-list>`](https://www.w3.org/TR/filter-effects-1/#typedef-filter-value-list) support.
  Meaning that the `filter` attribute can have multiple values now.
  Like `url(#filter1) blur(2)`. SVG2
- All [filter functions](https://www.w3.org/TR/filter-effects-1/#filter-functions). SVG2
- Support all [new](https://www.w3.org/TR/compositing-1/#ltblendmodegt) `feBlend` modes. SVG2
- Automatic SVG size detection when `width`/`height`/`viewBox` is not set.
  Thanks to [reknih](https://github.com/reknih).
- `usvg::Options::default_size`
- `--default-width` and `--default-height` to usvg.

### Changed
- `usvg::Group::filter` is a list of filter IDs now.
- `usvg::FeColorMatrixKind::Saturate` accepts any positive `f64` value now.
- `svgfilters::ColorMatrix::Saturate` accepts any positive `f64` value now.
- Fonts memory mapping was split into a separate build feature: `memmap-fonts`.
  Now you can build resvg/usvg with `system-fonts`, but without `memmap-fonts`.
  Enabled by default.
- The `--dump-svg` argument in resvg CLI tool should be enabled using `--features dump-svg` now.
  No enabled by default.
- `usvg::Tree::to_string` is behind the `export` build feature now.

### Fixed
- When writing SVG, `usvg` will use `rgba()` notations for colors instead of `#RRGGBB`.

## [0.16.0] - 2021-08-22
### Added
- CSS3 colors support. Specifically `rgba`, `hsl`, `hsla` and `transparent`. SVG2
- Allow missing `rx`/`ry` attributes on `ellipse`. SVG2
- Allow markers on all shapes. SVG2
- `textPath` can reference basic shapes now. SVG2
- `usvg::OptionsRef`, which is a non-owned `usvg::Options` variant.
- `simplecss` updated with CSS specificity support.
- `turn` angle unit support. SVG2
- Basic `font-variant=small-caps` support. No font fallback.
- `--export-area-page` to resvg.
- `--export-area-drawing` to resvg.

### Changed
- `resvg::render_node` requires `usvg::Tree` now.
- `usvg::Color` gained an `alpha` field.

### Removed
- `usvg::Node::tree`. Cannot be implemented efficiently anymore.
- `usvg::SystemFontDB`. No longer needed.

### Fixed
- `pattern` scaling.
- Greatly improve `symbol` resolving speed in `usvg`.
- Whitespaces trimming on nested `tspan`.

## [0.15.0] - 2021-06-13
### Added
- Allow reading SVG from stdin in `resvg` binary.
- `--id-prefix` to `usvg`.
- `FitTo::Size`
- `resvg` binary accepts `--width` and `--height` args together now.
  Previously, only `--width` or `--height` were allowed.
- `usvg::Path::text_bbox`
- The maximum number of SVG elements is limited by 1_000_000 now.
  Mainly to prevent a billion laugh style attacks.
- The maximum SVG elements nesting is limited by 1024 now.
- `usvg::Error::ElementsLimitReached`

### Changed
- Improve clipping and masking performance on large images.
- Remove layers caching. This was a pointless optimization.
- Split _Preprocessing_ into _Reading_ and _Parsing_ in `resvg --perf`.
- `usvg::XmlOptions` rewritten.
- `usvg::Tree::to_string` requires a reference to `XmlOptions` now.

### Removed
- `usvg::Tree::from_file`. Use `from_data` or `from_str` instead.
- `usvg::Error::InvalidFileSuffix`
- `usvg::Error::FileOpenFailed`
- (c-api) `RESVG_ERROR_INVALID_FILE_SUFFIX`

### Fixed
- Ignore tiny blur values. It could lead to a transparent image.
- `use` style propagation when used with `symbol`.
- Vertical text layout with relative offsets.
- Text bbox calculation. `usvg` uses font metrics instead of path bbox now.

## [0.14.1] - 2021-04-18
### Added
- Allow `href` without the `xlink` namespace.
  This feature is part of SVG 2 (which we do not support),
  but there are more and more files like this in the wild.

### Changed
- (usvg) Do not write `usvg:version` to the output SVG.

### Fixed
- (usvg) `overflow='inherit'` resolving.
- (usvg) SVG Path length calculation that affects `startOffset` property in `textPath`.
- (usvg) Fix `feImage` resolving when the linked element has
  `opacity`, `clip-path`, `mask` and/or `filter` attributes.
- (usvg) Fix chained `feImage` resolving.
- CLI arguments processing.

## [0.14.0] - 2021-03-06
### Fixed
- Multiple critical bugs in `tiny-skia`.

## [0.13.1] - 2021-01-20
### Fixed
- `image` with float size scaling.
- Critical bug in `tiny-skia`.

## [0.13.0] - 2020-12-21
### Added
- `--resources-dir` option to CLI tools.
- (usvg) `Tree::from_xmltree`

### Changed
- Remove the `Image` struct. `render()` and `render_node()` methods now accept `tiny_skia::PixmapMut`.
- Update `fontdb`.
- Update `tiny-skia`.
- (c-api) `resvg_size` uses `double` instead of `uint32_t` now.
- (qt-api) `defaultSize()` and `defaultSizeF()` methods now return SVG size
  and not SVG viewbox size.
- (usvg) `Options::path` changed to `Options::resources_dir` and requires a directory now.
- (c-api) `resvg_options_set_file_path` changed to `resvg_options_set_resources_dir`
  and requires a directory now.
- (qt-api) `ResvgOptions::setFilePath` changed to `ResvgOptions::setResourcesDir`
  and requires a directory now.

### Fixed
- Support multiple values inside a `text-decoration` attribute.

### Removed
- `Image`. Use `tiny_skia::PixmapMut` instead.
- (c-api) `resvg_image` struct and `resvg_image_*` methods. `resvg` renders onto
  the provided buffer now.
- (c-api) `resvg_color`, because unused.

## [0.12.0] - 2020-12-05
### Changed
- resvg no longer requires a C++ compiler!
- `tiny-skia` was updated to a pure Rust version, which means that `resvg` no longer
  depends on `clang` and should work on 32bit targets.
- `rustybuzz` was updated to a pure Rust version.
- `tools/explorer-thumbnailer` is back and written in Rust now.
  Thanks to [gentoo90](https://github.com/gentoo90).

### Fixed
- (usvg) Do not panic when a font has a zero-sized underline thickness.
- (usvg) Multiple `textPath` processing fixes by [chubei-oppen](https://github.com/chubei-oppen).
- (qt-api) `boundsOnElement` and `boundingBox` were returning transposed bounds.

## [0.11.0] - 2020-07-04
### Highlights
- All backends except Skia were removed. Skia is the only official one from now.
- New C API implementation.

### Added
- Support for user-defined fonts in usvg, resvg and C API.
- `--serif-family`, `--sans-serif-family`, `--cursive-family`, `--fantasy-family`
  `--monospace-family`, `--use-font-file`, `--use-fonts-dir`, `--skip-system-fonts` and `--list-fonts`
  options to all CLI tools.
- New tests suite. Instead of testing against the previous build, now we're testing against
  prerendered PNG images. Which is way faster.<br>
  And you can test resvg without the internet connection now.<br>
  And all you need is just `cargo test`.

### Changed
- Library uses an embedded Skia by default now.
- Switch `harfbuzz_rs` with `rustybuzz`.
- Rendering doesn't require `usvg::Options` now.
- (usvg) The `fontdb` module moved into its own crate.
- (usvg) `fontconfig` is no longer used for matching
  [generic fonts](https://www.w3.org/TR/2018/REC-css-fonts-3-20180920/#generic-family-value)
  on Linux. Mainly because it's very slow.
- (usvg) When an `image` element contains a file path, the file will be loaded into memory now,
  instead of simply storing a file path. And will be dumped as base64 on SVG save.
  In case of an SVG image, it will be loaded as a `Tree` and saved as base64 encoded XML on save.
- (usvg) `ImageData` replaced with `ImageKind`.
- (usvg) Fonts database is empty by default now and should be filled manually.
- (c-api) Almost a complete rewrite.

### Removed
- All backends except the Skia one.
- `Options` from all backends. We don't use it anymore.
- (usvg) `ImageFormat`.
- (c-api) Rendering on a backends canvas no longer supported. Was constantly misused.

## [0.10.0] - 2020-06-19

### Changed
- The `resvg` crate has been split into four: resvg-cairo, resvg-qt, resvg-skia and resvg-raqote.<br/>
  So from now, instead of enabling a required backend via cargo features,
  you should select a required backend-specific crate.<br/>
  This allows us to have a better integration with a selected 2D library.<br/>
  And we also have separated C API implementations now.<br/>
  And each backend has its own vendored archive too.
- (qt-backend) Use `QImage` instead of Rust libraries for raster images loading.

### Removed
- The `resvg` crate. Use backend-specific crates.
- `tools/rendersvg`. Each backend has its own CLI tool now.
- `tools/usvg`. `usvg` implements CLI by default now.
- (c-api) `resvg_*_render_to_file` methods.
- (qt-backend) `jpeg-decoder` and `png` dependencies.

## [0.9.1] - 2020-06-03
### Fixed
- Stack overflow when `enable-background` and `filter` are set on the same element.
- Grayscale PNG loading.
- Allow building on BSD.
- (usvg) Font fallback when shaping produces a different amount of glyphs.
- (usvg) Ignore a space after the last character during `letter-spacing` processing.
- (usvg) `marker-end` rendering when the last segment is a curve with the second control point
  that coincides with end point.
- (usvg) Accept embedded `image` data without mime.
- (usvg) Fonts search in a home directory on Linux.
- (usvg) `dy` calculation for `textPath` thanks to [Stoeoef](https://github.com/Stoeoef)
- (usvg) `textPath` resolving when a referenced path has a transform.<br/>
  Thanks to [Stoeoef](https://github.com/Stoeoef).
- (usvg) Load user fonts on macOS too.
- (xmlparser) Parsing comment before DTD.

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

[Unreleased]: https://github.com/RazrFalcon/resvg/compare/v0.19.0...HEAD
[0.19.0]: https://github.com/RazrFalcon/resvg/compare/v0.18.0...v0.19.0
[0.18.0]: https://github.com/RazrFalcon/resvg/compare/v0.17.0...v0.18.0
[0.17.0]: https://github.com/RazrFalcon/resvg/compare/v0.16.0...v0.17.0
[0.16.0]: https://github.com/RazrFalcon/resvg/compare/v0.15.0...v0.16.0
[0.15.0]: https://github.com/RazrFalcon/resvg/compare/v0.14.1...v0.15.0
[0.14.1]: https://github.com/RazrFalcon/resvg/compare/v0.14.0...v0.14.1
[0.14.0]: https://github.com/RazrFalcon/resvg/compare/v0.13.1...v0.14.0
[0.13.1]: https://github.com/RazrFalcon/resvg/compare/v0.13.0...v0.13.1
[0.13.0]: https://github.com/RazrFalcon/resvg/compare/v0.12.0...v0.13.0
[0.12.0]: https://github.com/RazrFalcon/resvg/compare/v0.11.0...v0.12.0
[0.11.0]: https://github.com/RazrFalcon/resvg/compare/v0.10.0...v0.11.0
[0.10.0]: https://github.com/RazrFalcon/resvg/compare/v0.9.1...v0.10.0
[0.9.1]: https://github.com/RazrFalcon/resvg/compare/v0.9.0...v0.9.1
[0.9.0]: https://github.com/RazrFalcon/resvg/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/RazrFalcon/resvg/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/RazrFalcon/resvg/compare/v0.6.1...v0.7.0
[0.6.1]: https://github.com/RazrFalcon/resvg/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/RazrFalcon/resvg/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/RazrFalcon/resvg/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/RazrFalcon/resvg/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/RazrFalcon/resvg/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/RazrFalcon/resvg/compare/v0.1.0...v0.2.0
