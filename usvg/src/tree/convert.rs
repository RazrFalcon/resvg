// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use base64;
use svgdom;

// self
use super::*;
use geom::*;
use short::{
    AId,
    AValue,
    EId,
};


pub fn conv_doc(tree: &Tree) -> svgdom::Document {
    let mut new_doc = svgdom::Document::new();

    let mut svg = new_doc.create_element(EId::Svg);
    new_doc.root().append(svg.clone());

    let svg_node = tree.svg_node();

    svg.set_attribute((AId::Width,  svg_node.size.width));
    svg.set_attribute((AId::Height, svg_node.size.height));
    conv_viewbox(&svg_node.view_box, &mut svg);
    svg.set_attribute((("xmlns:usvg"), "https://github.com/RazrFalcon/usvg"));
    svg.set_attribute((("usvg:version"), env!("CARGO_PKG_VERSION")));

    let mut defs = new_doc.create_element(EId::Defs);
    svg.append(defs.clone());

    conv_defs(tree, &mut new_doc, &mut defs);
    conv_elements(tree, &tree.root(), &defs, &mut new_doc, &mut svg);

    new_doc
}

fn conv_defs(
    tree: &Tree,
    new_doc: &mut svgdom::Document,
    defs: &mut svgdom::Node,
) {
    let mut later_nodes = Vec::new();
    let mut link_clip_later = Vec::new();
    let mut link_mask_later = Vec::new();

    for n in tree.defs().children() {
        match *n.borrow() {
            NodeKind::LinearGradient(ref lg) => {
                let mut grad_elem = new_doc.create_element(EId::LinearGradient);
                defs.append(grad_elem.clone());

                grad_elem.set_id(lg.id.clone());

                grad_elem.set_attribute((AId::X1, lg.x1));
                grad_elem.set_attribute((AId::Y1, lg.y1));
                grad_elem.set_attribute((AId::X2, lg.x2));
                grad_elem.set_attribute((AId::Y2, lg.y2));

                conv_base_grad(&lg.base, new_doc, &mut grad_elem);
            }
            NodeKind::RadialGradient(ref rg) => {
                let mut grad_elem = new_doc.create_element(EId::RadialGradient);
                defs.append(grad_elem.clone());

                grad_elem.set_id(rg.id.clone());

                grad_elem.set_attribute((AId::Cx, rg.cx));
                grad_elem.set_attribute((AId::Cy, rg.cy));
                grad_elem.set_attribute((AId::R,  rg.r.value()));
                grad_elem.set_attribute((AId::Fx, rg.fx));
                grad_elem.set_attribute((AId::Fy, rg.fy));

                conv_base_grad(&rg.base, new_doc, &mut grad_elem);
            }
            NodeKind::ClipPath(ref clip) => {
                let mut clip_elem = new_doc.create_element(EId::ClipPath);
                defs.append(clip_elem.clone());

                clip_elem.set_id(clip.id.clone());
                conv_units(AId::ClipPathUnits, clip.units, &mut clip_elem);
                conv_transform(AId::Transform, &clip.transform, &mut clip_elem);

                if let Some(ref id) = clip.clip_path {
                    link_clip_later.push((id.clone(), clip_elem.clone()));
                }

                later_nodes.push((n.clone(), clip_elem.clone()));
            }
            NodeKind::Mask(ref mask) => {
                let mut mask_elem = new_doc.create_element(EId::Mask);
                defs.append(mask_elem.clone());

                mask_elem.set_id(mask.id.clone());
                conv_units(AId::MaskUnits, mask.units, &mut mask_elem);
                conv_units(AId::MaskContentUnits, mask.content_units, &mut mask_elem);
                conv_rect(mask.rect, &mut mask_elem);

                if let Some(ref id) = mask.mask {
                    link_mask_later.push((id.clone(), mask_elem.clone()));
                }

                later_nodes.push((n.clone(), mask_elem.clone()));
            }
            NodeKind::Pattern(ref pattern) => {
                let mut pattern_elem = new_doc.create_element(EId::Pattern);
                defs.append(pattern_elem.clone());

                pattern_elem.set_id(pattern.id.clone());

                conv_rect(pattern.rect, &mut pattern_elem);

                if let Some(vbox) = pattern.view_box {
                    conv_viewbox(&vbox, &mut pattern_elem);
                }

                conv_units(AId::PatternUnits, pattern.units, &mut pattern_elem);
                conv_units(AId::PatternContentUnits, pattern.content_units, &mut pattern_elem);
                conv_transform(AId::PatternTransform, &pattern.transform, &mut pattern_elem);
                later_nodes.push((n.clone(), pattern_elem.clone()));
            }
            NodeKind::Filter(ref filter) => {
                let mut filter_elem = new_doc.create_element(EId::Filter);
                defs.append(filter_elem.clone());

                filter_elem.set_id(filter.id.clone());

                conv_rect(filter.rect, &mut filter_elem);

                conv_units(AId::FilterUnits, filter.units, &mut filter_elem);
                conv_units(AId::PrimitiveUnits, filter.primitive_units, &mut filter_elem);

                for fe in &filter.children {
                    let mut fe_elem = match fe.kind {
                        FilterKind::FeGaussianBlur(ref blur) => {
                            let mut fe_elem = new_doc.create_element(EId::FeGaussianBlur);
                            filter_elem.append(fe_elem.clone());

                            let std_dev = NumberList(vec![
                                blur.std_dev_x.value(),
                                blur.std_dev_y.value()
                            ]);
                            fe_elem.set_attribute((AId::StdDeviation, std_dev));

                            fe_elem.set_attribute((AId::In, blur.input.to_string()));

                            fe_elem
                        }
                        FilterKind::FeOffset(ref offset) => {
                            let mut fe_elem = new_doc.create_element(EId::FeOffset);
                            filter_elem.append(fe_elem.clone());

                            fe_elem.set_attribute((AId::Dx, offset.dx));
                            fe_elem.set_attribute((AId::Dy, offset.dy));

                            fe_elem.set_attribute((AId::In, offset.input.to_string()));

                            fe_elem
                        }
                        FilterKind::FeBlend(ref blend) => {
                            let mut fe_elem = new_doc.create_element(EId::FeBlend);
                            filter_elem.append(fe_elem.clone());

                            fe_elem.set_attribute((AId::Mode, blend.mode.to_string()));
                            fe_elem.set_attribute((AId::In, blend.input1.to_string()));
                            fe_elem.set_attribute((AId::In2, blend.input2.to_string()));

                            fe_elem
                        }
                        FilterKind::FeFlood(ref flood) => {
                            let mut fe_elem = new_doc.create_element(EId::FeFlood);
                            filter_elem.append(fe_elem.clone());

                            fe_elem.set_attribute((AId::FloodColor, flood.color));
                            fe_elem.set_attribute((AId::FloodOpacity, flood.opacity.value()));

                            fe_elem
                        }
                        FilterKind::FeComposite(ref composite) => {
                            let mut fe_elem = new_doc.create_element(EId::FeComposite);
                            filter_elem.append(fe_elem.clone());

                            fe_elem.set_attribute((AId::Operator, composite.operator.to_string()));

                            fe_elem.set_attribute((AId::In, composite.input1.to_string()));
                            fe_elem.set_attribute((AId::In2, composite.input2.to_string()));

                            fe_elem
                        }
                        FilterKind::FeMerge(ref merge) => {
                            let mut fe_elem = new_doc.create_element(EId::FeMerge);
                            filter_elem.append(fe_elem.clone());

                            for input in &merge.inputs {
                                let mut child_elem = new_doc.create_element(EId::FeMergeNode);
                                fe_elem.append(child_elem.clone());

                                child_elem.set_attribute((AId::In, input.to_string()));
                            }

                            fe_elem
                        }
                        FilterKind::FeTile(ref tile) => {
                            let mut fe_elem = new_doc.create_element(EId::FeTile);
                            filter_elem.append(fe_elem.clone());

                            fe_elem.set_attribute((AId::In, tile.input.to_string()));

                            fe_elem
                        }
                        FilterKind::FeImage(ref img) => {
                            let mut fe_elem = new_doc.create_element(EId::FeImage);
                            filter_elem.append(fe_elem.clone());

                            match img.data {
                                FeImageKind::None => {}
                                FeImageKind::Image(ref data, format) => {
                                    let href = conv_image_data(data, format);
                                    fe_elem.set_attribute((AId::Href, href));
                                }
                                FeImageKind::Use(..) => {}
                            }

                            fe_elem.set_attribute((AId::PreserveAspectRatio, img.aspect));

                            fe_elem
                        }
                    };

                    if let Some(n) = fe.x { fe_elem.set_attribute((AId::X, n)); }
                    if let Some(n) = fe.y { fe_elem.set_attribute((AId::Y, n)); }
                    if let Some(n) = fe.width { fe_elem.set_attribute((AId::Width, n)); }
                    if let Some(n) = fe.height { fe_elem.set_attribute((AId::Height, n)); }

                    fe_elem.set_attribute(
                        (AId::ColorInterpolationFilters, fe.color_interpolation.to_string())
                    );

                    fe_elem.set_attribute((AId::Result, fe.result.as_str()));
                }
            }
            _ => {}
        }
    }

    for (id, mut elem) in link_clip_later {
        conv_link(tree, defs, AId::ClipPath, &id, &mut elem);
    }

    for (id, mut elem) in link_mask_later {
        conv_link(tree, defs, AId::Mask, &id, &mut elem);
    }

    for (rnode, mut elem) in later_nodes {
        conv_elements(tree, &rnode, defs, new_doc, &mut elem);
    }
}

fn conv_elements(
    tree: &Tree,
    root: &Node,
    defs: &svgdom::Node,
    new_doc: &mut svgdom::Document,
    parent: &mut svgdom::Node,
) {
    for n in root.children() {
        match *n.borrow() {
            NodeKind::Path(ref p) => {
                let mut path_elem = new_doc.create_element(EId::Path);
                parent.append(path_elem.clone());

                conv_transform(AId::Transform, &p.transform, &mut path_elem);
                conv_visibility(p.visibility, &mut path_elem);
                path_elem.set_id(p.id.clone());

                use svgdom::Path as SvgDomPath;
                use svgdom::PathSegment as SvgDomPathSegment;

                let mut path = SvgDomPath::with_capacity(p.segments.len());
                for seg in &p.segments {
                    match *seg {
                        PathSegment::MoveTo { x, y } => {
                            path.push(SvgDomPathSegment::MoveTo { abs: true, x, y });
                        }
                        PathSegment::LineTo { x, y } => {
                            path.push(SvgDomPathSegment::LineTo { abs: true, x, y });
                        }
                        PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                            path.push(SvgDomPathSegment::CurveTo { abs: true, x1, y1, x2, y2, x, y });
                        }
                        PathSegment::ClosePath => {
                            path.push(SvgDomPathSegment::ClosePath { abs: true });
                        }
                    }
                }

                path_elem.set_attribute((AId::D, path));

                conv_fill(tree, &p.fill, defs, parent, &mut path_elem);
                conv_stroke(tree, &p.stroke, defs, &mut path_elem);
            }
            NodeKind::Text(ref text) => {
                let mut text_elem = new_doc.create_element(EId::Text);
                parent.append(text_elem.clone());

                conv_transform(AId::Transform, &text.transform, &mut text_elem);
                text_elem.set_id(text.id.clone());


                if let Some(ref rotate) = text.rotate {
                    text_elem.set_attribute((AId::Rotate, rotate.clone()));
                }

                // conv_text_decoration(&text.decoration, &mut text_elem);

                let mut is_preserve_required = false;

                for chunk in &text.chunks {
                    let mut chunk_tspan_elem = new_doc.create_element(EId::Tspan);
                    text_elem.append(chunk_tspan_elem.clone());

                    if let Some(ref x) = chunk.x {
                        chunk_tspan_elem.set_attribute((AId::X, x.clone()));
                    }

                    if let Some(ref y) = chunk.y {
                        chunk_tspan_elem.set_attribute((AId::Y, y.clone()));
                    }

                    if let Some(ref dx) = chunk.dx {
                        chunk_tspan_elem.set_attribute((AId::Dx, dx.clone()));
                    }

                    if let Some(ref dy) = chunk.dy {
                        chunk_tspan_elem.set_attribute((AId::Dy, dy.clone()));
                    }

                    if chunk.anchor != TextAnchor::Start {
                        chunk_tspan_elem.set_attribute((AId::TextAnchor,
                            match chunk.anchor {
                                TextAnchor::Start => "start",
                                TextAnchor::Middle => "middle",
                                TextAnchor::End => "end",
                            }
                        ));
                    }

                    for tspan in &chunk.spans {
                        let mut tspan_elem = new_doc.create_element(EId::Tspan);
                        chunk_tspan_elem.append(tspan_elem.clone());

                        let text_node = new_doc.create_node(
                            svgdom::NodeType::Text,
                            tspan.text.clone(),
                        );
                        tspan_elem.append(text_node.clone());

                        conv_visibility(tspan.visibility, &mut tspan_elem);
                        conv_fill(tree, &tspan.fill, defs, parent, &mut tspan_elem);
                        conv_stroke(tree, &tspan.stroke, defs, &mut tspan_elem);
                        conv_font(&tspan.font, &mut tspan_elem);
                        conv_baseline_shift(tspan.baseline_shift, &mut tspan_elem);

                        if tspan.text.contains("  ") {
                            is_preserve_required = true;
                        }

                        // TODO: text-decoration
                    }
                }

                if is_preserve_required {
                    text_elem.set_attribute((AId::Space, "preserve"));
                }
            }
            NodeKind::Image(ref img) => {
                let mut img_elem = new_doc.create_element(EId::Image);
                parent.append(img_elem.clone());

                conv_transform(AId::Transform, &img.transform, &mut img_elem);
                conv_visibility(img.visibility, &mut img_elem);
                img_elem.set_id(img.id.clone());
                conv_viewbox2(&img.view_box, &mut img_elem);

                img_elem.set_attribute((AId::Href, conv_image_data(&img.data, img.format)));
            }
            NodeKind::Group(ref g) => {
                let mut g_elem = if parent.is_tag_name(EId::ClipPath) {
                    conv_elements(tree, &n, defs, new_doc, parent);
                    parent.last_child().unwrap()
                } else {
                    let mut g_elem = new_doc.create_element(EId::G);
                    parent.append(g_elem.clone());
                    g_elem
                };

                conv_transform(AId::Transform, &g.transform, &mut g_elem);
                g_elem.set_id(g.id.clone());

                if let Some(ref id) = g.clip_path {
                    conv_link(tree, defs, AId::ClipPath, id, &mut g_elem);
                }

                if let Some(ref id) = g.mask {
                    conv_link(tree, defs, AId::Mask, id, &mut g_elem);
                }

                if let Some(ref id) = g.filter {
                    conv_link(tree, defs, AId::Filter, id, &mut g_elem);
                }

                if let Some(opacity) = g.opacity {
                    g_elem.set_attribute((AId::Opacity, opacity.value()));
                }

                if !g_elem.has_id() && g_elem.attributes().len() == 0 {
                    warn!("Group must have at least one attribute otherwise it's pointless.");
                }

                if !parent.is_tag_name(EId::ClipPath) {
                    conv_elements(tree, &n, defs, new_doc, &mut g_elem);
                }
            }
            _ => {}
        }
    }
}

fn conv_viewbox(
    view_box: &ViewBox,
    node: &mut svgdom::Node,
) {
    let r = view_box.rect;
    let vb = svgdom::ViewBox::new(r.x, r.y, r.width, r.height);
    node.set_attribute((AId::ViewBox, vb));

    node.set_attribute((AId::PreserveAspectRatio, view_box.aspect));
}

fn conv_rect(
    r: Rect,
    node: &mut svgdom::Node,
) {
    node.set_attribute((AId::X, r.x));
    node.set_attribute((AId::Y, r.y));
    node.set_attribute((AId::Width, r.width));
    node.set_attribute((AId::Height, r.height));
}

fn conv_viewbox2(
    vb: &ViewBox,
    node: &mut svgdom::Node,
) {
    conv_rect(vb.rect, node);
    node.set_attribute((AId::PreserveAspectRatio, vb.aspect));
}

fn conv_fill(
    tree: &Tree,
    fill: &Option<Fill>,
    defs: &svgdom::Node,
    parent: &svgdom::Node,
    node: &mut svgdom::Node,
) {
    match *fill {
        Some(ref fill) => {
            match fill.paint {
                Paint::Color(c) => node.set_attribute((AId::Fill, c)),
                Paint::Link(ref id) => conv_link(tree, defs, AId::Fill, id, node),
            }

            node.set_attribute((AId::FillOpacity, fill.opacity.value()));

            let rule = if fill.rule == FillRule::NonZero { "nonzero" } else { "evenodd" };
            let rule_aid = if parent.is_tag_name(EId::ClipPath) {
                AId::ClipRule
            } else {
                AId::FillRule
            };
            node.set_attribute((rule_aid, rule));
        }
        None => {
            node.set_attribute((AId::Fill, AValue::None));
        }
    }
}

fn conv_stroke(
    tree: &Tree,
    stroke: &Option<Stroke>,
    defs: &svgdom::Node,
    node: &mut svgdom::Node,
) {
    match *stroke {
        Some(ref stroke) => {
            match stroke.paint {
                Paint::Color(c) => node.set_attribute((AId::Stroke, c)),
                Paint::Link(ref id) => conv_link(tree, defs, AId::Stroke, id, node),
            }

            node.set_attribute((AId::StrokeOpacity, stroke.opacity.value()));
            node.set_attribute((AId::StrokeDashoffset, stroke.dashoffset));
            node.set_attribute((AId::StrokeMiterlimit, stroke.miterlimit.value()));
            node.set_attribute((AId::StrokeWidth, stroke.width.value()));

            node.set_attribute((AId::StrokeLinecap,
                match stroke.linecap {
                    LineCap::Butt => "butt",
                    LineCap::Round => "round",
                    LineCap::Square => "square",
                }
            ));

            node.set_attribute((AId::StrokeLinejoin,
                match stroke.linejoin {
                    LineJoin::Miter => "miter",
                    LineJoin::Round => "round",
                    LineJoin::Bevel => "bevel",
                }
            ));

            if let Some(ref array) = stroke.dasharray {
                node.set_attribute((AId::StrokeDasharray, array.clone()));
            } else {
                node.set_attribute((AId::StrokeDasharray, AValue::None));
            }
        }
        None => {
            node.set_attribute((AId::Stroke, AValue::None));
        }
    }
}

fn conv_base_grad(
    g: &BaseGradient,
    doc: &mut svgdom::Document,
    node: &mut svgdom::Node,
) {
    conv_units(AId::GradientUnits, g.units, node);

    node.set_attribute((AId::SpreadMethod,
        match g.spread_method {
            SpreadMethod::Pad => "pad",
            SpreadMethod::Reflect => "reflect",
            SpreadMethod::Repeat => "repeat",
        }
    ));

    conv_transform(AId::GradientTransform, &g.transform, node);

    for s in &g.stops {
        let mut stop = doc.create_element(EId::Stop);
        node.append(stop.clone());

        stop.set_attribute((AId::Offset, s.offset.value()));
        stop.set_attribute((AId::StopColor, s.color));
        stop.set_attribute((AId::StopOpacity, s.opacity.value()));
    }
}

fn conv_units(
    aid: AId,
    units: Units,
    node: &mut svgdom::Node,
) {
    node.set_attribute((aid,
        match units {
            Units::UserSpaceOnUse => "userSpaceOnUse",
            Units::ObjectBoundingBox => "objectBoundingBox",
        }
    ));
}

fn conv_transform(
    aid: AId,
    ts: &svgdom::Transform,
    node: &mut svgdom::Node,
) {
    if !ts.is_default() {
        node.set_attribute((aid, *ts));
    }
}

fn conv_font(
    font: &Font,
    node: &mut svgdom::Node,
) {
    node.set_attribute((AId::FontFamily, font.family.clone()));
    node.set_attribute((AId::FontSize, font.size.value()));

    node.set_attribute((AId::FontStyle,
        match font.style {
            FontStyle::Normal => "normal",
            FontStyle::Italic => "italic",
            FontStyle::Oblique => "oblique",
        }
    ));

    node.set_attribute((AId::FontVariant,
        match font.variant {
            FontVariant::Normal => "normal",
            FontVariant::SmallCaps => "small-caps",
        }
    ));

    node.set_attribute((AId::FontWeight,
        match font.weight {
            FontWeight::W100 => "100",
            FontWeight::W200 => "200",
            FontWeight::W300 => "300",
            FontWeight::W400 => "400",
            FontWeight::W500 => "500",
            FontWeight::W600 => "600",
            FontWeight::W700 => "700",
            FontWeight::W800 => "800",
            FontWeight::W900 => "900",
        }
    ));

    node.set_attribute((AId::FontStretch,
        match font.stretch {
            FontStretch::Normal => "normal",
            FontStretch::Wider => "wider",
            FontStretch::Narrower => "narrower",
            FontStretch::UltraCondensed => "ultra-condensed",
            FontStretch::ExtraCondensed => "extra-condensed",
            FontStretch::Condensed => "condensed",
            FontStretch::SemiCondensed => "semi-condensed",
            FontStretch::SemiExpanded => "semi-expanded",
            FontStretch::Expanded => "expanded",
            FontStretch::ExtraExpanded => "extra-expanded",
            FontStretch::UltraExpanded => "ultra-expanded",
        }
    ));

    conv_text_spacing(font.letter_spacing, AId::LetterSpacing, node);
    conv_text_spacing(font.word_spacing, AId::WordSpacing, node);
}

fn conv_baseline_shift(
    baseline_shift: BaselineShift,
    node: &mut svgdom::Node,
) {
    let av: AValue = match baseline_shift {
        BaselineShift::Baseline => "baseline".into(),
        BaselineShift::Subscript => "sub".into(),
        BaselineShift::Superscript => "super".into(),
        BaselineShift::Percent(n) => svgdom::Length::new(n, svgdom::LengthUnit::Percent).into(),
        BaselineShift::Number(n) => n.into(),
    };

    node.set_attribute((AId::BaselineShift, av));
}

fn conv_text_spacing(
    spacing: Option<f64>,
    aid: AId,
    node: &mut svgdom::Node,
) {
    let spacing: AValue = match spacing {
        Some(n) => n.into(),
        None => "normal".into(),
    };

    node.set_attribute((aid, spacing));
}

fn conv_link(
    tree: &Tree,
    defs: &svgdom::Node,
    aid: AId,
    id: &str,
    node: &mut svgdom::Node,
) {
    if let Some(n) = tree.defs_by_id(id) {
        let defs_id = n.id();
        let link = match defs.children().find(|n| *n.id() == *defs_id) {
            Some(v) => v,
            None => {
                debug_panic!("unresolved FuncLink '{}'", defs_id);
                return;
            }
        };
        node.set_attribute((aid, link));
    }
}

fn conv_visibility(
    value: Visibility,
    node: &mut svgdom::Node,
) {
    let s = match value {
        Visibility::Visible     => "visible",
        Visibility::Hidden      => "hidden",
        Visibility::Collapse    => "collapse",
    };

    node.set_attribute((AId::Visibility, s));
}

fn conv_image_data(
    data: &ImageData,
    format: ImageFormat,
) -> String {
    match data {
        ImageData::Path(ref path) => path.to_str().unwrap().to_owned(),
        ImageData::Raw(ref data) => {
            let mut d = String::with_capacity(data.len() + 20);

            d.push_str("data:image/");
            match format {
                ImageFormat::PNG => d.push_str("png"),
                ImageFormat::JPEG => d.push_str("jpg"),
                ImageFormat::SVG => d.push_str("svg+xml"),
            }
            d.push_str(";base64, ");
            d.push_str(&base64::encode(data));

            d
        }
    }
}
