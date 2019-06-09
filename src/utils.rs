// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Some useful utilities.

pub use usvg::utils::*;

use super::prelude::*;
use crate::FitTo;


/// Returns `size` preprocessed according to `FitTo`.
pub(crate) fn fit_to(
    size: ScreenSize,
    fit: FitTo,
) -> Option<ScreenSize> {
    let sizef = size.to_size();

    match fit {
        FitTo::Original => {
            Some(size)
        }
        FitTo::Width(w) => {
            let h = (w as f64 * sizef.height() / sizef.width()).ceil();
            ScreenSize::new(w, h as u32)
        }
        FitTo::Height(h) => {
            let w = (h as f64 * sizef.width() / sizef.height()).ceil();
            ScreenSize::new(w as u32, h)
        }
        FitTo::Zoom(z) => {
            Size::new(sizef.width() * z as f64, sizef.height() * z as f64)
                 .map(|s| s.to_screen_size())
        }
    }
}

pub(crate) fn apply_view_box(
    vb: &usvg::ViewBox,
    img_size: ScreenSize,
) -> ScreenSize {
    let s = vb.rect.to_screen_size();

    if vb.aspect.align == usvg::Align::None {
        s
    } else {
        if vb.aspect.slice {
            img_size.expand_to(s)
        } else {
            img_size.scale_to(s)
        }
    }
}

/// Returns node's absolute transform.
///
/// Does not include the node's transform itself.
pub fn abs_transform(
    node: &usvg::Node,
) -> usvg::Transform {
    let mut ts_list = Vec::new();
    for p in node.ancestors().skip(1) {
        ts_list.push(p.transform());
    }

    let mut root_ts = usvg::Transform::default();
    for ts in ts_list.iter().rev() {
        root_ts.append(ts);
    }

    root_ts
}
