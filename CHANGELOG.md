# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased]
### Added
- (c-api) `resvg_is_image_empty`.
- (c-api) `resvg_error` enum.
- (c-api) Qt wrapper.
- (rendersvg) Use `getopts` instead of `clap` to reduce the executable size.

### Changed
- (c-api) `resvg_parse_tree_from_file`, `resvg_parse_tree_from_data`
  `resvg_cairo_render_to_image` and `resvg_qt_render_to_image`
  will return an error code now.
- (cairo-backend) Use `gdk-pixbuf` crate instead of `image`.
- (lib) `Render::render_to_image` and `Render::render_node_to_image` will return
  `Option` and not `Result` now.
- (lib) New geometry primitives implementation.

### Fixed
- (qt-backend) Gradient with `objectBoundingBox` rendering.
- (qt-backend) Text bounding box detection during the rendering.
- (cairo-backend) `image` element clipping.
- (cairo-backend) Layers management.
- (c-api) `resvg_get_node_transform` will return a correct transform now.
- (lib) `text-decoration` thickness.

### Removed
- (c-api) `resvg_error_msg_destroy`.
- (lib) `parse_rtree_*` methods. Use `usvg::Tree::from_` instead.
- (lib) `Error`.

## [0.2.0] - 2018-04-24
### Added
- (svg) Partial `clipPath` support.
- (svg) Partial `mask` support.
- (svg) Partial `pattern` support.
- (svg) `preserveAspectRatio` support.
- (svg) Check that an external image is PNG or JPEG.
- (cli) Added `--query-all` and `--export-id` arguments to render SVG items by ID.
- (cli) Added `--perf` argument for a simple performance stats.

### Changed
- (lib) API is completely new.

### Fixed
- `font-size` attribute inheritance during `use` resolving.

[Unreleased]: https://github.com/RazrFalcon/resvg/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/RazrFalcon/resvg/compare/0.1.0...0.2.0
