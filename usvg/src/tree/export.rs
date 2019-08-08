// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt::Display;
use std::io::Write;
use std::ops::Deref;

use svgdom::WriteBuffer;
use xmlwriter::XmlWriter;

use super::*;
use crate::{geom::*, short::*, IsDefault};


pub fn convert(tree: &Tree, opt: XmlOptions) -> String {
    let mut xml = XmlWriter::new(opt);

    let svg_node = tree.svg_node();

    xml.start_svg_element(EId::Svg);
    xml.write_svg_attribute(AId::Width, &svg_node.size.width());
    xml.write_svg_attribute(AId::Height, &svg_node.size.height());
    xml.write_viewbox(&svg_node.view_box);
    xml.write_attribute("xmlns", "http://www.w3.org/2000/svg");
    if has_xlink(tree) {
        xml.write_attribute("xmlns:xlink", "http://www.w3.org/1999/xlink");
    }
    xml.write_attribute("xmlns:usvg", "https://github.com/RazrFalcon/usvg");
    xml.write_attribute("usvg:version", env!("CARGO_PKG_VERSION"));

    xml.start_svg_element(EId::Defs);
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
                xml.start_svg_element(EId::LinearGradient);
                xml.write_svg_attribute(AId::Id, &lg.id);
                xml.write_svg_attribute(AId::X1, &lg.x1);
                xml.write_svg_attribute(AId::Y1, &lg.y1);
                xml.write_svg_attribute(AId::X2, &lg.x2);
                xml.write_svg_attribute(AId::Y2, &lg.y2);
                write_base_grad(&lg.base, xml);
                xml.end_element();
            }
            NodeKind::RadialGradient(ref rg) => {
                xml.start_svg_element(EId::RadialGradient);
                xml.write_svg_attribute(AId::Id, &rg.id);
                xml.write_svg_attribute(AId::Cx, &rg.cx);
                xml.write_svg_attribute(AId::Cy, &rg.cy);
                xml.write_svg_attribute(AId::R,  &rg.r.value());
                xml.write_svg_attribute(AId::Fx, &rg.fx);
                xml.write_svg_attribute(AId::Fy, &rg.fy);
                write_base_grad(&rg.base, xml);
                xml.end_element();
            }
            NodeKind::ClipPath(ref clip) => {
                xml.start_svg_element(EId::ClipPath);
                xml.write_svg_attribute(AId::Id, &clip.id);
                xml.write_units(AId::ClipPathUnits, clip.units, Units::UserSpaceOnUse);
                xml.write_transform(AId::Transform, clip.transform);

                if let Some(ref id) = clip.clip_path {
                    xml.write_func_iri(AId::ClipPath, id);
                }

                conv_elements(&n, true, xml);

                xml.end_element();
            }
            NodeKind::Mask(ref mask) => {
                xml.start_svg_element(EId::Mask);
                xml.write_svg_attribute(AId::Id, &mask.id);
                xml.write_units(AId::MaskUnits, mask.units, Units::ObjectBoundingBox);
                xml.write_units(AId::MaskContentUnits, mask.content_units, Units::UserSpaceOnUse);
                xml.write_rect_attrs(mask.rect);

                if let Some(ref id) = mask.mask {
                    xml.write_func_iri(AId::Mask, id);
                }

                conv_elements(&n, false, xml);

                xml.end_element();
            }
            NodeKind::Pattern(ref pattern) => {
                xml.start_svg_element(EId::Pattern);
                xml.write_svg_attribute(AId::Id, &pattern.id);
                xml.write_rect_attrs(pattern.rect);
                xml.write_units(AId::PatternUnits, pattern.units, Units::ObjectBoundingBox);
                xml.write_units(AId::PatternContentUnits, pattern.content_units, Units::UserSpaceOnUse);
                xml.write_transform(AId::PatternTransform, pattern.transform);

                if let Some(ref vbox) = pattern.view_box {
                    xml.write_viewbox(vbox);
                }

                conv_elements(&n, false, xml);

                xml.end_element();
            }
            NodeKind::Filter(ref filter) => {
                xml.start_svg_element(EId::Filter);
                xml.write_svg_attribute(AId::Id, &filter.id);
                xml.write_rect_attrs(filter.rect);
                xml.write_units(AId::FilterUnits, filter.units, Units::ObjectBoundingBox);
                xml.write_units(AId::PrimitiveUnits, filter.primitive_units, Units::UserSpaceOnUse);

                for fe in &filter.children {
                    match fe.kind {
                        FilterKind::FeGaussianBlur(ref blur) => {
                            xml.start_svg_element(EId::FeGaussianBlur);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_filter_input(AId::In, &blur.input);
                            xml.write_attribute_fmt(
                                AId::StdDeviation.as_str(),
                                format_args!("{} {}", blur.std_dev_x.value(), blur.std_dev_y.value()),
                            );
                            xml.write_svg_attribute(AId::Result, &fe.result);
                            xml.end_element();
                        }
                        FilterKind::FeOffset(ref offset) => {
                            xml.start_svg_element(EId::FeOffset);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_filter_input(AId::In, &offset.input);
                            xml.write_svg_attribute(AId::Dx, &offset.dx);
                            xml.write_svg_attribute(AId::Dy, &offset.dy);
                            xml.write_svg_attribute(AId::Result, &fe.result);
                            xml.end_element();
                        }
                        FilterKind::FeBlend(ref blend) => {
                            xml.start_svg_element(EId::FeBlend);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_filter_input(AId::In, &blend.input1);
                            xml.write_filter_input(AId::In2, &blend.input2);
                            xml.write_svg_attribute(AId::Mode, match blend.mode {
                                FeBlendMode::Normal     => "normal",
                                FeBlendMode::Multiply   => "multiply",
                                FeBlendMode::Screen     => "screen",
                                FeBlendMode::Darken     => "darken",
                                FeBlendMode::Lighten    => "lighten",
                            });
                            xml.write_svg_attribute(AId::Result, &fe.result);
                            xml.end_element();
                        }
                        FilterKind::FeFlood(ref flood) => {
                            xml.start_svg_element(EId::FeFlood);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_svg_attribute(AId::FloodColor, &flood.color);
                            xml.write_svg_attribute(AId::FloodOpacity, &flood.opacity.value());
                            xml.write_svg_attribute(AId::Result, &fe.result);
                            xml.end_element();
                        }
                        FilterKind::FeComposite(ref composite) => {
                            xml.start_svg_element(EId::FeComposite);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_filter_input(AId::In, &composite.input1);
                            xml.write_filter_input(AId::In2, &composite.input2);
                            xml.write_svg_attribute(AId::Operator, match composite.operator {
                                FeCompositeOperator::Over               => "over",
                                FeCompositeOperator::In                 => "in",
                                FeCompositeOperator::Out                => "out",
                                FeCompositeOperator::Atop               => "atop",
                                FeCompositeOperator::Xor                => "xor",
                                FeCompositeOperator::Arithmetic { .. }  => "arithmetic",
                            });

                            match composite.operator {
                                FeCompositeOperator::Arithmetic { k1, k2, k3, k4 } => {
                                    xml.write_svg_attribute(AId::K1, &k1.value());
                                    xml.write_svg_attribute(AId::K2, &k2.value());
                                    xml.write_svg_attribute(AId::K3, &k3.value());
                                    xml.write_svg_attribute(AId::K4, &k4.value());
                                }
                                _ => {}
                            }

                            xml.write_svg_attribute(AId::Result, &fe.result);
                            xml.end_element();
                        }
                        FilterKind::FeMerge(ref merge) => {
                            xml.start_svg_element(EId::FeMerge);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_svg_attribute(AId::Result, &fe.result);
                            for input in &merge.inputs {
                                xml.start_svg_element(EId::FeMergeNode);
                                xml.write_filter_input(AId::In, &input);
                                xml.end_element();
                            }

                            xml.end_element();
                        }
                        FilterKind::FeTile(ref tile) => {
                            xml.start_svg_element(EId::FeTile);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_filter_input(AId::In, &tile.input);
                            xml.write_svg_attribute(AId::Result, &fe.result);
                            xml.end_element();
                        }
                        FilterKind::FeImage(ref img) => {
                            xml.start_svg_element(EId::FeImage);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_aspect(img.aspect);
                            xml.write_svg_attribute(AId::ImageRendering, match img.rendering_mode {
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

                            xml.write_svg_attribute(AId::Result, &fe.result);
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
                xml.start_svg_element(EId::Image);
                if !img.id.is_empty() {
                    xml.write_svg_attribute(AId::Id, &img.id);
                }

                xml.write_rect_attrs(img.view_box.rect);
                if !img.view_box.aspect.is_default() {
                    xml.write_aspect(img.view_box.aspect);
                }

                xml.write_visibility(img.visibility);

                match img.rendering_mode {
                    ImageRendering::OptimizeQuality => {}
                    ImageRendering::OptimizeSpeed => {
                        xml.write_svg_attribute(AId::ImageRendering, "optimizeSpeed");
                    }
                }

                xml.write_transform(AId::Transform, img.transform);
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

                xml.start_svg_element(EId::G);
                if !g.id.is_empty() {
                    xml.write_svg_attribute(AId::Id, &g.id);
                };

                if let Some(ref id) = g.clip_path {
                    xml.write_func_iri(AId::ClipPath, id);
                }

                if let Some(ref id) = g.mask {
                    xml.write_func_iri(AId::Mask, id);
                }

                if let Some(ref id) = g.filter {
                    xml.write_func_iri(AId::Filter, id);
                }

                if !g.opacity.is_default() {
                    xml.write_svg_attribute(AId::Opacity, &g.opacity.value());
                }

                xml.write_transform(AId::Transform, g.transform);

                conv_elements(&n, false, xml);

                xml.end_element();
            }
            _ => {}
        }
    }
}

trait XmlWriterExt {
    fn start_svg_element(&mut self, id: EId);
    fn write_svg_attribute<V: Display + ?Sized>(&mut self, id: AId, value: &V);
    fn write_viewbox(&mut self, view_box: &ViewBox);
    fn write_aspect(&mut self, aspect: AspectRatio);
    fn write_units(&mut self, id: AId, units: Units, def: Units);
    fn write_transform(&mut self, id: AId, units: Transform);
    fn write_visibility(&mut self, value: Visibility);
    fn write_func_iri(&mut self, aid: AId, id: &str);
    fn write_rect_attrs(&mut self, r: Rect);
    fn write_filter_input(&mut self, id: AId, input: &FilterInput);
    fn write_filter_primitive_attrs(&mut self, fe: &FilterPrimitive);
    fn write_image_data(&mut self, data: &ImageData, format: ImageFormat);
}

impl XmlWriterExt for XmlWriter {
    #[inline]
    fn start_svg_element(&mut self, id: EId) {
        self.start_element(id.as_str());
    }

    #[inline]
    fn write_svg_attribute<V: Display + ?Sized>(&mut self, id: AId, value: &V) {
        self.write_attribute(id.as_str(), value)
    }

    fn write_viewbox(&mut self, view_box: &ViewBox) {
        let r = view_box.rect;
        self.write_attribute_fmt(
            AId::ViewBox.as_str(),
            format_args!("{} {} {} {}", r.x(), r.y(), r.width(), r.height()),
        );

        if !view_box.aspect.is_default() {
            self.write_aspect(view_box.aspect);
        }
    }

    fn write_aspect(&mut self, aspect: AspectRatio) {
        self.write_attribute_raw(AId::PreserveAspectRatio.as_str(), |buf| aspect.write_buf(buf));
    }

    #[inline]
    fn write_units(&mut self, id: AId, units: Units, def: Units) {
        if units != def {
            self.write_attribute(id.as_str(), match units {
                Units::UserSpaceOnUse => "userSpaceOnUse",
                Units::ObjectBoundingBox => "objectBoundingBox",
            });
        }
    }

    fn write_transform(&mut self, id: AId, ts: Transform) {
        if !ts.is_default() {
            self.write_attribute_fmt(
                id.as_str(),
                format_args!("matrix({} {} {} {} {} {})", ts.a, ts.b, ts.c, ts.d, ts.e, ts.f),
            );
        }
    }

    fn write_visibility(&mut self, value: Visibility) {
        match value {
            Visibility::Visible => {},
            Visibility::Hidden => self.write_attribute(AId::Visibility.as_str(), "hidden"),
            Visibility::Collapse => self.write_attribute(AId::Visibility.as_str(), "collapse"),
        }
    }

    fn write_func_iri(&mut self, aid: AId, id: &str) {
        self.write_attribute_fmt(aid.as_str(), format_args!("url(#{})", id));
    }

    fn write_rect_attrs(&mut self, r: Rect) {
        self.write_svg_attribute(AId::X, &r.x());
        self.write_svg_attribute(AId::Y, &r.y());
        self.write_svg_attribute(AId::Width, &r.width());
        self.write_svg_attribute(AId::Height, &r.height());
    }

    fn write_filter_input(&mut self, id: AId, input: &FilterInput) {
        self.write_attribute(id.as_str(), match input {
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
        if let Some(n) = fe.x { self.write_svg_attribute(AId::X, &n); }
        if let Some(n) = fe.y { self.write_svg_attribute(AId::Y, &n); }
        if let Some(n) = fe.width { self.write_svg_attribute(AId::Width, &n); }
        if let Some(n) = fe.height { self.write_svg_attribute(AId::Height, &n); }

        self.write_attribute(AId::ColorInterpolationFilters.as_str(), match fe.color_interpolation {
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
    xml.write_units(AId::GradientUnits, g.units, Units::ObjectBoundingBox);
    xml.write_transform(AId::GradientTransform, g.transform);

    match g.spread_method {
        SpreadMethod::Pad => {},
        SpreadMethod::Reflect => xml.write_svg_attribute(AId::SpreadMethod, "reflect"),
        SpreadMethod::Repeat => xml.write_svg_attribute(AId::SpreadMethod, "repeat"),
    }

    for s in &g.stops {
        xml.start_svg_element(EId::Stop);
        xml.write_svg_attribute(AId::Offset, &s.offset.value());
        xml.write_svg_attribute(AId::StopColor, &s.color);
        if !s.opacity.is_default() {
            xml.write_svg_attribute(AId::StopOpacity, &s.opacity.value());
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
    xml.start_svg_element(EId::Path);
    if !path.id.is_empty() {
        xml.write_svg_attribute(AId::Id, &path.id);
    }

    write_fill(&path.fill, is_clip_path, xml);
    write_stroke(&path.stroke, xml);

    xml.write_visibility(path.visibility);

    match path.rendering_mode {
        ShapeRendering::OptimizeSpeed => {
            xml.write_svg_attribute(AId::ShapeRendering, "optimizeSpeed");
        }
        ShapeRendering::CrispEdges => {
            xml.write_svg_attribute(AId::ShapeRendering, "crispEdges")
        }
        ShapeRendering::GeometricPrecision  => {}
    }

    if let Some(ref id) = clip_path {
        xml.write_func_iri(AId::ClipPath, id);
    }

    xml.write_transform(AId::Transform, path.transform);

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
                        xml.write_svg_attribute(AId::Fill, &c);
                    }
                }
                Paint::Link(ref id) => {
                    xml.write_func_iri(AId::Fill, id);
                }
            }

            if !fill.opacity.is_default() {
                xml.write_svg_attribute(AId::FillOpacity, &fill.opacity.value());
            }

            if !fill.rule.is_default() {
                let name = if is_clip_path {
                    AId::ClipRule
                } else {
                    AId::FillRule
                };

                xml.write_svg_attribute(name, "evenodd");
            }
        }
        None => {
            xml.write_svg_attribute(AId::Fill, "none");
        }
    }
}

fn write_stroke(
    stroke: &Option<Stroke>,
    xml: &mut XmlWriter,
) {
    if let Some(ref stroke) = stroke {
        match stroke.paint {
            Paint::Color(ref c) => xml.write_svg_attribute(AId::Stroke, c),
            Paint::Link(ref id) => xml.write_func_iri(AId::Stroke, id),
        }

        if !stroke.opacity.is_default() {
            xml.write_svg_attribute(AId::StrokeOpacity, &stroke.opacity.value());
        }

        if !(stroke.dashoffset as f64).is_fuzzy_zero() {
            xml.write_svg_attribute(AId::StrokeDashoffset, &stroke.dashoffset)
        }

        if !stroke.miterlimit.is_default() {
            xml.write_svg_attribute(AId::StrokeMiterlimit, &stroke.miterlimit.value());
        }

        if !stroke.width.is_default() {
            xml.write_svg_attribute(AId::StrokeWidth, &stroke.width.value());
        }

        match stroke.linecap {
            LineCap::Butt => {}
            LineCap::Round => xml.write_svg_attribute(AId::StrokeLinecap, "round"),
            LineCap::Square => xml.write_svg_attribute(AId::StrokeLinecap, "square"),
        }

        match stroke.linejoin {
            LineJoin::Miter => {}
            LineJoin::Round => xml.write_svg_attribute(AId::StrokeLinejoin, "round"),
            LineJoin::Bevel => xml.write_svg_attribute(AId::StrokeLinejoin, "bevel"),
        }

        if let Some(ref array) = stroke.dasharray {
            xml.write_attribute_raw(AId::StrokeDasharray.as_str(), |buf| {
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
