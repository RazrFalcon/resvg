// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use qt;
use dom;


pub fn draw(
    image: &dom::Image,
    p: &qt::Painter,
) {
    let img = match image.data {
        dom::ImageData::Path(ref path) => {
            match qt::Image::from_file(path) {
                Some(v) => v,
                None => {
                    warn!("Failed to load an external image: {:?}.", path);
                    return;
                }
            }
        }
        dom::ImageData::Raw(ref data, _) => {
            match qt::Image::from_data(data) {
                Some(v) => v,
                None => {
                    warn!("Failed to load an embedded image.");
                    return;
                }
            }
        }
    };

    let img = match img.resize(image.rect.w as i32, image.rect.h as i32) {
        Some(v) => v,
        None => {
            warn!("Failed to scale an image.");
            return;
        }
    };

    p.draw_image(image.rect.x, image.rect.y, &img);
}
