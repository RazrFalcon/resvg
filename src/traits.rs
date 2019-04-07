// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use usvg;

// self
use geom::*;


pub(crate) trait ConvTransform<T> {
    fn to_native(&self) -> T;
    fn from_native(&T) -> Self;
}


pub(crate) trait TransformFromBBox: Sized {
    fn from_bbox(bbox: Rect) -> Self;
}

impl TransformFromBBox for usvg::Transform {
    fn from_bbox(bbox: Rect) -> Self {
        Self::new(bbox.width(), 0.0, 0.0, bbox.height(), bbox.x(), bbox.y())
    }
}
