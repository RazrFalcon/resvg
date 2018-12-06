// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// self
use geom::*;


/// Converts an image to an alpha mask.
pub fn image_to_mask(
    data: &mut [u8],
    img_size: ScreenSize,
) {
    let width = img_size.width;
    let height = img_size.height;
    let stride = width * 4;

    let coeff_r = 0.2125 / 255.0;
    let coeff_g = 0.7154 / 255.0;
    let coeff_b = 0.0721 / 255.0;

    for y in 0..height {
        for x in 0..width {
            let idx = (y * stride + x * 4) as usize;

            let r = data[idx + 2] as f64;
            let g = data[idx + 1] as f64;
            let b = data[idx + 0] as f64;

            let luma = r * coeff_r + g * coeff_g + b * coeff_b;

            data[idx + 0] = 0;
            data[idx + 1] = 0;
            data[idx + 2] = 0;
            data[idx + 3] = f64_bound(0.0, luma * 255.0, 255.0) as u8;
        }
    }
}
