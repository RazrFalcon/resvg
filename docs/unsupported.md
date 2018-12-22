### Elements

- Filter based
  - `feConvolveMatrix`
  - `feDiffuseLighting`
  - `feDisplacementMap`
  - `feMorphology`
  - `feSpecularLighting`
  - `feDistantLight`
  - `fePointLight`
  - `feSpotLight`
  - `feImage` with a reference to an element
- Font based
  - `font`
  - `glyph`
  - `missing-glyph`
  - `hkern`
  - `vkern`
  - `font-face`
  - `font-face-src`
  - `font-face-uri`
  - `font-face-format`
  - `font-face-name`
  - `altGlyph`
  - `altGlyphDef`
  - `altGlyphItem`
  - `glyphRef`
- `color-profile`
- `marker`
- `textPath`
- `use` with a reference to an external SVG

### Attributes

- `alignment-baseline`
- Nested `baseline-shift`
- `clip` (deprecated in the SVG 2)
- `color-interpolation`
- `color-profile`
- `color-rendering`
- `direction`
- `dominant-baseline`
- [`enable-background`](https://www.w3.org/TR/SVG11/filters.html#EnableBackgroundProperty) (deprecated in the SVG 2)
- `font`
- `font-size-adjust`
- `glyph-orientation-horizontal` (removed in the SVG 2)
- `glyph-orientation-vertical` (deprecated in the SVG 2)
- [`in`](https://www.w3.org/TR/SVG11/filters.html#FilterPrimitiveInAttribute)
  with `BackgroundImage`, `BackgroundAlpha`, `FillPaint`, `StrokePaint`
- `image-rendering`
- `kerning`
- `lighting-color`
- `marker-start`
- `marker-mid`
- `marker-end`
- `shape-rendering`
- `text-rendering`
- `unicode-bidi`
- `word-spacing` (unsupported only on the cairo backend)
- `writing-mode`

**Note:** this list does not include elements and attributes outside the
[static SVG](http://www.w3.org/TR/SVG11/feature#SVG-static) subset.
