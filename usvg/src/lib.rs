// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/*!
`usvg` (micro SVG) is an [SVG] simplification tool.

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
*usvg* takes a different approach. It converts an input SVG to an extremely
simple representation, which is still a valid SVG.
And now, all you need is to convert your SVG to a simplified one via *usvg*
and an XML library with some small amount of code.

## Key features of the simplified SVG

- No basic shapes (rect, circle, etc). Only paths
- Simple paths:
  - Only MoveTo, LineTo, CurveTo and ClosePath will be produced
  - All path segments are in absolute coordinates
  - No implicit segment commands
  - All values are separated by space
- All (supported) attributes are resolved. No implicit one
- No `use`. Everything is resolved
- No invisible elements
- No invalid elements (like `rect` with negative/zero size)
- No units (mm, em, etc.)
- No comments
- No DTD
- No CSS (partial support)
- No `script` (simply ignoring it)

Full spec can be found [here](https://github.com/RazrFalcon/usvg/blob/master/docs/usvg_spec.adoc).

## Limitations

- Currently, it's not lossless. Some SVG features isn't supported yet and will be ignored.
- CSS support is minimal.
- Scripting and animation isn't supported and not planned.
- `a` elements will be removed.
- Unsupported elements:
  - some filter-based elements
  - font-based elements

[SVG]: https://en.wikipedia.org/wiki/Scalable_Vector_Graphics
*/

#![doc(html_root_url = "https://docs.rs/usvg/0.6.1")]

#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![warn(missing_copy_implementations)]

pub use svgdom;

/// Unwraps `Option` and invokes `return` on `None`.
#[macro_export]
macro_rules! try_opt {
    ($task:expr) => {
        match $task {
            Some(v) => v,
            None => return,
        }
    };
}

/// Unwraps `Option` and invokes `return $ret` on `None`.
#[macro_export]
macro_rules! try_opt_or {
    ($task:expr, $ret:expr) => {
        match $task {
            Some(v) => v,
            None => return $ret,
        }
    };
}

/// Unwraps `Option` and invokes `return` on `None` with a warning.
#[macro_export]
macro_rules! try_opt_warn {
    ($task:expr, $msg:expr) => {
        match $task {
            Some(v) => v,
            None => {
                log::warn!($msg);
                return;
            }
        }
    };
    ($task:expr, $fmt:expr, $($arg:tt)*) => {
        match $task {
            Some(v) => v,
            None => {
                log::warn!($fmt, $($arg)*);
                return;
            }
        }
    };
}

/// Unwraps `Option` and invokes `return $ret` on `None` with a warning.
#[macro_export]
macro_rules! try_opt_warn_or {
    ($task:expr, $ret:expr, $msg:expr) => {
        match $task {
            Some(v) => v,
            None => {
                log::warn!($msg);
                return $ret;
            }
        }
    };
    ($task:expr, $ret:expr, $fmt:expr, $($arg:tt)*) => {
        match $task {
            Some(v) => v,
            None => {
                log::warn!($fmt, $($arg)*);
                return $ret;
            }
        }
    };
}


pub mod utils;
mod convert;
mod error;
mod fontdb;
mod geom;
mod options;
mod tree;

/// Shorthand names for modules.
mod short {
    pub use svgdom::{
        LengthUnit as Unit,
        ElementId as EId,
        AttributeId as AId,
        AttributeValue as AValue,
    };
}


pub use crate::error::*;
pub use crate::geom::*;
pub use crate::options::*;
pub use crate::tree::*;


/// Checks that type has a default value.
pub trait IsDefault: Default {
    /// Checks that type has a default value.
    fn is_default(&self) -> bool;
}

impl<T: Default + PartialEq + Copy> IsDefault for T {
    fn is_default(&self) -> bool {
        *self == Self::default()
    }
}


/// Checks that the current number is > 0.
pub trait IsValidLength {
    /// Checks that the current number is > 0.
    fn is_valid_length(&self) -> bool;
}

impl IsValidLength for f64 {
    fn is_valid_length(&self) -> bool {
        *self > 0.0
    }
}


/// Converts `Rect` into bbox `Transform`.
pub trait TransformFromBBox: Sized {
    /// Converts `Rect` into bbox `Transform`.
    fn from_bbox(bbox: Rect) -> Self;
}

impl TransformFromBBox for tree::Transform {
    fn from_bbox(bbox: Rect) -> Self {
        Self::new(bbox.width(), 0.0, 0.0, bbox.height(), bbox.x(), bbox.y())
    }
}
