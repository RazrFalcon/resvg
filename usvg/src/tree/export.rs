// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use base64;
use svgdom;

// self
use super::*;
use traits::IsDefault;
use geom::*;
use short::{
    AId,
    AValue,
    EId,
};


pub fn convert(tree: &Tree) -> svgdom::Document {
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
    let mut link_later = Vec::new();

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

                conv_units(AId::ClipPathUnits, clip.units, Units::UserSpaceOnUse, &mut clip_elem);

                conv_transform(AId::Transform, &clip.transform, &mut clip_elem);

                if let Some(ref id) = clip.clip_path {
                    link_later.push((id.clone(), AId::ClipPath, clip_elem.clone()));
                }

                later_nodes.push((n.clone(), clip_elem.clone()));
            }
            NodeKind::Mask(ref mask) => {
                let mut mask_elem = new_doc.create_element(EId::Mask);
                defs.append(mask_elem.clone());

                mask_elem.set_id(mask.id.clone());
                conv_units(AId::MaskUnits, mask.units, Units::ObjectBoundingBox, &mut mask_elem);
                conv_units(AId::MaskContentUnits, mask.content_units, Units::UserSpaceOnUse, &mut mask_elem);
                conv_rect(mask.rect, &mut mask_elem);

                if let Some(ref id) = mask.mask {
                    link_later.push((id.clone(), AId::Mask, mask_elem.clone()));
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

                conv_units(AId::PatternUnits, pattern.units, Units::ObjectBoundingBox, &mut pattern_elem);
                conv_units(AId::PatternContentUnits, pattern.content_units, Units::UserSpaceOnUse, &mut pattern_elem);

                conv_transform(AId::PatternTransform, &pattern.transform, &mut pattern_elem);
                later_nodes.push((n.clone(), pattern_elem.clone()));
            }
            NodeKind::Filter(ref filter) => {
                let mut filter_elem = new_doc.create_element(EId::Filter);
                defs.append(filter_elem.clone());

                filter_elem.set_id(filter.id.clone());

                conv_rect(filter.rect, &mut filter_elem);

                conv_units(AId::FilterUnits, filter.units, Units::ObjectBoundingBox, &mut filter_elem);
                conv_units(AId::PrimitiveUnits, filter.primitive_units, Units::UserSpaceOnUse, &mut filter_elem);

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

    for (id, aid, mut elem) in link_later {
        conv_link(tree, defs, aid, &id, &mut elem);
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
                path_elem.set_enum_attribute(AId::Visibility, p.visibility);
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

                    chunk_tspan_elem.set_enum_attribute(AId::TextAnchor, chunk.anchor);

                    for tspan in &chunk.spans {
                        let mut tspan_elem = new_doc.create_element(EId::Tspan);
                        chunk_tspan_elem.append(tspan_elem.clone());

                        let text_node = new_doc.create_node(
                            svgdom::NodeType::Text,
                            tspan.text.clone(),
                        );
                        tspan_elem.append(text_node.clone());

                        tspan_elem.set_enum_attribute(AId::Visibility, tspan.visibility);

                        conv_fill(tree, &tspan.fill, defs, parent, &mut tspan_elem);
                        conv_stroke(tree, &tspan.stroke, defs, &mut tspan_elem);
                        conv_font(&tspan.font, &mut tspan_elem);

                        if !tspan.baseline_shift.is_fuzzy_zero() {
                            tspan_elem.set_attribute((AId::BaselineShift, tspan.baseline_shift));
                        }

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
                img_elem.set_enum_attribute(AId::Visibility, img.visibility);
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

                conv_opt_link(tree, defs, AId::ClipPath, &g.clip_path, &mut g_elem);
                conv_opt_link(tree, defs, AId::Mask, &g.mask, &mut g_elem);
                conv_opt_link(tree, defs, AId::Filter, &g.filter, &mut g_elem);

                if !g.opacity.is_default() {
                    g_elem.set_attribute((AId::Opacity, g.opacity.value()));
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

    if !view_box.aspect.is_default() {
        node.set_attribute((AId::PreserveAspectRatio, view_box.aspect));
    }
}

fn conv_viewbox2(
    view_box: &ViewBox,
    node: &mut svgdom::Node,
) {
    conv_rect(view_box.rect, node);

    if !view_box.aspect.is_default() {
        node.set_attribute((AId::PreserveAspectRatio, view_box.aspect));
    }
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

fn conv_units(
    aid: AId,
    units: Units,
    def: Units,
    node: &mut svgdom::Node,
) {
    if units != def {
        node.set_attribute((aid, units.to_string()));
    }
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
                Paint::Color(c) => {
                    if c != Color::black() {
                        node.set_attribute((AId::Fill, c))
                    }
                }
                Paint::Link(ref id) => {
                    conv_link(tree, defs, AId::Fill, id, node)
                }
            }

            if !fill.opacity.is_default() {
                node.set_attribute((AId::FillOpacity, fill.opacity.value()));
            }

            if !fill.rule.is_default() {
                let rule_aid = if parent.is_tag_name(EId::ClipPath) {
                    AId::ClipRule
                } else {
                    AId::FillRule
                };

                node.set_attribute((rule_aid, fill.rule.to_string()));
            }
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
    if let Some(ref stroke) = stroke {
        match stroke.paint {
            Paint::Color(c) => node.set_attribute((AId::Stroke, c)),
            Paint::Link(ref id) => conv_link(tree, defs, AId::Stroke, id, node),
        }

        if !stroke.opacity.is_default() {
            node.set_attribute((AId::StrokeOpacity, stroke.opacity.value()));
        }

        if !(stroke.dashoffset as f64).is_fuzzy_zero() {
            node.set_attribute((AId::StrokeDashoffset, stroke.dashoffset as f64));
        }

        if !stroke.miterlimit.is_default() {
            node.set_attribute((AId::StrokeMiterlimit, stroke.miterlimit.value()));
        }

        if !stroke.width.is_default() {
            node.set_attribute((AId::StrokeWidth, stroke.width.value()));
        }

        node.set_enum_attribute(AId::StrokeLinecap, stroke.linecap);
        node.set_enum_attribute(AId::StrokeLinejoin, stroke.linejoin);

        if let Some(ref array) = stroke.dasharray {
            node.set_attribute((AId::StrokeDasharray, array.clone()));
        }
    }
}

fn conv_base_grad(
    g: &BaseGradient,
    doc: &mut svgdom::Document,
    node: &mut svgdom::Node,
) {
    if g.units != Units::ObjectBoundingBox {
        node.set_attribute((AId::GradientUnits, g.units.to_string()));
    }

    node.set_enum_attribute(AId::SpreadMethod, g.spread_method);

    conv_transform(AId::GradientTransform, &g.transform, node);

    for s in &g.stops {
        let mut stop = doc.create_element(EId::Stop);
        node.append(stop.clone());

        stop.set_attribute((AId::Offset, s.offset.value()));
        stop.set_attribute((AId::StopColor, s.color));

        if !s.opacity.is_default() {
            stop.set_attribute((AId::StopOpacity, s.opacity.value()));
        }
    }
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
    node.set_enum_attribute(AId::FontStyle, font.style);
    node.set_enum_attribute(AId::FontVariant, font.variant);
    node.set_enum_attribute(AId::FontWeight, font.weight);
    node.set_enum_attribute(AId::FontStretch, font.stretch);
    conv_text_spacing(font.letter_spacing, AId::LetterSpacing, node);
    conv_text_spacing(font.word_spacing, AId::WordSpacing, node);
}

fn conv_text_spacing(
    spacing: Option<f64>,
    aid: AId,
    node: &mut svgdom::Node,
) {
    if let Some(spacing) = spacing {
        node.set_attribute((aid, spacing));
    }
}

fn conv_opt_link(
    tree: &Tree,
    defs: &svgdom::Node,
    aid: AId,
    id: &Option<String>,
    node: &mut svgdom::Node,
) {
    if let Some(id) = id {
        conv_link(tree, defs, aid, id, node);
    }
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


// TODO: find a way to do this for numbers too
trait SetEnumAttribute<T> {
    fn set_enum_attribute(&mut self, aid: AId, value: T);
}

impl<T> SetEnumAttribute<T> for svgdom::Node
    where T: IsDefault + ToString
{
    fn set_enum_attribute(&mut self, aid: AId, value: T) {
        if !value.is_default() {
            self.set_attribute((aid, value.to_string()));
        }
    }
}
