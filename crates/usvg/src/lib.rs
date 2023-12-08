// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/*!
`usvg` (micro SVG) is an [SVG] parser that tries to solve most of SVG complexity.

SVG is notoriously hard to parse. `usvg` presents a layer between an XML library and
a potential SVG rendering library. It will parse an input SVG into a strongly-typed tree structure
were all the elements, attributes, references and other SVG features are already resolved
and presented in a simplest way possible.
So a caller doesn't have to worry about most of the issues related to SVG parsing
and can focus just on the rendering part.

## Features

- All supported attributes are resolved.
  No need to worry about inheritable, implicit and default attributes
- CSS will be applied
- Only simple paths
  - Basic shapes (like `rect` and `circle`) will be converted into paths
  - Paths contain only absolute *MoveTo*, *LineTo*, *CurveTo* and *ClosePath* segments.
    ArcTo, implicit and relative segments will be converted
- `use` will be resolved and replaced with the reference content
- Nested `svg` will be resolved
- Invalid, malformed elements will be removed
- Relative length units (mm, em, etc.) will be converted into pixels/points
- External images will be loaded
- Internal, base64 images will be decoded
- All references (like `#elem` and `url(#elem)`) will be resolved
- `switch` will be resolved
- Text elements, which are probably the hardest part of SVG, will be completely resolved.
  This includes all the attributes resolving, whitespaces preprocessing (`xml:space`),
  text chunks and spans resolving
- Markers will be converted into regular elements. No need to place them manually
- All filters are supported. Including filter functions, like `filter="contrast(50%)"`
- Recursive elements will be detected an removed

## Limitations

- Unsupported SVG features will be ignored
- CSS support is minimal
- Only [static](http://www.w3.org/TR/SVG11/feature#SVG-static) SVG features,
  e.g. no `a`, `view`, `cursor`, `script`, no events and no animations
- Text elements must be converted into paths before writing to SVG.

[SVG]: https://en.wikipedia.org/wiki/Scalable_Vector_Graphics
*/

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![warn(missing_copy_implementations)]

mod writer;

pub use usvg_parser::*;
#[cfg(feature = "text")]
pub use usvg_text_layout::*;
pub use usvg_tree::*;

pub use writer::XmlOptions;

/// A trait to write `usvg::Tree` back to SVG.
pub trait TreeWriting {
    /// Writes `usvg::Tree` back to SVG.
    fn to_string(&self, opt: &XmlOptions) -> String;
}

impl TreeWriting for usvg_tree::Tree {
    fn to_string(&self, opt: &XmlOptions) -> String {
        writer::convert(self, opt)
    }
}
