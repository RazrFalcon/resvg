// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::rc::Rc;

use crate::paint_server::Paint;
use crate::render::Context;
use crate::tree::{BBoxes, ConvTransform, Node, TinySkiaRectExt};

pub struct FillPath {
    pub transform: tiny_skia::Transform,
    pub paint: Paint,
    pub rule: tiny_skia::FillRule,
    pub anti_alias: bool,
    pub path: Rc<tiny_skia::Path>,
}

pub struct StrokePath {
    pub transform: tiny_skia::Transform,
    pub paint: Paint,
    pub stroke: tiny_skia::Stroke,
    pub anti_alias: bool,
    pub path: Rc<tiny_skia::Path>,
}

pub fn convert(upath: &usvg::Path, children: &mut Vec<Node>) -> Option<BBoxes> {
    let transform = upath.transform.to_native();
    let anti_alias = upath.rendering_mode.use_shape_antialiasing();
    let path = match convert_path_data(&upath.data) {
        Some(v) => Rc::new(v),
        None => return None,
    };

    let fill_path = upath.fill.as_ref().and_then(|ufill| {
        convert_fill_path(ufill, path.clone(), transform, upath.text_bbox, anti_alias)
    });

    let stroke_path = upath.stroke.as_ref().and_then(|ustroke| {
        convert_stroke_path(
            ustroke,
            path.clone(),
            transform,
            upath.text_bbox,
            anti_alias,
        )
    });

    if fill_path.is_none() && stroke_path.is_none() {
        return None;
    }

    let mut bboxes = BBoxes::default();

    if let Some((_, l_bbox, o_bbox)) = fill_path {
        bboxes.layer = bboxes.layer.expand(l_bbox);
        bboxes.object = bboxes.object.expand(o_bbox);
    }
    if let Some((_, l_bbox, o_bbox)) = stroke_path {
        bboxes.layer = bboxes.layer.expand(l_bbox);
        bboxes.object = bboxes.object.expand(o_bbox);
    }

    bboxes.transformed_object = bboxes.object.transform(&upath.transform)?;

    // Do not add hidden paths, but preserve the bbox.
    // visibility=hidden still affects the bbox calculation.
    if upath.visibility != usvg::Visibility::Visible {
        return Some(bboxes);
    }

    if upath.paint_order == usvg::PaintOrder::FillAndStroke {
        if let Some((path, _, _)) = fill_path {
            children.push(Node::FillPath(path));
        }

        if let Some((path, _, _)) = stroke_path {
            children.push(Node::StrokePath(path));
        }
    } else {
        if let Some((path, _, _)) = stroke_path {
            children.push(Node::StrokePath(path));
        }

        if let Some((path, _, _)) = fill_path {
            children.push(Node::FillPath(path));
        }
    }

    Some(bboxes)
}

fn convert_fill_path(
    ufill: &usvg::Fill,
    path: Rc<tiny_skia::Path>,
    transform: tiny_skia::Transform,
    text_bbox: Option<usvg::Rect>,
    anti_alias: bool,
) -> Option<(FillPath, usvg::PathBbox, usvg::PathBbox)> {
    // Horizontal and vertical lines cannot be filled. Skip.
    if path.bounds().width() == 0.0 || path.bounds().height() == 0.0 {
        return None;
    }

    let paint = crate::paint_server::convert(&ufill.paint, ufill.opacity, path.bounds())?;

    let rule = match ufill.rule {
        usvg::FillRule::NonZero => tiny_skia::FillRule::Winding,
        usvg::FillRule::EvenOdd => tiny_skia::FillRule::EvenOdd,
    };

    let mut object_bbox = path.bounds().to_path_bbox()?;
    if let Some(text_bbox) = text_bbox {
        object_bbox = object_bbox.expand(text_bbox.to_path_bbox());
    }

    let path = FillPath {
        transform,
        paint,
        rule,
        anti_alias,
        path,
    };

    Some((path, object_bbox, object_bbox))
}

fn convert_stroke_path(
    ustroke: &usvg::Stroke,
    path: Rc<tiny_skia::Path>,
    transform: tiny_skia::Transform,
    text_bbox: Option<usvg::Rect>,
    anti_alias: bool,
) -> Option<(StrokePath, usvg::PathBbox, usvg::PathBbox)> {
    let paint = crate::paint_server::convert(&ustroke.paint, ustroke.opacity, path.bounds())?;

    let mut stroke = tiny_skia::Stroke {
        width: ustroke.width.get() as f32,
        miter_limit: ustroke.miterlimit.get() as f32,
        line_cap: match ustroke.linecap {
            usvg::LineCap::Butt => tiny_skia::LineCap::Butt,
            usvg::LineCap::Round => tiny_skia::LineCap::Round,
            usvg::LineCap::Square => tiny_skia::LineCap::Square,
        },
        line_join: match ustroke.linejoin {
            usvg::LineJoin::Miter => tiny_skia::LineJoin::Miter,
            usvg::LineJoin::Round => tiny_skia::LineJoin::Round,
            usvg::LineJoin::Bevel => tiny_skia::LineJoin::Bevel,
        },
        dash: None,
    };

    // Zero-sized stroke path is not an error, because linecap round or square
    // would produce the shape either way.
    // TODO: Find a better way to handle it.
    let object_bbox = path
        .bounds()
        .to_path_bbox()
        .unwrap_or_else(|| usvg::PathBbox::new(0.0, 0.0, 1.0, 1.0).unwrap());

    if let Some(ref list) = ustroke.dasharray {
        let list: Vec<_> = list.iter().map(|n| *n as f32).collect();
        stroke.dash = tiny_skia::StrokeDash::new(list, ustroke.dashoffset);
    }

    // TODO: explain
    // TODO: expand by stroke width for round/bevel joins
    let resolution_scale = tiny_skia::PathStroker::compute_resolution_scale(&transform);
    let resolution_scale = resolution_scale.max(10.0);
    let stroked_path = path.stroke(&stroke, resolution_scale)?;

    let mut layer_bbox = stroked_path.bounds().to_path_bbox()?;
    if let Some(text_bbox) = text_bbox {
        layer_bbox = layer_bbox.expand(text_bbox.to_path_bbox());
    }

    // TODO: dash beforehand
    // TODO: preserve stroked path

    let path = StrokePath {
        transform,
        paint,
        stroke: stroke,
        anti_alias,
        path,
    };

    Some((path, layer_bbox, object_bbox))
}

fn convert_path_data(path: &usvg::PathData) -> Option<tiny_skia::Path> {
    let mut pb = tiny_skia::PathBuilder::new();
    for seg in path.segments() {
        match seg {
            usvg::PathSegment::MoveTo { x, y } => {
                pb.move_to(x as f32, y as f32);
            }
            usvg::PathSegment::LineTo { x, y } => {
                pb.line_to(x as f32, y as f32);
            }
            #[rustfmt::skip]
            usvg::PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                pb.cubic_to(x1 as f32, y1 as f32, x2 as f32, y2 as f32, x as f32, y as f32);
            }
            usvg::PathSegment::ClosePath => {
                pb.close();
            }
        }
    }

    pb.finish()
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
                pattern.opacity.get() as f32,
                patt_ts,
            )
        }
    }

    paint.anti_alias = path.anti_alias;
    paint.blend_mode = blend_mode;

    let transform = transform.pre_concat(path.transform);
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
                pattern.opacity.get() as f32,
                patt_ts,
            )
        }
    }

    paint.anti_alias = path.anti_alias;
    paint.blend_mode = blend_mode;

    // TODO: fallback to a stroked path when possible

    let transform = transform.pre_concat(path.transform);
    pixmap.stroke_path(&path.path, &paint, &path.stroke, transform, None);

    Some(())
}
