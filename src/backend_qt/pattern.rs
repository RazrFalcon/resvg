// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use qt;
use usvg;
use usvg::prelude::*;

// self
use super::prelude::*;

pub fn apply(
    pattern_node: &usvg::Node,
    pattern: &usvg::Pattern,
    opt: &Options,
    global_ts: qt::Transform,
    bbox: Rect,
    opacity: usvg::Opacity,
    brush: &mut qt::Brush,
) {
    let r = if pattern.units == usvg::Units::ObjectBoundingBox {
        pattern.rect.transform(usvg::Transform::from_bbox(bbox))
    } else {
        pattern.rect
    };

    let global_ts = usvg::Transform::from_native(&global_ts);
    let (sx, sy) = global_ts.get_scale();
    // Only integer scaling is allowed.
    let (sx, sy) = (sx.round(), sy.round());

    let img_size = Size::new(r.width * sx, r.height * sy).to_screen_size();
    let mut img = try_create_image!(img_size, ());

    img.set_dpi(opt.usvg.dpi);
    img.fill(0, 0, 0, 0);

    let p = qt::Painter::new(&img);

    p.scale(sx, sy);
    if let Some(vbox) = pattern.view_box {
        let ts = utils::view_box_to_transform(vbox.rect, vbox.aspect, r.size());
        p.apply_transform(&ts.to_native());
    } else if pattern.content_units == usvg::Units::ObjectBoundingBox {
        // 'Note that this attribute has no effect if attribute `viewBox` is specified.'

        // We don't use Transform::from_bbox(bbox) because `x` and `y` should be
        // ignored for some reasons...
        p.scale(bbox.width, bbox.height);
    }

    let mut layers = super::create_layers(img_size, opt);
    super::render_group(pattern_node, opt, &mut layers, &p);
    p.end();

    let img = if opacity.fuzzy_ne(&1.0) {
        // If `opacity` isn't `1` then we have to make image semitransparent.
        // The only way to do this is by making a new image and rendering
        // the pattern on it with transparency.

        let mut img2 = try_create_image!(img_size, ());
        img2.fill(0, 0, 0, 0);

        let p2 = qt::Painter::new(&img2);
        p2.set_opacity(*opacity);
        p2.draw_image(0.0, 0.0, &img);
        p2.end();

        img2
    } else {
        img
    };

    brush.set_pattern(img);

    let mut ts = usvg::Transform::default();
    ts.append(&pattern.transform);
    ts.translate(r.x, r.y);
    ts.scale(1.0 / sx, 1.0 / sy);
    brush.set_transform(ts.to_native());
}
