# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased]

### Added
- The `Document` internals are public now.
- Check that an external image is PNG or JPEG.
- Partial `clipPath` support.

### Changed
- `dom` -> `tree`.
- `dom::Document` -> `tree::RenderTree`.
- New render tree implementation.
- The `render_to_canvas` methods don't reset the global transform now.

### Fixed
- `font-size` attribute inheritance during `use` resolving.

[Unreleased]: https://github.com/RazrFalcon/svgcleaner/compare/v0.1.0...HEAD
