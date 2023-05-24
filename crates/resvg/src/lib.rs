// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/*!
[resvg](https://github.com/RazrFalcon/resvg) is an SVG rendering library.
*/

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::identity_op)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::wrong_self_convention)]

pub use tiny_skia;
pub use usvg;

mod clip;
mod filter;
mod geom;
mod image;
mod mask;
mod paint_server;
mod path;
mod render;
mod tree;

pub use crate::tree::Tree;
