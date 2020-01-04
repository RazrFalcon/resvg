// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{ImageRef, ImageRefMut};

/// A color channel.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ColorChannel { R, G, B, A }

/// Applies a displacement map.
///
/// - `map` pixels should have a **unpremultiplied alpha**.
/// - `src` pixels can have any alpha method.
///
/// `sx` and `sy` indicate canvas scale.
///
/// # Panics
///
/// When `src`, `map` and `dest` have different sizes.
pub fn displacement_map(
    x_channel_selector: ColorChannel,
    y_channel_selector: ColorChannel,
    sx: f64,
    sy: f64,
    src: ImageRef,
    map: ImageRef,
    dest: ImageRefMut,
) {
    assert!(src.width == map.width && src.width == dest.width);
    assert!(src.height == map.height && src.height == dest.height);

    let w = src.width as i32;
    let h = src.height as i32;

    let mut x: u32 = 0;
    let mut y: u32 = 0;
    for pixel in map.data.iter() {
        let calc_offset = |channel| {
            let c = match channel {
                ColorChannel::B => pixel.b,
                ColorChannel::G => pixel.g,
                ColorChannel::R => pixel.r,
                ColorChannel::A => pixel.a,
            };

            c as f64 / 255.0 - 0.5
        };

        let dx = calc_offset(x_channel_selector);
        let dy = calc_offset(y_channel_selector);
        let ox = (x as f64 + dx * sx).round() as i32;
        let oy = (y as f64 + dy * sy).round() as i32;

        // TODO: we should use some kind of anti-aliasing when offset is on a pixel border

        if    x < w as u32 && y < h as u32
           && ox >= 0 && ox < w && oy >= 0 && oy < h
        {
            let idx = (oy * w + ox) as usize;
            let idx1 = (y * w as u32 + x) as usize;
            dest.data[idx1] = src.data[idx];
        }

        x += 1;
        if x == src.width {
            x = 0;
            y += 1;
        }
    }
}
