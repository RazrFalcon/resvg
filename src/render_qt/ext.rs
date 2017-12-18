// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use qt;
use svgdom;


pub trait TransformToMatrix {
    fn to_qtransform(&self) -> qt::Transform;
}

impl TransformToMatrix for svgdom::types::Transform {
    fn to_qtransform(&self) -> qt::Transform {
        qt::Transform::new(self.a, self.b, self.c, self.d, self.e, self.f)
    }
}
