// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

pub mod filter;
pub mod image;
pub mod mask;

mod prelude {
    pub use super::super::prelude::*;
}

pub fn use_shape_antialiasing(
    mode: usvg::ShapeRendering,
) -> bool {
    match mode {
        usvg::ShapeRendering::OptimizeSpeed         => false,
        usvg::ShapeRendering::CrispEdges            => false,
        usvg::ShapeRendering::GeometricPrecision    => true,
    }
}

pub trait ConvTransform<T> {
    fn to_native(&self) -> T;
    fn from_native(_: &T) -> Self;
}
