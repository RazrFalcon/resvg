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
  - `marker`

[SVG]: https://en.wikipedia.org/wiki/Scalable_Vector_Graphics
*/

#![doc(html_root_url = "https://docs.rs/usvg/0.6.1")]

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![warn(missing_copy_implementations)]


pub extern crate svgdom;
pub extern crate lyon_geom;
#[macro_use] extern crate log;
extern crate harfbuzz_rs as harfbuzz;


/// Task, return value.
#[macro_export]
macro_rules! try_opt {
    ($task:expr, $ret:expr) => {
        match $task {
            Some(v) => v,
            None => return $ret,
        }
    };
}

/// Task, return value, warning message.
#[macro_export]
macro_rules! try_opt_warn {
    ($task:expr, $ret:expr, $msg:expr) => {
        match $task {
            Some(v) => v,
            None => {
                warn!($msg);
                return $ret;
            }
        }
    };
    ($task:expr, $ret:expr, $fmt:expr, $($arg:tt)*) => {
        match $task {
            Some(v) => v,
            None => {
                warn!($fmt, $($arg)*);
                return $ret;
            }
        }
    };
}

/// Panics in debug, prints a warning in release.
macro_rules! debug_panic {
    ($msg:expr) => {
        debug_assert!(false, $msg);
        warn!($msg);
    };
    ($fmt:expr, $($arg:tt)*) => {
        debug_assert!(false, $fmt, $($arg)*);
        warn!($fmt, $($arg)*);
    };
}


pub mod utils;
mod convert;
mod error;
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
pub use crate::convert::{
    IsDefault,
    IsValidLength,
};
