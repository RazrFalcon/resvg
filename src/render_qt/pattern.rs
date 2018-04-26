// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use qt;
use usvg::tree;
use usvg::tree::prelude::*;

// self
use geom::*;
use traits::{
    ConvTransform,
    TransformFromBBox,
};
use utils;
use {
    Options,
};

pub fn apply(
    pattern_node: &tree::Node,
    pattern: &tree::Pattern,
    opt: &Options,
    global_ts: qt::Transform,
    bbox: Rect,
    opacity: tree::Opacity,
    brush: &mut qt::Brush,
) {
    let r = if pattern.units == tree::Units::ObjectBoundingBox {
        pattern.rect.transform(tree::Transform::from_bbox(bbox))
    } else {
        pattern.rect
    };

    let global_ts = tree::Transform::from_native(&global_ts);
    let (sx, sy) = global_ts.get_scale();

    let img_size = Size::new(r.width() * sx, r.height() * sy).to_screen_size();
    let mut img = try_create_image!(img_size, ());

    img.set_dpi(opt.usvg.dpi);
    img.fill(0, 0, 0, 0);

    let p = qt::Painter::new(&img);

    p.apply_transform(&qt::Transform::new(sx, 0.0, 0.0, sy, 0.0, 0.0));
    if let Some(vbox) = pattern.view_box {
        let (dx, dy, sx2, sy2) = utils::view_box_transform(vbox, r.to_screen_size());
        p.apply_transform(&qt::Transform::new(sx2, 0.0, 0.0, sy2, dx, dy));
    } else if pattern.content_units == tree::Units::ObjectBoundingBox {
        // 'Note that this attribute has no effect if attribute `viewBox` is specified.'

        // We don't use Transform::from_bbox(bbox) because `x` and `y` should be
        // ignored for some reasons...
        p.apply_transform(&qt::Transform::new(bbox.width(), 0.0, 0.0, bbox.height(), 0.0, 0.0));
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

    let mut ts = tree::Transform::default();
    ts.append(&pattern.transform);
    ts.translate(r.x(), r.y());
    ts.scale(1.0 / sx, 1.0 / sy);
    brush.set_transform(ts.to_native());
}
