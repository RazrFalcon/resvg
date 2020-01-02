// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt::Display;
use std::io::Write;
use std::ops::Deref;

use svgtypes::WriteBuffer;
use xmlwriter::XmlWriter;

use super::*;
use crate::{geom::*, svgtree::{EId, AId}, IsDefault};


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
    xml.write_attribute("xmlns:usvg", "https://github.com/RazrFalcon/resvg");
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
                                AId::StdDeviation.to_str(),
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
                                    xml.write_svg_attribute(AId::K1, &k1);
                                    xml.write_svg_attribute(AId::K2, &k2);
                                    xml.write_svg_attribute(AId::K3, &k3);
                                    xml.write_svg_attribute(AId::K4, &k4);
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
                                FeImageKind::Image(ref data, format) => {
                                    xml.write_image_data(data, format);
                                }
                                FeImageKind::Use(ref id) => {
                                    xml.write_attribute_fmt("xlink:href", format_args!("#{}", id));
                                }
                            }

                            xml.write_svg_attribute(AId::Result, &fe.result);
                            xml.end_element();
                        }
                        FilterKind::FeComponentTransfer(ref transfer) => {
                            xml.start_svg_element(EId::FeComponentTransfer);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_filter_input(AId::In, &transfer.input);
                            xml.write_svg_attribute(AId::Result, &fe.result);

                            xml.write_filter_transfer_function(EId::FeFuncR, &transfer.func_r);
                            xml.write_filter_transfer_function(EId::FeFuncG, &transfer.func_g);
                            xml.write_filter_transfer_function(EId::FeFuncB, &transfer.func_b);
                            xml.write_filter_transfer_function(EId::FeFuncA, &transfer.func_a);

                            xml.end_element();
                        }
                        FilterKind::FeColorMatrix(ref matrix) => {
                            xml.start_svg_element(EId::FeColorMatrix);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_filter_input(AId::In, &matrix.input);
                            xml.write_svg_attribute(AId::Result, &fe.result);

                            match matrix.kind {
                                FeColorMatrixKind::Matrix(ref values) => {
                                    xml.write_svg_attribute(AId::Type, "matrix");
                                    xml.write_numbers(AId::Values, values);
                                }
                                FeColorMatrixKind::Saturate(value) => {
                                    xml.write_svg_attribute(AId::Type, "saturate");
                                    xml.write_svg_attribute(AId::Values, &value.value());
                                }
                                FeColorMatrixKind::HueRotate(angle) => {
                                    xml.write_svg_attribute(AId::Type, "hueRotate");
                                    xml.write_svg_attribute(AId::Values, &angle);
                                }
                                FeColorMatrixKind::LuminanceToAlpha => {
                                    xml.write_svg_attribute(AId::Type, "luminanceToAlpha");
                                }
                            }

                            xml.end_element();
                        }
                        FilterKind::FeConvolveMatrix(ref matrix) => {
                            xml.start_svg_element(EId::FeConvolveMatrix);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_filter_input(AId::In, &matrix.input);
                            xml.write_svg_attribute(AId::Result, &fe.result);

                            xml.write_attribute_fmt(
                                AId::Order.to_str(),
                                format_args!("{} {}", matrix.matrix.columns(), matrix.matrix.rows()),
                            );
                            xml.write_numbers(AId::KernelMatrix, matrix.matrix.data());
                            xml.write_svg_attribute(AId::Divisor, &matrix.divisor.value());
                            xml.write_svg_attribute(AId::Bias, &matrix.bias);
                            xml.write_svg_attribute(AId::TargetX, &matrix.matrix.target_x());
                            xml.write_svg_attribute(AId::TargetY, &matrix.matrix.target_y());
                            xml.write_svg_attribute(AId::EdgeMode, match matrix.edge_mode {
                                FeEdgeMode::None => "none",
                                FeEdgeMode::Duplicate => "duplicate",
                                FeEdgeMode::Wrap => "wrap",
                            });
                            xml.write_svg_attribute(AId::PreserveAlpha,
                                if matrix.preserve_alpha { "true" } else { "false" });

                            xml.end_element();
                        }
                        FilterKind::FeMorphology(ref morphology) => {
                            xml.start_svg_element(EId::FeMorphology);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_filter_input(AId::In, &morphology.input);
                            xml.write_svg_attribute(AId::Result, &fe.result);

                            xml.write_svg_attribute(AId::Operator, match morphology.operator {
                                FeMorphologyOperator::Erode => "erode",
                                FeMorphologyOperator::Dilate => "dilate",
                            });
                            xml.write_attribute_fmt(
                                AId::Radius.to_str(),
                                format_args!("{} {}", morphology.radius_x.value(),
                                                      morphology.radius_y.value()),
                            );

                            xml.end_element();
                        }
                        FilterKind::FeDisplacementMap(ref map) => {
                            xml.start_svg_element(EId::FeDisplacementMap);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_filter_input(AId::In, &map.input1);
                            xml.write_filter_input(AId::In2, &map.input2);
                            xml.write_svg_attribute(AId::Result, &fe.result);

                            xml.write_svg_attribute(AId::Scale, &map.scale);

                            let mut write_channel = |c, aid| {
                                xml.write_svg_attribute(aid, match c {
                                    ColorChannel::R => "R",
                                    ColorChannel::G => "G",
                                    ColorChannel::B => "B",
                                    ColorChannel::A => "A",
                                });
                            };
                            write_channel(map.x_channel_selector, AId::XChannelSelector);
                            write_channel(map.y_channel_selector, AId::YChannelSelector);

                            xml.end_element();
                        }
                        FilterKind::FeTurbulence(ref turbulence) => {
                            xml.start_svg_element(EId::FeTurbulence);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_svg_attribute(AId::Result, &fe.result);

                            xml.write_point(AId::BaseFrequency, turbulence.base_frequency);
                            xml.write_svg_attribute(AId::NumOctaves, &turbulence.num_octaves);
                            xml.write_svg_attribute(AId::Seed, &turbulence.seed);
                            xml.write_svg_attribute(AId::StitchTiles, match turbulence.stitch_tiles {
                                true => "stitch",
                                false => "noStitch",
                            });
                            xml.write_svg_attribute(AId::Type, match turbulence.kind {
                                FeTurbulenceKind::FractalNoise => "fractalNoise",
                                FeTurbulenceKind::Turbulence => "turbulence",
                            });

                            xml.end_element();
                        }
                        FilterKind::FeDiffuseLighting(ref light) => {
                            xml.start_svg_element(EId::FeDiffuseLighting);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_svg_attribute(AId::Result, &fe.result);

                            xml.write_svg_attribute(AId::SurfaceScale, &light.surface_scale);
                            xml.write_svg_attribute(AId::DiffuseConstant, &light.diffuse_constant);
                            xml.write_svg_attribute(AId::LightingColor, &light.lighting_color);
                            write_light_source(&light.light_source, xml);

                            xml.end_element();
                        }
                        FilterKind::FeSpecularLighting(ref light) => {
                            xml.start_svg_element(EId::FeSpecularLighting);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_svg_attribute(AId::Result, &fe.result);

                            xml.write_svg_attribute(AId::SurfaceScale, &light.surface_scale);
                            xml.write_svg_attribute(AId::SpecularConstant, &light.specular_constant);
                            xml.write_svg_attribute(AId::SpecularExponent, &light.specular_exponent);
                            xml.write_svg_attribute(AId::LightingColor, &light.lighting_color);
                            write_light_source(&light.light_source, xml);

                            xml.end_element();
                        }
                    };
                }

                xml.end_element();
            }
            NodeKind::Group(_) |
            NodeKind::Image(_) |
            NodeKind::Path(_) => {
                conv_element(&n, false, xml);
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
        conv_element(&n, is_clip_path, xml);
    }
}

fn conv_element(
    node: &Node,
    is_clip_path: bool,
    xml: &mut XmlWriter,
) {
    match *node.borrow() {
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

                if let NodeKind::Path(ref path) = *node.first_child().unwrap().borrow() {
                    let clip_id = g.clip_path.as_ref().map(String::deref);
                    write_path(path, is_clip_path, clip_id, xml);
                }

                return;
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

                if let Some(ref fill) = g.filter_fill {
                    write_paint(AId::Fill, fill, xml);
                }

                if let Some(ref stroke) = g.filter_stroke {
                    write_paint(AId::Stroke, stroke, xml);
                }
            }

            if !g.opacity.is_default() {
                xml.write_svg_attribute(AId::Opacity, &g.opacity.value());
            }

            xml.write_transform(AId::Transform, g.transform);

            if let Some(eb) = g.enable_background {
                xml.write_enable_background(eb);
            }

            conv_elements(&node, false, xml);

            xml.end_element();
        }
        _ => {}
    }
}

trait XmlWriterExt {
    fn start_svg_element(&mut self, id: EId);
    fn write_svg_attribute<V: Display + ?Sized>(&mut self, id: AId, value: &V);
    fn write_viewbox(&mut self, view_box: &ViewBox);
    fn write_aspect(&mut self, aspect: AspectRatio);
    fn write_units(&mut self, id: AId, units: Units, def: Units);
    fn write_transform(&mut self, id: AId, units: Transform);
    fn write_enable_background(&mut self, eb: EnableBackground);
    fn write_visibility(&mut self, value: Visibility);
    fn write_func_iri(&mut self, aid: AId, id: &str);
    fn write_rect_attrs(&mut self, r: Rect);
    fn write_numbers(&mut self, aid: AId, list: &[f64]);
    fn write_point<T: Display>(&mut self, id: AId, p: Point<T>);
    fn write_filter_input(&mut self, id: AId, input: &FilterInput);
    fn write_filter_primitive_attrs(&mut self, fe: &FilterPrimitive);
    fn write_filter_transfer_function(&mut self, eid: EId, fe: &TransferFunction);
    fn write_image_data(&mut self, data: &ImageData, format: ImageFormat);
}

impl XmlWriterExt for XmlWriter {
    #[inline(never)]
    fn start_svg_element(&mut self, id: EId) {
        self.start_element(id.to_str());
    }

    #[inline(never)]
    fn write_svg_attribute<V: Display + ?Sized>(&mut self, id: AId, value: &V) {
        self.write_attribute(id.to_str(), value)
    }

    fn write_viewbox(&mut self, view_box: &ViewBox) {
        let r = view_box.rect;
        self.write_attribute_fmt(
            AId::ViewBox.to_str(),
            format_args!("{} {} {} {}", r.x(), r.y(), r.width(), r.height()),
        );

        if !view_box.aspect.is_default() {
            self.write_aspect(view_box.aspect);
        }
    }

    fn write_aspect(&mut self, aspect: AspectRatio) {
        self.write_attribute_raw(AId::PreserveAspectRatio.to_str(), |buf| aspect.write_buf(buf));
    }

    fn write_units(&mut self, id: AId, units: Units, def: Units) {
        if units != def {
            self.write_attribute(id.to_str(), match units {
                Units::UserSpaceOnUse => "userSpaceOnUse",
                Units::ObjectBoundingBox => "objectBoundingBox",
            });
        }
    }

    fn write_transform(&mut self, id: AId, ts: Transform) {
        if !ts.is_default() {
            self.write_attribute_fmt(
                id.to_str(),
                format_args!("matrix({} {} {} {} {} {})", ts.a, ts.b, ts.c, ts.d, ts.e, ts.f),
            );
        }
    }

    fn write_enable_background(&mut self, eb: EnableBackground) {
        let id = AId::EnableBackground.to_str();
        match eb {
            EnableBackground(None) => {
                self.write_attribute(id, "new");
            }
            EnableBackground(Some(r)) => {
                self.write_attribute_fmt(
                    id,
                    format_args!("new {} {} {} {}", r.x(), r.y(), r.width(), r.height()),
                );
            }
        }
    }

    fn write_visibility(&mut self, value: Visibility) {
        match value {
            Visibility::Visible => {},
            Visibility::Hidden => self.write_attribute(AId::Visibility.to_str(), "hidden"),
            Visibility::Collapse => self.write_attribute(AId::Visibility.to_str(), "collapse"),
        }
    }

    fn write_func_iri(&mut self, aid: AId, id: &str) {
        self.write_attribute_fmt(aid.to_str(), format_args!("url(#{})", id));
    }

    fn write_rect_attrs(&mut self, r: Rect) {
        self.write_svg_attribute(AId::X, &r.x());
        self.write_svg_attribute(AId::Y, &r.y());
        self.write_svg_attribute(AId::Width, &r.width());
        self.write_svg_attribute(AId::Height, &r.height());
    }

    fn write_numbers(&mut self, aid: AId, list: &[f64]) {
        self.write_attribute_raw(aid.to_str(), |buf| {
            for n in list {
                buf.write_fmt(format_args!("{} ", n)).unwrap();
            }

            if !list.is_empty() {
                buf.pop();
            }
        });
    }

    fn write_point<T: Display>(&mut self, id: AId, p: Point<T>) {
        self.write_attribute_fmt(id.to_str(), format_args!("{} {}", p.x, p.y));
    }

    fn write_filter_input(&mut self, id: AId, input: &FilterInput) {
        self.write_attribute(id.to_str(), match input {
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

        self.write_attribute(AId::ColorInterpolationFilters.to_str(), match fe.color_interpolation {
            ColorInterpolation::SRGB        => "sRGB",
            ColorInterpolation::LinearRGB   => "linearRGB"
        });
    }

    fn write_filter_transfer_function(&mut self, eid: EId, fe: &TransferFunction) {
        self.start_svg_element(eid);

        match fe {
            TransferFunction::Identity => {
                self.write_svg_attribute(AId::Type, "identity");
            }
            TransferFunction::Table(ref values) => {
                self.write_svg_attribute(AId::Type, "table");
                self.write_numbers(AId::TableValues, values);
            }
            TransferFunction::Discrete(ref values) => {
                self.write_svg_attribute(AId::Type, "discrete");
                self.write_numbers(AId::TableValues, values);
            }
            TransferFunction::Linear { slope, intercept } => {
                self.write_svg_attribute(AId::Type, "linear");
                self.write_svg_attribute(AId::Slope, &slope);
                self.write_svg_attribute(AId::Intercept, &intercept);
            }
            TransferFunction::Gamma { amplitude, exponent, offset } => {
                self.write_svg_attribute(AId::Type, "gamma");
                self.write_svg_attribute(AId::Amplitude, &amplitude);
                self.write_svg_attribute(AId::Exponent, &exponent);
                self.write_svg_attribute(AId::Offset, &offset);
            }
        }

        self.end_element();
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
                    if let FilterKind::FeImage(..) = fe.kind {
                        return true;
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
        ShapeRendering::GeometricPrecision => {}
    }

    if let Some(ref id) = clip_path {
        xml.write_func_iri(AId::ClipPath, id);
    }

    xml.write_transform(AId::Transform, path.transform);

    xml.write_attribute_raw("d", |buf| {
        for seg in path.data.iter() {
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

        if !path.data.is_empty() {
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
    if let Some(ref fill) = fill {
        write_paint(AId::Fill, &fill.paint, xml);

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
    } else {
        xml.write_svg_attribute(AId::Fill, "none");
    }
}

fn write_stroke(
    stroke: &Option<Stroke>,
    xml: &mut XmlWriter,
) {
    if let Some(ref stroke) = stroke {
        write_paint(AId::Stroke, &stroke.paint, xml);

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
            xml.write_numbers(AId::StrokeDasharray, array);
        }
    } else {
        // Always set `stroke` to `none` to override the parent value.
        // In 99.9% of the cases it's redundant, but a group with `filter` with `StrokePaint`
        // will set `stroke`, which will interfere with children nodes.
        xml.write_svg_attribute(AId::Stroke, "none");
    }
}

fn write_paint(
    aid: AId,
    paint: &Paint,
    xml: &mut XmlWriter,
) {
    match paint {
        Paint::Color(ref c) => xml.write_svg_attribute(aid, c),
        Paint::Link(ref id) => xml.write_func_iri(aid, id),
    }
}

fn write_light_source(
    light: &FeLightSource,
    xml: &mut XmlWriter,
) {
    match light {
        FeLightSource::FeDistantLight(ref light) => {
            xml.start_svg_element(EId::FeDistantLight);
            xml.write_svg_attribute(AId::Azimuth, &light.azimuth);
            xml.write_svg_attribute(AId::Elevation, &light.elevation);
        }
        FeLightSource::FePointLight(ref light) => {
            xml.start_svg_element(EId::FePointLight);
            xml.write_svg_attribute(AId::X, &light.x);
            xml.write_svg_attribute(AId::Y, &light.y);
            xml.write_svg_attribute(AId::Z, &light.z);
        }
        FeLightSource::FeSpotLight(ref light) => {
            xml.start_svg_element(EId::FeSpotLight);
            xml.write_svg_attribute(AId::X, &light.x);
            xml.write_svg_attribute(AId::Y, &light.y);
            xml.write_svg_attribute(AId::Z, &light.z);
            xml.write_svg_attribute(AId::PointsAtX, &light.points_at_x);
            xml.write_svg_attribute(AId::PointsAtY, &light.points_at_y);
            xml.write_svg_attribute(AId::PointsAtZ, &light.points_at_z);
            xml.write_svg_attribute(AId::SpecularExponent, &light.specular_exponent);
            if let Some(ref n) = light.limiting_cone_angle {
                xml.write_svg_attribute(AId::LimitingConeAngle, n);
            }
        }
    }

    xml.end_element();
}
