// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use cairo::{
    Context,
    Matrix,
    MatrixTrait,
};
use svgdom::types::{
    Color,
    Transform,
};


pub trait ReCairoContextExt {
    fn set_source_color(&self, color: &Color, opacity: f64);
    fn reset_source_rgba(&self);
    fn set_transform(&self, ts: &Transform);
    fn apply_transform(&self, ts: &Transform);
}

impl ReCairoContextExt for Context {
    fn set_source_color(&self, color: &Color, opacity: f64) {
        self.set_source_rgba(color.red as f64 / 255.0,
                             color.green as f64 / 255.0,
                             color.blue as f64 / 255.0,
                             opacity);
    }

    fn reset_source_rgba(&self) {
        self.set_source_rgba(0.0, 0.0, 0.0, 0.0);
    }

    fn set_transform(&self, ts: &Transform) {
        self.set_matrix(ts.to_matrix());
    }

    fn apply_transform(&self, ts: &Transform) {
        self.transform(ts.to_matrix());
    }
}


pub trait TransformToMatrix {
    fn to_matrix(&self) -> Matrix;
}

impl TransformToMatrix for Transform {
    fn to_matrix(&self) -> Matrix {
        Matrix::new(self.a, self.b, self.c, self.d, self.e, self.f)
    }
}
