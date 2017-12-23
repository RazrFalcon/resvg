// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use base64;
use svgdom;

use svgdom::types::{
    FuzzyEq,
};

use super::*;

use short::{
    AId,
    EId,
};


// TODO: xml:space

pub fn conv_doc(doc: &Document) -> svgdom::Document {
    let mut new_doc = svgdom::Document::new();

    let mut svg = new_doc.create_element(EId::Svg);
    new_doc.append(&svg);

    let view_box = format!("{} {} {} {}", doc.view_box.x, doc.view_box.y,
                                          doc.view_box.w, doc.view_box.h);

    svg.set_attribute((AId::Xmlns, "http://www.w3.org/2000/svg"));
    if !doc.defs.is_empty() {
        svg.set_attribute((AId::XmlnsXlink, "http://www.w3.org/1999/xlink"));
    }
    svg.set_attribute((AId::Width,  doc.size.w));
    svg.set_attribute((AId::Height, doc.size.h));
    svg.set_attribute((AId::ViewBox, view_box));
    svg.set_attribute(("xmlns:resvg", "https://github.com/RazrFalcon/libresvg"));
    svg.set_attribute(("resvg:version", env!("CARGO_PKG_VERSION")));

    let mut defs_list = Vec::new();

    if !doc.defs.is_empty() {
        let mut defs = new_doc.create_element(EId::Defs);
        svg.append(&defs);

        for e in &doc.defs {
            match e.kind {
                element::RefElementKind::LinearGradient(ref lg) => {
                    let mut grad = new_doc.create_element(EId::LinearGradient);
                    defs.append(&grad);
                    defs_list.push(grad.clone());

                    grad.set_id(e.id.clone());

                    grad.set_attribute((AId::X1, lg.x1));
                    grad.set_attribute((AId::Y1, lg.y1));
                    grad.set_attribute((AId::X2, lg.x2));
                    grad.set_attribute((AId::Y2, lg.y2));

                    conv_base_grad(&lg.d, &mut new_doc, &mut grad);
                }
                element::RefElementKind::RadialGradient(ref rg) => {
                    let mut grad = new_doc.create_element(EId::RadialGradient);
                    defs.append(&grad);
                    defs_list.push(grad.clone());

                    grad.set_id(e.id.clone());

                    grad.set_attribute((AId::Cx, rg.cx));
                    grad.set_attribute((AId::Cy, rg.cy));
                    grad.set_attribute((AId::R,  rg.r));
                    grad.set_attribute((AId::Fx, rg.fx));
                    grad.set_attribute((AId::Fy, rg.fy));

                    conv_base_grad(&rg.d, &mut new_doc, &mut grad);
                }
                element::RefElementKind::ClipPath(ref c) => {
                    let mut clip = new_doc.create_element(EId::ClipPath);
                    defs.append(&clip);
                    defs_list.push(clip.clone());

                    clip.set_id(e.id.clone());
                    conv_units(AId::ClipPathUnits, c.units, &mut clip);
                    conv_transform(AId::Transform, &c.transform, &mut clip);
                    conv_elements(&c.children, &[], &mut new_doc, &mut clip);
                }
            }
        }
    }

    conv_elements(&doc.elements, &defs_list, &mut new_doc, &mut svg);

    new_doc
}

fn conv_elements(
    elements: &[Element],
    defs_list: &[svgdom::Node],
    new_doc: &mut svgdom::Document,
    parent: &mut svgdom::Node,
) {
    let base64_conf = base64::Config::new(
        base64::CharacterSet::Standard,
        true,
        true,
        base64::LineWrap::Wrap(64, base64::LineEnding::LF),
    );

    for e in elements {
        match e.kind {
            element::ElementKind::Path(ref p) => {
                let mut path_elem = new_doc.create_element(EId::Path);
                parent.append(&path_elem);

                conv_element(e, &mut path_elem);

                use svgdom::types::path::Path as SvgDomPath;
                use svgdom::types::path::Segment;

                let mut path = SvgDomPath::with_capacity(p.d.len());
                for seg in &p.d {
                    match *seg {
                        PathSegment::MoveTo { x, y } => {
                            path.d.push(Segment::new_move_to(x, y));
                        }
                        PathSegment::LineTo { x, y } => {
                            path.d.push(Segment::new_line_to(x, y));
                        }
                        PathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                            path.d.push(Segment::new_curve_to(x1, y1, x2, y2, x, y));
                        }
                        PathSegment::ClosePath => {
                            path.d.push(Segment::new_close_path());
                        }
                    }
                }

                path_elem.set_attribute((AId::D, path));

                conv_fill(&p.fill, parent, &defs_list, &mut path_elem);
                conv_stroke(&p.stroke, &defs_list, &mut path_elem);
            }
            element::ElementKind::Text(ref text) => {
                let mut text_elem = new_doc.create_element(EId::Text);
                parent.append(&text_elem);

                conv_element(e, &mut text_elem);

                // conv_text_decoration(&text.decoration, &mut text_elem);

                for chunk in &text.children {
                    let mut chunk_tspan_elem = new_doc.create_element(EId::Tspan);
                    text_elem.append(&chunk_tspan_elem);

                    chunk_tspan_elem.set_attribute((AId::X, chunk.x.clone()));
                    chunk_tspan_elem.set_attribute((AId::Y, chunk.y.clone()));

                    if chunk.anchor != TextAnchor::Start {
                        chunk_tspan_elem.set_attribute((AId::TextAnchor,
                            match chunk.anchor {
                                TextAnchor::Start => svgdom::ValueId::Start,
                                TextAnchor::Middle => svgdom::ValueId::Middle,
                                TextAnchor::End => svgdom::ValueId::End,
                            }
                        ));
                    }

                    for tspan in &chunk.children {
                        let mut tspan_elem = new_doc.create_element(EId::Tspan);
                        chunk_tspan_elem.append(&tspan_elem);

                        let text_node = new_doc.create_node(svgdom::NodeType::Text, &tspan.text);
                        tspan_elem.append(&text_node);

                        conv_fill(&tspan.fill, parent, &defs_list, &mut tspan_elem);
                        conv_stroke(&tspan.stroke, &defs_list, &mut tspan_elem);
                        conv_font(&tspan.font, &mut tspan_elem);

                        // TODO: text-decoration
                    }
                }
            }
            element::ElementKind::Image(ref img) => {
                let mut img_elem = new_doc.create_element(EId::Image);
                parent.append(&img_elem);

                conv_element(e, &mut img_elem);

                img_elem.set_attribute((AId::X, img.rect.x));
                img_elem.set_attribute((AId::Y, img.rect.y));
                img_elem.set_attribute((AId::Width, img.rect.w));
                img_elem.set_attribute((AId::Height, img.rect.h));

                let href = match img.data {
                    ImageData::Path(ref path) => path.to_str().unwrap().to_owned(),
                    ImageData::Raw(ref data, kind) => {
                        let mut d = String::with_capacity(data.len() + 20);

                        d.push_str("data:image/");
                        match kind {
                            ImageDataKind::PNG => d.push_str("png"),
                            ImageDataKind::JPEG => d.push_str("jpg"),
                        }
                        d.push_str(";base64,\n");
                        d.push_str(&base64::encode_config(data, base64_conf));

                        d
                    }
                };

                img_elem.set_attribute((AId::XlinkHref, href));
            }
            element::ElementKind::Group(ref g) => {
                let mut g_elem = new_doc.create_element(EId::G);
                parent.append(&g_elem);

                conv_element(e, &mut g_elem);

                if let Some(id) = g.clip_path {
                    g_elem.set_attribute((AId::ClipPath, defs_list[id].clone()));
                }

                if let Some(opacity) = g.opacity {
                    if opacity.fuzzy_ne(&1.0) {
                        g_elem.set_attribute((AId::Opacity, opacity));
                    }
                }

                conv_elements(&g.children, &defs_list, new_doc, &mut g_elem);
            }
        }
    }
}

fn conv_element(elem: &Element, node: &mut svgdom::Node) {
    conv_transform(AId::Transform, &elem.transform, node);

    if !elem.id.is_empty() {
        node.set_id(elem.id.clone());
    }
}

fn conv_fill(
    fill: &Option<Fill>,
    parent: &svgdom::Node,
    defs_list: &[svgdom::Node],
    node: &mut svgdom::Node
) {
    match *fill {
        Some(ref fill) => {
            match fill.paint {
                Paint::Color(c) => node.set_attribute((AId::Fill, c)),
                Paint::Link(id) => node.set_attribute((AId::Fill, defs_list[id].clone())),
            }

            if fill.opacity.fuzzy_ne(&1.0) {
                node.set_attribute((AId::FillOpacity, fill.opacity));
            }

            if fill.rule != FillRule::NonZero {
                if parent.is_tag_name(EId::ClipPath) {
                    node.set_attribute((AId::ClipRule, svgdom::ValueId::Evenodd));
                } else {
                    node.set_attribute((AId::FillRule, svgdom::ValueId::Evenodd));
                }
            }
        }
        None => {
            node.set_attribute((AId::Fill, svgdom::ValueId::None));
        }
    }
}

fn conv_stroke(stroke: &Option<Stroke>, defs_list: &[svgdom::Node], node: &mut svgdom::Node) {
    match *stroke {
        Some(ref stroke) => {
            match stroke.paint {
                Paint::Color(c) => node.set_attribute((AId::Stroke, c)),
                Paint::Link(id) => node.set_attribute((AId::Stroke, defs_list[id].clone())),
            }

            if stroke.opacity.fuzzy_ne(&1.0) {
                node.set_attribute((AId::StrokeOpacity, stroke.opacity));
            }

            if stroke.dashoffset.fuzzy_ne(&0.0) {
                node.set_attribute((AId::StrokeDashoffset, stroke.dashoffset));
            }

            if stroke.miterlimit.fuzzy_ne(&4.0) {
                node.set_attribute((AId::StrokeMiterlimit, stroke.miterlimit));
            }

            if stroke.width.fuzzy_ne(&1.0) {
                node.set_attribute((AId::StrokeWidth, stroke.width));
            }

            if stroke.linecap != LineCap::Butt {
                node.set_attribute((AId::StrokeLinecap,
                    match stroke.linecap {
                        LineCap::Butt => svgdom::ValueId::Butt,
                        LineCap::Round => svgdom::ValueId::Round,
                        LineCap::Square => svgdom::ValueId::Square,
                    }
                ));
            }

            if stroke.linejoin != LineJoin::Miter {
                node.set_attribute((AId::StrokeLinejoin,
                    match stroke.linejoin {
                        LineJoin::Miter => svgdom::ValueId::Miter,
                        LineJoin::Round => svgdom::ValueId::Round,
                        LineJoin::Bevel => svgdom::ValueId::Bevel,
                    }
                ));
            }

            if let Some(ref array) = stroke.dasharray {
                node.set_attribute((AId::StrokeDasharray, array.clone()));
            }
        }
        None => {
            node.set_attribute((AId::Stroke, svgdom::ValueId::None));
        }
    }
}

fn conv_base_grad(g: &element::BaseGradient, doc: &mut svgdom::Document, node: &mut svgdom::Node) {
    conv_units(AId::GradientUnits, g.units, node);

    node.set_attribute((AId::SpreadMethod,
        match g.spread_method {
            SpreadMethod::Pad => svgdom::ValueId::Pad,
            SpreadMethod::Reflect => svgdom::ValueId::Reflect,
            SpreadMethod::Repeat => svgdom::ValueId::Repeat,
        }
    ));

    conv_transform(AId::GradientTransform, &g.transform, node);

    for s in &g.stops {
        let mut stop = doc.create_element(EId::Stop);
        node.append(&stop);

        stop.set_attribute((AId::Offset, s.offset));
        stop.set_attribute((AId::StopColor, s.color));
        stop.set_attribute((AId::StopOpacity, s.opacity));
    }
}

fn conv_units(aid: AId, units: Units, node: &mut svgdom::Node) {
    node.set_attribute((aid,
        match units {
            Units::UserSpaceOnUse => svgdom::ValueId::UserSpaceOnUse,
            Units::ObjectBoundingBox => svgdom::ValueId::ObjectBoundingBox,
        }
    ));
}

fn conv_transform(aid: AId, ts: &svgdom::types::Transform, node: &mut svgdom::Node) {
    if !ts.is_default() {
        node.set_attribute((aid, *ts));
    }
}

fn conv_font(font: &Font, node: &mut svgdom::Node) {
    node.set_attribute((AId::FontFamily, font.family.clone()));
    node.set_attribute((AId::FontSize, font.size));

    if font.style != FontStyle::Normal {
        node.set_attribute((AId::FontStyle,
            match font.style {
                FontStyle::Normal => svgdom::ValueId::Normal,
                FontStyle::Italic => svgdom::ValueId::Italic,
                FontStyle::Oblique => svgdom::ValueId::Oblique,
            }
        ));
    }

    if font.variant != FontVariant::Normal {
        node.set_attribute((AId::FontVariant,
            match font.variant {
                FontVariant::Normal => svgdom::ValueId::Normal,
                FontVariant::SmallCaps => svgdom::ValueId::SmallCaps,
            }
        ));
    }

    if font.weight != FontWeight::Normal {
        node.set_attribute((AId::FontWeight,
            match font.weight {
                FontWeight::Normal => svgdom::ValueId::Normal,
                FontWeight::Bold => svgdom::ValueId::Bold,
                FontWeight::Bolder => svgdom::ValueId::Bolder,
                FontWeight::Lighter => svgdom::ValueId::Lighter,
                FontWeight::W100 => svgdom::ValueId::N100,
                FontWeight::W200 => svgdom::ValueId::N200,
                FontWeight::W300 => svgdom::ValueId::N300,
                FontWeight::W400 => svgdom::ValueId::N400,
                FontWeight::W500 => svgdom::ValueId::N500,
                FontWeight::W600 => svgdom::ValueId::N600,
                FontWeight::W700 => svgdom::ValueId::N700,
                FontWeight::W800 => svgdom::ValueId::N800,
                FontWeight::W900 => svgdom::ValueId::N900,
            }
        ));
    }

    if font.stretch != FontStretch::Normal {
        node.set_attribute((AId::FontStretch,
            match font.stretch {
                FontStretch::Normal => svgdom::ValueId::Normal,
                FontStretch::Wider => svgdom::ValueId::Wider,
                FontStretch::Narrower => svgdom::ValueId::Narrower,
                FontStretch::UltraCondensed => svgdom::ValueId::UltraCondensed,
                FontStretch::ExtraCondensed => svgdom::ValueId::ExtraCondensed,
                FontStretch::Condensed => svgdom::ValueId::Condensed,
                FontStretch::SemiCondensed => svgdom::ValueId::SemiCondensed,
                FontStretch::SemiExpanded => svgdom::ValueId::SemiExpanded,
                FontStretch::Expanded => svgdom::ValueId::Expanded,
                FontStretch::ExtraExpanded => svgdom::ValueId::ExtraExpanded,
                FontStretch::UltraExpanded => svgdom::ValueId::UltraExpanded,
            }
        ));
    }
}
