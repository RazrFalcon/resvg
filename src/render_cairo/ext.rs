// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use cairo;
use tree;


pub trait ReCairoContextExt {
    fn set_source_color(&self, color: &tree::Color, opacity: f64);
    fn reset_source_rgba(&self);
}

impl ReCairoContextExt for cairo::Context {
    fn set_source_color(&self, color: &tree::Color, opacity: f64) {
        self.set_source_rgba(color.red as f64 / 255.0,
                             color.green as f64 / 255.0,
                             color.blue as f64 / 255.0,
                             opacity);
    }

    fn reset_source_rgba(&self) {
        self.set_source_rgba(0.0, 0.0, 0.0, 0.0);
    }
}
