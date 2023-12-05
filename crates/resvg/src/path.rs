// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::rc::Rc;

use crate::paint_server::Paint;
use crate::render::Context;
use crate::tree::{BBoxes, Node};

pub struct FillPath {
    pub paint: Paint,
    pub rule: tiny_skia::FillRule,
    pub anti_alias: bool,
    pub path: Rc<tiny_skia::Path>,
}

pub struct StrokePath {
    pub paint: Paint,
    pub stroke: tiny_skia::Stroke,
    pub anti_alias: bool,
    pub path: Rc<tiny_skia::Path>,
}

pub fn convert(
    upath: &usvg::Path,
    text_bbox: Option<tiny_skia::NonZeroRect>,
    children: &mut Vec<Node>,
) -> Option<BBoxes> {
    let anti_alias = upath.rendering_mode.use_shape_antialiasing();

    let fill_path = upath
        .fill
        .as_ref()
        .and_then(|ufill| convert_fill_path(ufill, upath.data.clone(), text_bbox, anti_alias));

    let stroke_path = upath.stroke.as_ref().and_then(|ustroke| {
        convert_stroke_path(ustroke, upath.data.clone(), text_bbox, anti_alias)
    });

    if fill_path.is_none() && stroke_path.is_none() {
        return None;
    }

    let mut bboxes = BBoxes::default();

    if let Some((_, o_bbox)) = fill_path {
        bboxes.layer = bboxes.layer.expand(o_bbox);
        bboxes.object = bboxes.object.expand(o_bbox);
    }
    if let Some((_, l_bbox, o_bbox)) = stroke_path {
        bboxes.layer = bboxes.layer.expand(l_bbox);
        bboxes.object = bboxes.object.expand(o_bbox);
    }

    // Do not add hidden paths, but preserve the bbox.
    // visibility=hidden still affects the bbox calculation.
    if upath.visibility != usvg::Visibility::Visible {
        return Some(bboxes);
    }

    if upath.paint_order == usvg::PaintOrder::FillAndStroke {
        if let Some((path, _)) = fill_path {
            children.push(Node::FillPath(path));
        }

        if let Some((path, _, _)) = stroke_path {
            children.push(Node::StrokePath(path));
        }
    } else {
        if let Some((path, _, _)) = stroke_path {
            children.push(Node::StrokePath(path));
        }

        if let Some((path, _)) = fill_path {
            children.push(Node::FillPath(path));
        }
    }

    Some(bboxes)
}

fn convert_fill_path(
    ufill: &usvg::Fill,
    path: Rc<tiny_skia::Path>,
    text_bbox: Option<tiny_skia::NonZeroRect>,
    anti_alias: bool,
) -> Option<(FillPath, usvg::BBox)> {
    // Horizontal and vertical lines cannot be filled. Skip.
    if path.bounds().width() == 0.0 || path.bounds().height() == 0.0 {
        return None;
    }

    let rule = match ufill.rule {
        usvg::FillRule::NonZero => tiny_skia::FillRule::Winding,
        usvg::FillRule::EvenOdd => tiny_skia::FillRule::EvenOdd,
    };

    let mut object_bbox = usvg::BBox::from(path.compute_tight_bounds()?);
    if let Some(text_bbox) = text_bbox {
        object_bbox = object_bbox.expand(usvg::BBox::from(text_bbox));
    }

    let paint =
        crate::paint_server::convert(&ufill.paint, ufill.opacity, object_bbox.to_non_zero_rect())?;

    let path = FillPath {
        paint,
        rule,
        anti_alias,
        path,
    };

    Some((path, object_bbox))
}

fn convert_stroke_path(
    ustroke: &usvg::Stroke,
    path: Rc<tiny_skia::Path>,
    text_bbox: Option<tiny_skia::NonZeroRect>,
    anti_alias: bool,
) -> Option<(StrokePath, usvg::BBox, usvg::BBox)> {
    let mut stroke = tiny_skia::Stroke {
        width: ustroke.width.get(),
        miter_limit: ustroke.miterlimit.get(),
        line_cap: match ustroke.linecap {
            usvg::LineCap::Butt => tiny_skia::LineCap::Butt,
            usvg::LineCap::Round => tiny_skia::LineCap::Round,
            usvg::LineCap::Square => tiny_skia::LineCap::Square,
        },
        line_join: match ustroke.linejoin {
            usvg::LineJoin::Miter => tiny_skia::LineJoin::Miter,
            usvg::LineJoin::MiterClip => tiny_skia::LineJoin::MiterClip,
            usvg::LineJoin::Round => tiny_skia::LineJoin::Round,
            usvg::LineJoin::Bevel => tiny_skia::LineJoin::Bevel,
        },
        dash: None,
    };

    // Zero-sized stroke path is not an error, because linecap round or square
    // would produce the shape either way.
    // TODO: Find a better way to handle it.
    let object_bbox = usvg::BBox::from(path.compute_tight_bounds()?);

    let mut complete_object_bbox = object_bbox;
    if let Some(text_bbox) = text_bbox {
        complete_object_bbox = complete_object_bbox.expand(usvg::BBox::from(text_bbox));
    }
    let paint = crate::paint_server::convert(
        &ustroke.paint,
        ustroke.opacity,
        complete_object_bbox.to_non_zero_rect(),
    )?;

    if let Some(ref list) = ustroke.dasharray {
        stroke.dash = tiny_skia::StrokeDash::new(list.clone(), ustroke.dashoffset);
    }

    // TODO: explain
    // TODO: expand by stroke width for round/bevel joins
    let stroked_path = path.stroke(&stroke, 1.0)?;

    let mut layer_bbox = usvg::BBox::from(stroked_path.compute_tight_bounds()?);
    if let Some(text_bbox) = text_bbox {
        layer_bbox = layer_bbox.expand(usvg::BBox::from(text_bbox));
    }

    // TODO: dash beforehand
    // TODO: preserve stroked path

    let path = StrokePath {
        paint,
        stroke,
        anti_alias,
        path,
    };

    Some((path, layer_bbox, object_bbox))
}

pub fn render_fill_path(
    path: &FillPath,
    blend_mode: tiny_skia::BlendMode,
    ctx: &Context,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) -> Option<()> {
    let pattern_pixmap;
    let mut paint = tiny_skia::Paint::default();
    match path.paint {
        Paint::Shader(ref shader) => {
            paint.shader = shader.clone(); // TODO: avoid clone
        }
        Paint::Pattern(ref pattern) => {
            let (patt_pix, patt_ts) =
                crate::paint_server::prepare_pattern_pixmap(pattern, ctx, transform)?;

            pattern_pixmap = patt_pix;
            paint.shader = tiny_skia::Pattern::new(
                pattern_pixmap.as_ref(),
                tiny_skia::SpreadMode::Repeat,
                tiny_skia::FilterQuality::Bicubic,
                pattern.opacity.get(),
                patt_ts,
            )
        }
    }

    paint.anti_alias = path.anti_alias;
    paint.blend_mode = blend_mode;

    pixmap.fill_path(&path.path, &paint, path.rule, transform, None);

    Some(())
}

pub fn render_stroke_path(
    path: &StrokePath,
    blend_mode: tiny_skia::BlendMode,
    ctx: &Context,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) -> Option<()> {
    let pattern_pixmap;
    let mut paint = tiny_skia::Paint::default();
    match path.paint {
        Paint::Shader(ref shader) => {
            paint.shader = shader.clone(); // TODO: avoid clone
        }
        Paint::Pattern(ref pattern) => {
            let (patt_pix, patt_ts) =
                crate::paint_server::prepare_pattern_pixmap(pattern, ctx, transform)?;

            pattern_pixmap = patt_pix;
            paint.shader = tiny_skia::Pattern::new(
                pattern_pixmap.as_ref(),
                tiny_skia::SpreadMode::Repeat,
                tiny_skia::FilterQuality::Bicubic,
                pattern.opacity.get(),
                patt_ts,
            )
        }
    }

    paint.anti_alias = path.anti_alias;
    paint.blend_mode = blend_mode;

    // TODO: fallback to a stroked path when possible

    pixmap.stroke_path(&path.path, &paint, &path.stroke, transform, None);

    Some(())
}
