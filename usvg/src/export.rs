// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt::Display;
use std::io::Write;

use xmlwriter::XmlWriter;

use crate::svgtree::{EId, AId};
use crate::*;

pub(crate) fn convert(tree: &Tree, opt: &XmlOptions) -> String {
    let mut xml = XmlWriter::new(opt.writer_opts);

    let svg_node = tree.svg_node();

    xml.start_svg_element(EId::Svg);
    xml.write_svg_attribute(AId::Width, &svg_node.size.width());
    xml.write_svg_attribute(AId::Height, &svg_node.size.height());
    xml.write_viewbox(&svg_node.view_box);
    xml.write_attribute("xmlns", "http://www.w3.org/2000/svg");
    if has_xlink(tree) {
        xml.write_attribute("xmlns:xlink", "http://www.w3.org/1999/xlink");
    }

    xml.start_svg_element(EId::Defs);
    conv_defs(tree, opt, &mut xml);
    xml.end_element();

    conv_elements(&tree.root(), false, opt, &mut xml);

    xml.end_document()
}

fn conv_defs(tree: &Tree, opt: &XmlOptions, xml: &mut XmlWriter) {
    for n in tree.defs().children() {
        match *n.borrow() {
            NodeKind::LinearGradient(ref lg) => {
                xml.start_svg_element(EId::LinearGradient);
                xml.write_id_attribute(&lg.id, opt);
                xml.write_svg_attribute(AId::X1, &lg.x1);
                xml.write_svg_attribute(AId::Y1, &lg.y1);
                xml.write_svg_attribute(AId::X2, &lg.x2);
                xml.write_svg_attribute(AId::Y2, &lg.y2);
                write_base_grad(&lg.base, xml);
                xml.end_element();
            }
            NodeKind::RadialGradient(ref rg) => {
                xml.start_svg_element(EId::RadialGradient);
                xml.write_id_attribute(&rg.id, opt);
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
                xml.write_id_attribute(&clip.id, opt);
                xml.write_units(AId::ClipPathUnits, clip.units, Units::UserSpaceOnUse);
                xml.write_transform(AId::Transform, clip.transform);

                if let Some(ref id) = clip.clip_path {
                    xml.write_func_iri(AId::ClipPath, id, opt);
                }

                conv_elements(&n, true, opt, xml);

                xml.end_element();
            }
            NodeKind::Mask(ref mask) => {
                xml.start_svg_element(EId::Mask);
                xml.write_id_attribute(&mask.id, opt);
                xml.write_units(AId::MaskUnits, mask.units, Units::ObjectBoundingBox);
                xml.write_units(AId::MaskContentUnits, mask.content_units, Units::UserSpaceOnUse);
                xml.write_rect_attrs(mask.rect);

                if let Some(ref id) = mask.mask {
                    xml.write_func_iri(AId::Mask, id, opt);
                }

                conv_elements(&n, false, opt, xml);

                xml.end_element();
            }
            NodeKind::Pattern(ref pattern) => {
                xml.start_svg_element(EId::Pattern);
                xml.write_id_attribute(&pattern.id, opt);
                xml.write_rect_attrs(pattern.rect);
                xml.write_units(AId::PatternUnits, pattern.units, Units::ObjectBoundingBox);
                xml.write_units(AId::PatternContentUnits, pattern.content_units, Units::UserSpaceOnUse);
                xml.write_transform(AId::PatternTransform, pattern.transform);

                if let Some(ref vbox) = pattern.view_box {
                    xml.write_viewbox(vbox);
                }

                conv_elements(&n, false, opt, xml);

                xml.end_element();
            }
            NodeKind::Filter(ref filter) => {
                xml.start_svg_element(EId::Filter);
                xml.write_id_attribute(&filter.id, opt);
                xml.write_rect_attrs(filter.rect);
                xml.write_units(AId::FilterUnits, filter.units, Units::ObjectBoundingBox);
                xml.write_units(AId::PrimitiveUnits, filter.primitive_units, Units::UserSpaceOnUse);

                for fe in &filter.primitives {
                    match fe.kind {
                        filter::Kind::DropShadow(ref shadow) => {
                            xml.start_svg_element(EId::FeDropShadow);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_filter_input(AId::In, &shadow.input);
                            xml.write_attribute_fmt(
                                AId::StdDeviation.to_str(),
                                format_args!("{} {}", shadow.std_dev_x.value(), shadow.std_dev_y.value()),
                            );
                            xml.write_svg_attribute(AId::Dx, &shadow.dx);
                            xml.write_svg_attribute(AId::Dy, &shadow.dy);
                            xml.write_color(AId::FloodColor, shadow.color);
                            xml.write_svg_attribute(AId::FloodOpacity, &shadow.opacity.value());
                            xml.write_svg_attribute(AId::Result, &fe.result);
                            xml.end_element();
                        }
                        filter::Kind::GaussianBlur(ref blur) => {
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
                        filter::Kind::Offset(ref offset) => {
                            xml.start_svg_element(EId::FeOffset);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_filter_input(AId::In, &offset.input);
                            xml.write_svg_attribute(AId::Dx, &offset.dx);
                            xml.write_svg_attribute(AId::Dy, &offset.dy);
                            xml.write_svg_attribute(AId::Result, &fe.result);
                            xml.end_element();
                        }
                        filter::Kind::Blend(ref blend) => {
                            xml.start_svg_element(EId::FeBlend);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_filter_input(AId::In, &blend.input1);
                            xml.write_filter_input(AId::In2, &blend.input2);
                            xml.write_svg_attribute(AId::Mode, match blend.mode {
                                filter::BlendMode::Normal     => "normal",
                                filter::BlendMode::Multiply   => "multiply",
                                filter::BlendMode::Screen     => "screen",
                                filter::BlendMode::Overlay    => "overlay",
                                filter::BlendMode::Darken     => "darken",
                                filter::BlendMode::Lighten    => "lighten",
                                filter::BlendMode::ColorDodge => "color-dodge",
                                filter::BlendMode::ColorBurn  => "color-burn",
                                filter::BlendMode::HardLight  => "hard-light",
                                filter::BlendMode::SoftLight  => "soft-light",
                                filter::BlendMode::Difference => "difference",
                                filter::BlendMode::Exclusion  => "exclusion",
                                filter::BlendMode::Hue        => "hue",
                                filter::BlendMode::Saturation => "saturation",
                                filter::BlendMode::Color      => "color",
                                filter::BlendMode::Luminosity => "luminosity",
                            });
                            xml.write_svg_attribute(AId::Result, &fe.result);
                            xml.end_element();
                        }
                        filter::Kind::Flood(ref flood) => {
                            xml.start_svg_element(EId::FeFlood);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_color(AId::FloodColor, flood.color);
                            xml.write_svg_attribute(AId::FloodOpacity, &flood.opacity.value());
                            xml.write_svg_attribute(AId::Result, &fe.result);
                            xml.end_element();
                        }
                        filter::Kind::Composite(ref composite) => {
                            xml.start_svg_element(EId::FeComposite);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_filter_input(AId::In, &composite.input1);
                            xml.write_filter_input(AId::In2, &composite.input2);
                            xml.write_svg_attribute(AId::Operator, match composite.operator {
                                filter::CompositeOperator::Over               => "over",
                                filter::CompositeOperator::In                 => "in",
                                filter::CompositeOperator::Out                => "out",
                                filter::CompositeOperator::Atop               => "atop",
                                filter::CompositeOperator::Xor                => "xor",
                                filter::CompositeOperator::Arithmetic { .. }  => "arithmetic",
                            });

                           if let filter::CompositeOperator::Arithmetic { k1, k2, k3, k4 } = composite.operator {
                                    xml.write_svg_attribute(AId::K1, &k1);
                                    xml.write_svg_attribute(AId::K2, &k2);
                                    xml.write_svg_attribute(AId::K3, &k3);
                                    xml.write_svg_attribute(AId::K4, &k4);
                            }

                            xml.write_svg_attribute(AId::Result, &fe.result);
                            xml.end_element();
                        }
                        filter::Kind::Merge(ref merge) => {
                            xml.start_svg_element(EId::FeMerge);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_svg_attribute(AId::Result, &fe.result);
                            for input in &merge.inputs {
                                xml.start_svg_element(EId::FeMergeNode);
                                xml.write_filter_input(AId::In, input);
                                xml.end_element();
                            }

                            xml.end_element();
                        }
                        filter::Kind::Tile(ref tile) => {
                            xml.start_svg_element(EId::FeTile);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_filter_input(AId::In, &tile.input);
                            xml.write_svg_attribute(AId::Result, &fe.result);
                            xml.end_element();
                        }
                        filter::Kind::Image(ref img) => {
                            xml.start_svg_element(EId::FeImage);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_aspect(img.aspect);
                            xml.write_svg_attribute(AId::ImageRendering, match img.rendering_mode {
                                ImageRendering::OptimizeQuality => "optimizeQuality",
                                ImageRendering::OptimizeSpeed   => "optimizeSpeed",
                            });
                            match img.data {
                                filter::ImageKind::Image(ref kind) => {
                                    xml.write_image_data(kind);
                                }
                                filter::ImageKind::Use(ref id) => {
                                    let prefix = opt.id_prefix.as_deref().unwrap_or_default();
                                    xml.write_attribute_fmt(
                                        "xlink:href",
                                        format_args!("#{}{}", prefix, id),
                                    );
                                }
                            }

                            xml.write_svg_attribute(AId::Result, &fe.result);
                            xml.end_element();
                        }
                        filter::Kind::ComponentTransfer(ref transfer) => {
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
                        filter::Kind::ColorMatrix(ref matrix) => {
                            xml.start_svg_element(EId::FeColorMatrix);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_filter_input(AId::In, &matrix.input);
                            xml.write_svg_attribute(AId::Result, &fe.result);

                            match matrix.kind {
                                filter::ColorMatrixKind::Matrix(ref values) => {
                                    xml.write_svg_attribute(AId::Type, "matrix");
                                    xml.write_numbers(AId::Values, values);
                                }
                                filter::ColorMatrixKind::Saturate(value) => {
                                    xml.write_svg_attribute(AId::Type, "saturate");
                                    xml.write_svg_attribute(AId::Values, &value.value());
                                }
                                filter::ColorMatrixKind::HueRotate(angle) => {
                                    xml.write_svg_attribute(AId::Type, "hueRotate");
                                    xml.write_svg_attribute(AId::Values, &angle);
                                }
                                filter::ColorMatrixKind::LuminanceToAlpha => {
                                    xml.write_svg_attribute(AId::Type, "luminanceToAlpha");
                                }
                            }

                            xml.end_element();
                        }
                        filter::Kind::ConvolveMatrix(ref matrix) => {
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
                                filter::EdgeMode::None => "none",
                                filter::EdgeMode::Duplicate => "duplicate",
                                filter::EdgeMode::Wrap => "wrap",
                            });
                            xml.write_svg_attribute(AId::PreserveAlpha,
                                if matrix.preserve_alpha { "true" } else { "false" });

                            xml.end_element();
                        }
                        filter::Kind::Morphology(ref morphology) => {
                            xml.start_svg_element(EId::FeMorphology);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_filter_input(AId::In, &morphology.input);
                            xml.write_svg_attribute(AId::Result, &fe.result);

                            xml.write_svg_attribute(AId::Operator, match morphology.operator {
                                filter::MorphologyOperator::Erode => "erode",
                                filter::MorphologyOperator::Dilate => "dilate",
                            });
                            xml.write_attribute_fmt(
                                AId::Radius.to_str(),
                                format_args!("{} {}", morphology.radius_x.value(),
                                                      morphology.radius_y.value()),
                            );

                            xml.end_element();
                        }
                        filter::Kind::DisplacementMap(ref map) => {
                            xml.start_svg_element(EId::FeDisplacementMap);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_filter_input(AId::In, &map.input1);
                            xml.write_filter_input(AId::In2, &map.input2);
                            xml.write_svg_attribute(AId::Result, &fe.result);

                            xml.write_svg_attribute(AId::Scale, &map.scale);

                            let mut write_channel = |c, aid| {
                                xml.write_svg_attribute(aid, match c {
                                    filter::ColorChannel::R => "R",
                                    filter::ColorChannel::G => "G",
                                    filter::ColorChannel::B => "B",
                                    filter::ColorChannel::A => "A",
                                });
                            };
                            write_channel(map.x_channel_selector, AId::XChannelSelector);
                            write_channel(map.y_channel_selector, AId::YChannelSelector);

                            xml.end_element();
                        }
                        filter::Kind::Turbulence(ref turbulence) => {
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
                                filter::TurbulenceKind::FractalNoise => "fractalNoise",
                                filter::TurbulenceKind::Turbulence => "turbulence",
                            });

                            xml.end_element();
                        }
                        filter::Kind::DiffuseLighting(ref light) => {
                            xml.start_svg_element(EId::FeDiffuseLighting);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_svg_attribute(AId::Result, &fe.result);

                            xml.write_svg_attribute(AId::SurfaceScale, &light.surface_scale);
                            xml.write_svg_attribute(AId::DiffuseConstant, &light.diffuse_constant);
                            xml.write_color(AId::LightingColor, light.lighting_color);
                            write_light_source(&light.light_source, xml);

                            xml.end_element();
                        }
                        filter::Kind::SpecularLighting(ref light) => {
                            xml.start_svg_element(EId::FeSpecularLighting);
                            xml.write_filter_primitive_attrs(fe);
                            xml.write_svg_attribute(AId::Result, &fe.result);

                            xml.write_svg_attribute(AId::SurfaceScale, &light.surface_scale);
                            xml.write_svg_attribute(AId::SpecularConstant, &light.specular_constant);
                            xml.write_svg_attribute(AId::SpecularExponent, &light.specular_exponent);
                            xml.write_color(AId::LightingColor, light.lighting_color);
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
                conv_element(&n, false, opt, xml);
            }
            _ => {}
        }
    }
}

fn conv_elements(
    parent: &Node,
    is_clip_path: bool,
    opt: &XmlOptions,
    xml: &mut XmlWriter,
) {
    for n in parent.children() {
        conv_element(&n, is_clip_path, opt, xml);
    }
}

fn conv_element(
    node: &Node,
    is_clip_path: bool,
    opt: &XmlOptions,
    xml: &mut XmlWriter,
) {
    match *node.borrow() {
        NodeKind::Path(ref p) => {
            write_path(p, is_clip_path, None, opt, xml);
        }
        NodeKind::Image(ref img) => {
            xml.start_svg_element(EId::Image);
            if !img.id.is_empty() {
                xml.write_id_attribute(&img.id, opt);
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
            xml.write_image_data(&img.kind);

            xml.end_element();
        }
        NodeKind::Group(ref g) => {
            if is_clip_path {
                // ClipPath with a Group element is an `usvg` special case.
                // Group will contains a single Path element and we should set
                // `clip-path` on it.

                if let NodeKind::Path(ref path) = *node.first_child().unwrap().borrow() {
                    let clip_id = g.clip_path.as_deref();
                    write_path(path, is_clip_path, clip_id, opt, xml);
                }

                return;
            }

            xml.start_svg_element(EId::G);
            if !g.id.is_empty() {
                xml.write_id_attribute(&g.id, opt);
            };

            if let Some(ref id) = g.clip_path {
                xml.write_func_iri(AId::ClipPath, id, opt);
            }

            if let Some(ref id) = g.mask {
                xml.write_func_iri(AId::Mask, id, opt);
            }

            if !g.filter.is_empty() {
                let prefix = opt.id_prefix.as_deref().unwrap_or_default();
                let ids: Vec<_> = g.filter.iter()
                    .map(|id| format!("url(#{}{})", prefix, id))
                    .collect();
                xml.write_svg_attribute(AId::Filter, &ids.join(" "));

                if let Some(ref fill) = g.filter_fill {
                    write_paint(AId::Fill, fill, opt, xml);
                }

                if let Some(ref stroke) = g.filter_stroke {
                    write_paint(AId::Stroke, stroke, opt, xml);
                }
            }

            if !g.opacity.is_default() {
                xml.write_svg_attribute(AId::Opacity, &g.opacity.value());
            }

            xml.write_transform(AId::Transform, g.transform);

            if let Some(eb) = g.enable_background {
                xml.write_enable_background(eb);
            }

            conv_elements(node, false, opt, xml);

            xml.end_element();
        }
        _ => {}
    }
}

trait XmlWriterExt {
    fn start_svg_element(&mut self, id: EId);
    fn write_svg_attribute<V: Display + ?Sized>(&mut self, id: AId, value: &V);
    fn write_id_attribute(&mut self, value: &str, opt: &XmlOptions);
    fn write_color(&mut self, id: AId, color: Color);
    fn write_viewbox(&mut self, view_box: &ViewBox);
    fn write_aspect(&mut self, aspect: AspectRatio);
    fn write_units(&mut self, id: AId, units: Units, def: Units);
    fn write_transform(&mut self, id: AId, units: Transform);
    fn write_enable_background(&mut self, eb: EnableBackground);
    fn write_visibility(&mut self, value: Visibility);
    fn write_func_iri(&mut self, aid: AId, id: &str, opt: &XmlOptions);
    fn write_rect_attrs(&mut self, r: Rect);
    fn write_numbers(&mut self, aid: AId, list: &[f64]);
    fn write_point<T: Display>(&mut self, id: AId, p: Point<T>);
    fn write_filter_input(&mut self, id: AId, input: &filter::Input);
    fn write_filter_primitive_attrs(&mut self, fe: &filter::Primitive);
    fn write_filter_transfer_function(&mut self, eid: EId, fe: &filter::TransferFunction);
    fn write_image_data(&mut self, kind: &ImageKind);
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

    #[inline(never)]
    fn write_id_attribute(&mut self, value: &str, opt: &XmlOptions) {
        if let Some(ref prefix) = opt.id_prefix {
            self.write_attribute_fmt("id", format_args!("{}{}", prefix, value));
        } else {
            self.write_attribute("id", value);
        }
    }

    #[inline(never)]
    fn write_color(&mut self, id: AId, c: Color) {
        self.write_attribute_fmt(
            id.to_str(),
            format_args!("rgba({},{},{},{})", c.red, c.green, c.blue, c.alpha)
        )
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
        let mut value = Vec::new();

        if aspect.defer {
            value.extend_from_slice(b"defer ");
        }

        let align = match aspect.align {
            Align::None     => "none",
            Align::XMinYMin => "xMinYMin",
            Align::XMidYMin => "xMidYMin",
            Align::XMaxYMin => "xMaxYMin",
            Align::XMinYMid => "xMinYMid",
            Align::XMidYMid => "xMidYMid",
            Align::XMaxYMid => "xMaxYMid",
            Align::XMinYMax => "xMinYMax",
            Align::XMidYMax => "xMidYMax",
            Align::XMaxYMax => "xMaxYMax",
        };

        value.extend_from_slice(align.as_bytes());

        if aspect.slice {
            value.extend_from_slice(b" slice");
        }

        self.write_attribute_raw(
            AId::PreserveAspectRatio.to_str(),
            |buf| buf.extend_from_slice(&value),
        );
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

    fn write_func_iri(&mut self, aid: AId, id: &str, opt: &XmlOptions) {
        let prefix = opt.id_prefix.as_deref().unwrap_or_default();
        self.write_attribute_fmt(aid.to_str(), format_args!("url(#{}{})", prefix, id));
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

    fn write_filter_input(&mut self, id: AId, input: &filter::Input) {
        self.write_attribute(id.to_str(), match input {
            filter::Input::SourceGraphic      => "SourceGraphic",
            filter::Input::SourceAlpha        => "SourceAlpha",
            filter::Input::BackgroundImage    => "BackgroundImage",
            filter::Input::BackgroundAlpha    => "BackgroundAlpha",
            filter::Input::FillPaint          => "FillPaint",
            filter::Input::StrokePaint        => "StrokePaint",
            filter::Input::Reference(ref s)   => s,
        });
    }

    fn write_filter_primitive_attrs(&mut self, fe: &filter::Primitive) {
        if let Some(n) = fe.x { self.write_svg_attribute(AId::X, &n); }
        if let Some(n) = fe.y { self.write_svg_attribute(AId::Y, &n); }
        if let Some(n) = fe.width { self.write_svg_attribute(AId::Width, &n); }
        if let Some(n) = fe.height { self.write_svg_attribute(AId::Height, &n); }

        self.write_attribute(AId::ColorInterpolationFilters.to_str(), match fe.color_interpolation {
            filter::ColorInterpolation::SRGB        => "sRGB",
            filter::ColorInterpolation::LinearRGB   => "linearRGB"
        });
    }

    fn write_filter_transfer_function(&mut self, eid: EId, fe: &filter::TransferFunction) {
        self.start_svg_element(eid);

        match fe {
            filter::TransferFunction::Identity => {
                self.write_svg_attribute(AId::Type, "identity");
            }
            filter::TransferFunction::Table(ref values) => {
                self.write_svg_attribute(AId::Type, "table");
                self.write_numbers(AId::TableValues, values);
            }
            filter::TransferFunction::Discrete(ref values) => {
                self.write_svg_attribute(AId::Type, "discrete");
                self.write_numbers(AId::TableValues, values);
            }
            filter::TransferFunction::Linear { slope, intercept } => {
                self.write_svg_attribute(AId::Type, "linear");
                self.write_svg_attribute(AId::Slope, &slope);
                self.write_svg_attribute(AId::Intercept, &intercept);
            }
            filter::TransferFunction::Gamma { amplitude, exponent, offset } => {
                self.write_svg_attribute(AId::Type, "gamma");
                self.write_svg_attribute(AId::Amplitude, &amplitude);
                self.write_svg_attribute(AId::Exponent, &exponent);
                self.write_svg_attribute(AId::Offset, &offset);
            }
        }

        self.end_element();
    }

    fn write_image_data(&mut self, kind: &crate::ImageKind) {
        let svg_string;
        let (mime, data) = match kind {
            crate::ImageKind::JPEG(ref data) => {
                ("jpg", data.as_slice())
            }
            crate::ImageKind::PNG(ref data) => {
                ("png", data.as_slice())
            }
            crate::ImageKind::SVG(ref tree) => {
                svg_string = tree.to_string(&XmlOptions::default());
                ("svg+xml", svg_string.as_bytes())
            }
        };

        self.write_attribute_raw("xlink:href", |buf| {
            buf.extend_from_slice(b"data:image/");
            buf.extend_from_slice(mime.as_bytes());
            buf.extend_from_slice(b";base64, ");

            let mut enc = base64::write::EncoderWriter::new(buf, base64::STANDARD);
            enc.write_all(data).unwrap();
            enc.finish().unwrap();
        });
    }
}

fn has_xlink(tree: &Tree) -> bool {
    for n in tree.root().descendants() {
        match *n.borrow() {
            NodeKind::Filter(ref filter) => {
                for fe in &filter.primitives {
                    if let filter::Kind::Image(..) = fe.kind {
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
        xml.write_color(AId::StopColor, s.color);
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
    opt: &XmlOptions,
    xml: &mut XmlWriter,
) {
    xml.start_svg_element(EId::Path);
    if !path.id.is_empty() {
        xml.write_id_attribute(&path.id, opt);
    }

    write_fill(&path.fill, is_clip_path, opt, xml);
    write_stroke(&path.stroke, opt, xml);

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

    if let Some(id) = clip_path {
        xml.write_func_iri(AId::ClipPath, id, opt);
    }

    xml.write_transform(AId::Transform, path.transform);

    xml.write_attribute_raw("d", |buf| {
        for seg in path.data.iter() {
            match *seg {
                PathSegment::MoveTo { x, y } => {
                    buf.extend_from_slice(b"M ");
                    write_num(x, buf);
                    buf.push(b' ');
                    write_num(y, buf);
                    buf.push(b' ');
                }
                PathSegment::LineTo { x, y } => {
                    buf.extend_from_slice(b"L ");
                    write_num(x, buf);
                    buf.push(b' ');
                    write_num(y, buf);
                    buf.push(b' ');
                }
                PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                    buf.extend_from_slice(b"C ");
                    write_num(x1, buf);
                    buf.push(b' ');
                    write_num(y1, buf);
                    buf.push(b' ');
                    write_num(x2, buf);
                    buf.push(b' ');
                    write_num(y2, buf);
                    buf.push(b' ');
                    write_num(x, buf);
                    buf.push(b' ');
                    write_num(y, buf);
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
    opt: &XmlOptions,
    xml: &mut XmlWriter,
) {
    if let Some(ref fill) = fill {
        write_paint(AId::Fill, &fill.paint, opt, xml);

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
    opt: &XmlOptions,
    xml: &mut XmlWriter,
) {
    if let Some(ref stroke) = stroke {
        write_paint(AId::Stroke, &stroke.paint, opt, xml);

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
    opt: &XmlOptions,
    xml: &mut XmlWriter,
) {
    match paint {
        Paint::Color(c) => xml.write_color(aid, *c),
        Paint::Link(ref id) => xml.write_func_iri(aid, id, opt),
    }
}

fn write_light_source(
    light: &filter::LightSource,
    xml: &mut XmlWriter,
) {
    match light {
        filter::LightSource::DistantLight(ref light) => {
            xml.start_svg_element(EId::FeDistantLight);
            xml.write_svg_attribute(AId::Azimuth, &light.azimuth);
            xml.write_svg_attribute(AId::Elevation, &light.elevation);
        }
        filter::LightSource::PointLight(ref light) => {
            xml.start_svg_element(EId::FePointLight);
            xml.write_svg_attribute(AId::X, &light.x);
            xml.write_svg_attribute(AId::Y, &light.y);
            xml.write_svg_attribute(AId::Z, &light.z);
        }
        filter::LightSource::SpotLight(ref light) => {
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

fn write_num(num: f64, buf: &mut Vec<u8>) {
    // If number is an integer, it's faster to write it as i32.
    if num.fract().is_fuzzy_zero() {
        write!(buf, "{}", num as i32).unwrap();
        return;
    }

    // Round numbers up to 11 digits to prevent writing
    // ugly numbers like 29.999999999999996.
    // It's not 100% correct, but differences are insignificant.
    let v = (num * 100_000_000_000.0).round() / 100_000_000_000.0;

    write!(buf, "{}", v).unwrap();
}
