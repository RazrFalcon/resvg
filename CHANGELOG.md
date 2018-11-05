# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

This changelog also contains an important changes in dependencies.

## [Unreleased]
### Added
- (c-api) `RESVG_ERROR_PARSING_FAILED`.
- (c-api) `resvg_options::font_family` and `resvg_options::font_size`.
- (usvg) Default font family and size is configurable now.

### Changed
- (rendersvg) Use `gumdrop` instead of `getopts`.
- (c-api) Qt wrapper is header-only now.

### Fixed
- (cairo-backend) Text layout.
- (resvg) Recursive SVG images via `image` tag.
- (resvg) Bbox calculation of the text with rotate.
- (qt-api) SVG from QByteArray loading when data is invalid.
- (usvg) `display` attribute processing.

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

[Unreleased]: https://github.com/RazrFalcon/resvg/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/RazrFalcon/resvg/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/RazrFalcon/resvg/compare/v0.1.0...v0.2.0
