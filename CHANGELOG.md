# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased]

### Added
- (svg) Check that an external image is PNG or JPEG.
- (svg) Partial `clipPath` support.
  - `clip-path` attribute inside a `clipPath` element is not supported.
- (svg) Partial `pattern` support.
  - `preserveAspectRatio` attribute is not supported.
- (cli) Added `--query-all` and `--export-id` arguments to render SVG items by ID.

### Changed
- (lib) API is completely new.

### Fixed
- `font-size` attribute inheritance during `use` resolving.

[Unreleased]: https://github.com/RazrFalcon/svgcleaner/compare/v0.1.0...HEAD
