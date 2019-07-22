// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::io::Write;
use std::ops::Deref;

use svgdom::WriteBuffer;
use xmlwriter::XmlWriter;

use super::*;
use crate::{geom::*, IsDefault};


pub fn convert(tree: &Tree, opt: XmlOptions) -> String {
    let mut xml = XmlWriter::new(opt);

    let svg_node = tree.svg_node();

    xml.start_element("svg");
    xml.write_attribute("width", &svg_node.size.width());
    xml.write_attribute("height", &svg_node.size.height());
    xml.write_viewbox(&svg_node.view_box);
    xml.write_attribute("xmlns", "http://www.w3.org/2000/svg");
    if has_xlink(tree) {
        xml.write_attribute("xmlns:xlink", "http://www.w3.org/1999/xlink");
    }
    xml.write_attribute("xmlns:usvg", "https://github.com/RazrFalcon/usvg");
    xml.write_attribute("usvg:version", env!("CARGO_PKG_VERSION"));

    xml.start_element("defs");
    conv_defs(tree, &mut xml);
    xml.end_element();

    conv_elements(&tree.root(), false, &mut xml);

    xml.end_document()
}

fn conv_defs(
    tree: &Tree,
    xml: &mut XmlWriter,
) {
    for n in tree.defs().children() {
        match *n.borrow() {
            NodeKind::LinearGradient(ref lg) => {
                xml.start_element("linearGradient");
                xml.write_attribute("id", &lg.id);
                xml.write_attribute("x1", &lg.x1);
                xml.write_attribute("y1", &lg.y1);
                xml.write_attribute("x2", &lg.x2);
                xml.write_attribute("y2", &lg.y2);
                write_base_grad(&lg.base, xml);
                xml.end_element();
            }
            NodeKind::RadialGradient(ref rg) => {
                xml.start_element("radialGradient");
                xml.write_attribute("id", &rg.id);
                xml.write_attribute("cx", &rg.cx);
                xml.write_attribute("cy", &rg.cy);
                xml.write_attribute("r",  &rg.r.value());
                xml.write_attribute("fx", &rg.fx);
                xml.write_attribute("fy", &rg.fy);
                write_base_grad(&rg.base, xml);
                xml.end_element();
            }
            NodeKind::ClipPath(ref clip) => {
                xml.start_element("clipPath");
                xml.write_attribute("id", &clip.id);
                xml.write_units("clipPathUnits", clip.units, Units::UserSpaceOnUse);
                xml.write_transform("transform", clip.transform);

                if let Some(ref id) = clip.clip_path {
                    xml.write_func_iri("clip-path", id);
                }

                conv_elements(&n, true, xml);

                xml.end_element();
            }
            NodeKind::Mask(ref mask) => {
                xml.start_element("mask");
                xml.write_attribute("id", &mask.id);
                xml.write_units("maskUnits", mask.units, Units::ObjectBoundingBox);
                xml.write_units("maskContentUnits", mask.content_units, Units::UserSpaceOnUse);
                xml.write_rect_attrs(mask.rect);

                if let Some(ref id) = mask.mask {
                    xml.write_func_iri("mask", id);
                }

                conv_elements(&n, false, xml);

                xml.end_element();
            }
            NodeKind::Pattern(ref pattern) => {
                xml.start_element("pattern");
                xml.write_attribute("id", &pattern.id);
                xml.write_rect_attrs(pattern.rect);
                xml.write_units("patternUnits", pattern.units, Units::ObjectBoundingBox);
                xml.write_units("patternContentUnits", pattern.content_units, Units::UserSpaceOnUse);
                xml.write_transform("patternTransform", pattern.transform);

                if let Some(ref vbox) = pattern.view_box {
                    xml.write_viewbox(vbox);
                }

                conv_elements(&n, false, xml);

                xml.end_element();
            }
            NodeKind::Filter(ref filter) => {
                xml.start_element("filter");
                xml.write_attribute("id", &filter.id);
                xml.write_rect_attrs(filter.rect);
                xml.write_units("filterUnits", filter.units, Units::ObjectBoundingBox);
                xml.write_units("primitiveUnits", filter.primitive_units, Units::UserSpaceOnUse);

                for fe in &filter.children {
                    match fe.kind {
                        FilterKind::FeGaussianBlur(ref blur) => {
                            xml.start_element("feGaussianBlur");
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_filter_input("in", &blur.input);
                            xml.write_attribute_fmt(
                                "stdDeviation",
                                format_args!("{} {}", blur.std_dev_x.value(), blur.std_dev_y.value()),
                            );
                            xml.write_attribute("result", &fe.result);
                            xml.end_element();
                        }
                        FilterKind::FeOffset(ref offset) => {
                            xml.start_element("feOffset");
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_filter_input("in", &offset.input);
                            xml.write_attribute("dx", &offset.dx);
                            xml.write_attribute("dy", &offset.dy);
                            xml.write_attribute("result", &fe.result);
                            xml.end_element();
                        }
                        FilterKind::FeBlend(ref blend) => {
                            xml.start_element("feBlend");
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_filter_input("in", &blend.input1);
                            xml.write_filter_input("in2", &blend.input2);
                            xml.write_attribute("mode", match blend.mode {
                                FeBlendMode::Normal     => "normal",
                                FeBlendMode::Multiply   => "multiply",
                                FeBlendMode::Screen     => "screen",
                                FeBlendMode::Darken     => "darken",
                                FeBlendMode::Lighten    => "lighten",
                            });
                            xml.write_attribute("result", &fe.result);
                            xml.end_element();
                        }
                        FilterKind::FeFlood(ref flood) => {
                            xml.start_element("feFlood");
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_attribute("flood-color", &flood.color);
                            xml.write_attribute("flood-opacity", &flood.opacity.value());
                            xml.write_attribute("result", &fe.result);
                            xml.end_element();
                        }
                        FilterKind::FeComposite(ref composite) => {
                            xml.start_element("feComposite");
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_filter_input("in", &composite.input1);
                            xml.write_filter_input("in2", &composite.input2);
                            xml.write_attribute("operator", match composite.operator {
                                FeCompositeOperator::Over               => "over",
                                FeCompositeOperator::In                 => "in",
                                FeCompositeOperator::Out                => "out",
                                FeCompositeOperator::Atop               => "atop",
                                FeCompositeOperator::Xor                => "xor",
                                FeCompositeOperator::Arithmetic { .. }  => "arithmetic",
                            });

                            match composite.operator {
                                FeCompositeOperator::Arithmetic { k1, k2, k3, k4 } => {
                                    xml.write_attribute("k1", &k1.value());
                                    xml.write_attribute("k2", &k2.value());
                                    xml.write_attribute("k3", &k3.value());
                                    xml.write_attribute("k4", &k4.value());
                                }
                                _ => {}
                            }

                            xml.write_attribute("result", &fe.result);
                            xml.end_element();
                        }
                        FilterKind::FeMerge(ref merge) => {
                            xml.start_element("feMerge");
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_attribute("result", &fe.result);
                            for input in &merge.inputs {
                                xml.start_element("feMergeNode");
                                xml.write_filter_input("in", &input);
                                xml.end_element();
                            }

                            xml.end_element();
                        }
                        FilterKind::FeTile(ref tile) => {
                            xml.start_element("feTile");
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_filter_input("in", &tile.input);
                            xml.write_attribute("result", &fe.result);
                            xml.end_element();
                        }
                        FilterKind::FeImage(ref img) => {
                            xml.start_element("feImage");
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_aspect(img.aspect);
                            xml.write_attribute("imageRendering", match img.rendering_mode {
                                ImageRendering::OptimizeQuality => "optimizeQuality",
                                ImageRendering::OptimizeSpeed   => "optimizeSpeed",
                            });
                            match img.data {
                                FeImageKind::None => {}
                                FeImageKind::Image(ref data, format) => {
                                    xml.write_image_data(data, format);
                                }
                                FeImageKind::Use(..) => {}
                            }

                            xml.write_attribute("result", &fe.result);
                            xml.end_element();
                        }
                    };
                }

                xml.end_element();
            }
            _ => {}
        }
    }
}

fn conv_elements(
    parent: &Node,
    is_clip_path: bool,
    xml: &mut XmlWriter,
) {
    for n in parent.children() {
        match *n.borrow() {
            NodeKind::Path(ref p) => {
                write_path(p, is_clip_path, None, xml);
            }
            NodeKind::Image(ref img) => {
                xml.start_element("image");
                if !img.id.is_empty() {
                    xml.write_attribute("id", &img.id);
                }

                xml.write_rect_attrs(img.view_box.rect);
                if !img.view_box.aspect.is_default() {
                    xml.write_aspect(img.view_box.aspect);
                }

                xml.write_visibility(img.visibility);

                match img.rendering_mode {
                    ImageRendering::OptimizeQuality => {}
                    ImageRendering::OptimizeSpeed => {
                        xml.write_attribute("image-rendering", "optimizeSpeed");
                    }
                }

                xml.write_transform("transform", img.transform);
                xml.write_image_data(&img.data, img.format);

                xml.end_element();
            }
            NodeKind::Group(ref g) => {
                if is_clip_path {
                    // ClipPath with a Group element is an `usvg` special case.
                    // Group will contains a single Path element and we should set
                    // `clip-path` on it.

                    if let NodeKind::Path(ref path) = *n.first_child().unwrap().borrow() {
                        let clip_id = g.clip_path.as_ref().map(String::deref);
                        write_path(path, is_clip_path, clip_id, xml);
                    }

                    continue;
                }

                xml.start_element("g");
                if !g.id.is_empty() {
                    xml.write_attribute("id", &g.id);
                };

                if let Some(ref id) = g.clip_path {
                    xml.write_func_iri("clip-path", id);
                }

                if let Some(ref id) = g.mask {
                    xml.write_func_iri("mask", id);
                }

                if let Some(ref id) = g.filter {
                    xml.write_func_iri("filter", id);
                }

                if !g.opacity.is_default() {
                    xml.write_attribute("opacity", &g.opacity.value());
                }

                xml.write_transform("transform", g.transform);

                conv_elements(&n, false, xml);

                xml.end_element();
            }
            _ => {}
        }
    }
}

trait XmlWriterExt {
    fn write_viewbox(&mut self, view_box: &ViewBox);
    fn write_aspect(&mut self, aspect: AspectRatio);
    fn write_units(&mut self, name: &str, units: Units, def: Units);
    fn write_transform(&mut self, name: &str, units: Transform);
    fn write_visibility(&mut self, value: Visibility);
    fn write_func_iri(&mut self, name: &str, id: &str);
    fn write_rect_attrs(&mut self, r: Rect);
    fn write_filter_input(&mut self, name: &str, input: &FilterInput);
    fn write_filter_primitive_attrs(&mut self, fe: &FilterPrimitive);
    fn write_image_data(&mut self, data: &ImageData, format: ImageFormat);
}

impl XmlWriterExt for XmlWriter {
    fn write_viewbox(&mut self, view_box: &ViewBox) {
        let r = view_box.rect;
        self.write_attribute_fmt(
            "viewBox",
            format_args!("{} {} {} {}", r.x(), r.y(), r.width(), r.height()),
        );

        if !view_box.aspect.is_default() {
            self.write_aspect(view_box.aspect);
        }
    }

    fn write_aspect(&mut self, aspect: AspectRatio) {
        self.write_attribute_raw("preserveAspectRatio", |buf| aspect.write_buf(buf));
    }

    fn write_units(&mut self, name: &str, units: Units, def: Units) {
        if units != def {
            self.write_attribute(name, match units {
                Units::UserSpaceOnUse => "userSpaceOnUse",
                Units::ObjectBoundingBox => "objectBoundingBox",
            });
        }
    }

    fn write_transform(&mut self, name: &str, ts: Transform) {
        if !ts.is_default() {
            self.write_attribute_fmt(
                name,
                format_args!("matrix({} {} {} {} {} {})", ts.a, ts.b, ts.c, ts.d, ts.e, ts.f),
            );
        }
    }

    fn write_visibility(&mut self, value: Visibility) {
        match value {
            Visibility::Visible => {},
            Visibility::Hidden => self.write_attribute("visibility", "hidden"),
            Visibility::Collapse => self.write_attribute("visibility", "collapse"),
        }
    }

    fn write_func_iri(&mut self, name: &str, id: &str) {
        self.write_attribute_fmt(name, format_args!("url(#{})", id));
    }

    fn write_rect_attrs(&mut self, r: Rect) {
        self.write_attribute("x", &r.x());
        self.write_attribute("y", &r.y());
        self.write_attribute("width", &r.width());
        self.write_attribute("height", &r.height());
    }

    fn write_filter_input(&mut self, name: &str, input: &FilterInput) {
        self.write_attribute(name, match input {
            FilterInput::SourceGraphic      => "SourceGraphic",
            FilterInput::SourceAlpha        => "SourceAlpha",
            FilterInput::BackgroundImage    => "BackgroundImage",
            FilterInput::BackgroundAlpha    => "BackgroundAlpha",
            FilterInput::FillPaint          => "FillPaint",
            FilterInput::StrokePaint        => "StrokePaint",
            FilterInput::Reference(ref s)   => s,
        });
    }

    fn write_filter_primitive_attrs(&mut self, fe: &FilterPrimitive) {
        if let Some(n) = fe.x { self.write_attribute("x", &n); }
        if let Some(n) = fe.y { self.write_attribute("y", &n); }
        if let Some(n) = fe.width { self.write_attribute("width", &n); }
        if let Some(n) = fe.height { self.write_attribute("height", &n); }

        self.write_attribute("colorInterpolationFilters", match fe.color_interpolation {
            ColorInterpolation::SRGB        => "sRGB",
            ColorInterpolation::LinearRGB   => "linearRGB"
        });
    }

    fn write_image_data(&mut self, data: &ImageData, format: ImageFormat) {
        match data {
            ImageData::Path(ref path) => {
                self.write_attribute("xlink:href", &path.to_str().unwrap());
            }
            ImageData::Raw(ref data) => {
                self.write_attribute_raw("xlink:href", |buf| {
                    buf.extend_from_slice(b"data:image/");
                    buf.extend_from_slice(match format {
                        ImageFormat::PNG => b"png",
                        ImageFormat::JPEG => b"jpg",
                        ImageFormat::SVG => b"svg+xml",
                    });
                    buf.extend_from_slice(b";base64, ");

                    let mut enc = base64::write::EncoderWriter::new(buf, base64::STANDARD);
                    enc.write_all(data).unwrap();
                    enc.finish().unwrap();
                });
            }
        }
    }
}

fn has_xlink(tree: &Tree) -> bool {
    for n in tree.root().descendants() {
        match *n.borrow() {
            NodeKind::Filter(ref filter) => {
                for fe in &filter.children {
                    if let FilterKind::FeImage(ref img) = fe.kind {
                        if let FeImageKind::Image(..) = img.data {
                            return true;
                        }
                    }
                }
            }
            NodeKind::Image(_) => {
                return true;
            }
            _ => {}
        }
    }

    false
}

fn write_base_grad(
    g: &BaseGradient,
    xml: &mut XmlWriter,
) {
    xml.write_units("gradientUnits", g.units, Units::ObjectBoundingBox);
    xml.write_transform("gradientTransform", g.transform);

    match g.spread_method {
        SpreadMethod::Pad => {},
        SpreadMethod::Reflect => xml.write_attribute("spreadMethod", "reflect"),
        SpreadMethod::Repeat => xml.write_attribute("spreadMethod", "repeat"),
    }

    for s in &g.stops {
        xml.start_element("stop");
        xml.write_attribute("offset", &s.offset.value());
        xml.write_attribute("stop-color", &s.color);
        if !s.opacity.is_default() {
            xml.write_attribute("stop-opacity", &s.opacity.value());
        }

        xml.end_element();
    }
}

fn write_path(
    path: &Path,
    is_clip_path: bool,
    clip_path: Option<&str>,
    xml: &mut XmlWriter,
) {
    xml.start_element("path");
    if !path.id.is_empty() {
        xml.write_attribute("id", &path.id);
    }

    write_fill(&path.fill, is_clip_path, xml);
    write_stroke(&path.stroke, xml);

    xml.write_visibility(path.visibility);

    match path.rendering_mode {
        ShapeRendering::OptimizeSpeed => {
            xml.write_attribute("shape-rendering", "optimizeSpeed");
        }
        ShapeRendering::CrispEdges => {
            xml.write_attribute("shape-rendering", "crispEdges")
        }
        ShapeRendering::GeometricPrecision  => {}
    }

    if let Some(ref id) = clip_path {
        xml.write_func_iri("clip-path", id);
    }

    xml.write_transform("transform", path.transform);

    xml.write_attribute_raw("d", |buf| {
        for seg in &path.segments {
            match *seg {
                PathSegment::MoveTo { x, y } => {
                    buf.extend_from_slice(b"M ");
                    x.write_buf(buf);
                    buf.push(b' ');
                    y.write_buf(buf);
                    buf.push(b' ');
                }
                PathSegment::LineTo { x, y } => {
                    buf.extend_from_slice(b"L ");
                    x.write_buf(buf);
                    buf.push(b' ');
                    y.write_buf(buf);
                    buf.push(b' ');
                }
                PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                    buf.extend_from_slice(b"C ");
                    x1.write_buf(buf);
                    buf.push(b' ');
                    y1.write_buf(buf);
                    buf.push(b' ');
                    x2.write_buf(buf);
                    buf.push(b' ');
                    y2.write_buf(buf);
                    buf.push(b' ');
                    x.write_buf(buf);
                    buf.push(b' ');
                    y.write_buf(buf);
                    buf.push(b' ');
                }
                PathSegment::ClosePath => {
                    buf.extend_from_slice(b"Z ");
                }
            }
        }

        if !path.segments.is_empty() {
            buf.pop();
        }
    });

    xml.end_element();
}

fn write_fill(
    fill: &Option<Fill>,
    is_clip_path: bool,
    xml: &mut XmlWriter,
) {
    match fill {
        Some(ref fill) => {
            match fill.paint {
                Paint::Color(c) => {
                    if c != Color::black() {
                        xml.write_attribute("fill", &c);
                    }
                }
                Paint::Link(ref id) => {
                    xml.write_func_iri("fill", id);
                }
            }

            if !fill.opacity.is_default() {
                xml.write_attribute("fill-opacity", &fill.opacity.value());
            }

            if !fill.rule.is_default() {
                let name = if is_clip_path {
                    "clip-rule"
                } else {
                    "fill-rule"
                };

                xml.write_attribute(name, "evenodd");
            }
        }
        None => {
            xml.write_attribute("fill", "none");
        }
    }
}

fn write_stroke(
    stroke: &Option<Stroke>,
    xml: &mut XmlWriter,
) {
    if let Some(ref stroke) = stroke {
        match stroke.paint {
            Paint::Color(ref c) => xml.write_attribute("stroke", c),
            Paint::Link(ref id) => xml.write_func_iri("stroke", id),
        }

        if !stroke.opacity.is_default() {
            xml.write_attribute("stroke-opacity", &stroke.opacity.value());
        }

        if !(stroke.dashoffset as f64).is_fuzzy_zero() {
            xml.write_attribute("stroke-dashoffset", &stroke.dashoffset)
        }

        if !stroke.miterlimit.is_default() {
            xml.write_attribute("stroke-miterlimit", &stroke.miterlimit.value());
        }

        if !stroke.width.is_default() {
            xml.write_attribute("stroke-width", &stroke.width.value());
        }

        match stroke.linecap {
            LineCap::Butt => {}
            LineCap::Round => xml.write_attribute("stroke-linecap", "round"),
            LineCap::Square => xml.write_attribute("stroke-linecap", "square"),
        }

        match stroke.linejoin {
            LineJoin::Miter => {}
            LineJoin::Round => xml.write_attribute("stroke-linejoin", "round"),
            LineJoin::Bevel => xml.write_attribute("stroke-linejoin", "bevel"),
        }

        if let Some(ref array) = stroke.dasharray {
            xml.write_attribute_raw("stroke-dasharray", |buf| {
                for n in array {
                    buf.write_fmt(format_args!("{} ", n)).unwrap();
                }

                if !array.is_empty() {
                    buf.pop();
                }
            });
        }
    }
}
