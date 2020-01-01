// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::rc::Rc;
use std::str::FromStr;
use std::collections::HashMap;

use log::warn;

pub use roxmltree::Error;

use crate::tree;
use crate::Rect;
use super::{Document, Attribute, AId, EId, Node, NodeId, NodeKind, NodeData, AttributeValue};

const SVG_NS: &str = "http://www.w3.org/2000/svg";
const XLINK_NS: &str = "http://www.w3.org/1999/xlink";
const XML_NAMESPACE_NS: &str = "http://www.w3.org/XML/1998/namespace";


impl Document {
    pub fn parse(text: &str) -> Result<Document, Error> {
        parse(text)
    }

    fn append(&mut self, parent_id: NodeId, kind: NodeKind) -> NodeId {
        let new_child_id = NodeId(self.nodes.len());
        self.nodes.push(NodeData {
            parent: Some(parent_id),
            prev_sibling: None,
            next_sibling: None,
            children: None,
            kind,
        });

        let last_child_id = self.nodes[parent_id.0].children.map(|(_, id)| id);
        self.nodes[new_child_id.0].prev_sibling = last_child_id;

        if let Some(id) = last_child_id {
            self.nodes[id.0].next_sibling = Some(new_child_id);
        }

        self.nodes[parent_id.0].children = Some(
            if let Some((first_child_id, _)) = self.nodes[parent_id.0].children {
                (first_child_id, new_child_id)
            } else {
                (new_child_id, new_child_id)
            }
        );

        new_child_id
    }

    fn append_attribute(&mut self, tag_name: EId, aid: AId, value: &str) {
        let value2 = parse_svg_attribute(tag_name, aid, value);
        if let Ok(value) = value2 {
            self.attrs.push(Attribute {
                name: aid,
                value,
            });
        } else {
            // Invalid `enable-background` is not an error
            // since we are ignoring the `accumulate` value.
            if aid != AId::EnableBackground {
                warn!("Failed to parse {} value: '{}'.", aid, value);
            }
        }
    }
}

fn parse(text: &str) -> Result<Document, Error> {
    let xml = roxmltree::Document::parse(text)?;

    let mut doc = Document {
        nodes: Vec::new(),
        attrs: Vec::new(),
        links: HashMap::new(),
    };

    // Add a root node.
    doc.nodes.push(NodeData {
        parent: None,
        prev_sibling: None,
        next_sibling: None,
        children: None,
        kind: NodeKind::Root,
    });

    let style_sheet = resolve_css(&xml);

    parse_xml_node_children(xml.root(), xml.root(), doc.root().id, &style_sheet, false, &mut doc);

    // Check that the root element is `svg`.
    match doc.root().first_element_child() {
        Some(child) => {
            if child.tag_name() != Some(EId::Svg) {
                return Err(Error::NoRootNode)
            }
        }
        None => return Err(Error::NoRootNode),
    }

    // Collect all elements with `id` attribute.
    let mut links = HashMap::new();
    for node in doc.descendants() {
        if let Some(id) = node.attribute::<&str>(AId::Id) {
            links.insert(id.to_string(), node.id);
        }
    }
    doc.links = links;

    fix_recursive_patterns(&mut doc);
    fix_recursive_links(EId::ClipPath, AId::ClipPath, &mut doc);
    fix_recursive_links(EId::Mask, AId::Mask, &mut doc);
    fix_recursive_links(EId::Filter, AId::Filter, &mut doc);

    Ok(doc)
}

fn parse_tag_name(node: roxmltree::Node) -> Option<EId> {
    if !node.is_element() {
        return None;
    }

    if node.tag_name().namespace() != Some(SVG_NS) {
        return None;
    }

    EId::from_str(node.tag_name().name())
}

fn parse_xml_node_children(
    parent: roxmltree::Node,
    origin: roxmltree::Node,
    parent_id: NodeId,
    style_sheet: &simplecss::StyleSheet,
    ignore_ids: bool,
    doc: &mut Document,
) {
    for node in parent.children() {
        parse_xml_node(node, origin, parent_id, style_sheet, ignore_ids, doc);
    }
}

fn parse_xml_node(
    node: roxmltree::Node,
    origin: roxmltree::Node,
    parent_id: NodeId,
    style_sheet: &simplecss::StyleSheet,
    ignore_ids: bool,
    doc: &mut Document,
) {
    let mut tag_name = match parse_tag_name(node) {
        Some(id) => id,
        None => return,
    };

    if tag_name == EId::Style {
        return;
    }

    // Treat links as groups.
    if tag_name == EId::A {
        tag_name = EId::G;
    }

    let node_id = parse_svg_element(node, parent_id, tag_name, style_sheet, ignore_ids, doc);
    if tag_name == EId::Text {
        parse_svg_text_element(node, node_id, style_sheet, doc);
    } else if tag_name == EId::Use {
        parse_svg_use_element(node, origin, node_id, style_sheet, doc);
    } else {
        parse_xml_node_children(node, origin, node_id, style_sheet, ignore_ids, doc);
    }
}

fn parse_svg_element(
    xml_node: roxmltree::Node,
    parent_id: NodeId,
    tag_name: EId,
    style_sheet: &simplecss::StyleSheet,
    ignore_ids: bool,
    doc: &mut Document,
) -> NodeId {
    let attrs_start_idx = doc.attrs.len();

    // Copy presentational attributes first.
    for attr in xml_node.attributes() {
        match attr.namespace() {
              None
            | Some(SVG_NS)
            | Some(XLINK_NS)
            | Some(XML_NAMESPACE_NS) => {}
            _ => continue,
        }

        let aid = try_opt_continue!(AId::from_str(attr.name()));

        // During a `use` resolving, all `id` attributes must be ignored.
        // Otherwise we will get elements with duplicated id's.
        if ignore_ids && aid == AId::Id {
            continue;
        }

        append_attribute(parent_id, tag_name, aid, attr.value(), doc);
    }

    let mut insert_attribute = |aid, value: &str| {
        // Check that attribute already exists.
        let idx = doc.attrs[attrs_start_idx..].iter_mut().position(|a| a.name == aid);

        // Append an attribute as usual.
        let added = append_attribute(parent_id, tag_name, aid, value, doc);

        // Check that attribute was actually added, because it could be skipped.
        if added {
            if let Some(idx) = idx {
                // Swap the last attribute with an existing one.
                let last_idx = doc.attrs.len() - 1;
                doc.attrs.swap(attrs_start_idx + idx, last_idx);
                // Remove last.
                doc.attrs.pop();
            }
        }
    };

    // Apply CSS.
    for rule in &style_sheet.rules {
        if rule.selector.matches(&XmlNode(xml_node)) {
            for declaration in &rule.declarations {
                // TODO: preform XML attribute normalization
                if let Some(aid) = AId::from_str(declaration.name) {
                    // Parse only the presentation attributes.
                    // `transform` isn't a presentation attribute, but should be parsed anyway.
                    if aid.is_presentation() || aid == AId::Transform {
                        insert_attribute(aid, declaration.value);
                    }
                } else if declaration.name == "marker" {
                    insert_attribute(AId::MarkerStart, declaration.value);
                    insert_attribute(AId::MarkerMid, declaration.value);
                    insert_attribute(AId::MarkerEnd, declaration.value);
                }
            }
        }
    }

    // Split a `style` attribute.
    if let Some(value) = xml_node.attribute("style") {
        for declaration in simplecss::DeclarationTokenizer::from(value) {
            // TODO: preform XML attribute normalization
            if let Some(aid) = AId::from_str(declaration.name) {
                // Parse only the presentation attributes.
                // `transform` isn't a presentation attribute, but should be parsed anyway.
                if aid.is_presentation() || aid == AId::Transform {
                    insert_attribute(aid, declaration.value);
                }
            }
        }
    }

    let node_id = doc.append(parent_id, NodeKind::Element {
        tag_name,
        attributes: attrs_start_idx..doc.attrs.len(),
    });

    node_id
}

fn append_attribute(
    parent_id: NodeId,
    tag_name: EId,
    aid: AId,
    value: &str,
    doc: &mut Document,
) -> bool {
    match aid {
        // The `style` attribute will be split into attributes, so we don't need it.
        AId::Style |
        // No need to copy a `class` attribute since CSS were already resolved.
        AId::Class => return false,
        _ => {}
    }

    // Ignore `xlink:href` on `tspan` (which was originally `tref` or `a`),
    // because we will convert `tref` into `tspan` anyway.
    if tag_name == EId::Tspan && aid == AId::Href {
        return false;
    }

    if aid.allows_inherit_value() && value == "inherit" {
        return resolve_inherit(parent_id, tag_name, aid, doc);
    }

    doc.append_attribute(tag_name, aid, value);
    true
}

fn parse_svg_attribute(
    tag_name: EId,
    aid: AId,
    value: &str,
) -> Result<AttributeValue, svgtypes::Error> {
    Ok(match aid {
        AId::Href => {
            // `href` can contain base64 data and we do store it as is.
            match svgtypes::Stream::from(value).parse_iri() {
                Ok(link) => AttributeValue::Link(link.to_string()),
                Err(_) => AttributeValue::String(value.to_string()),
            }
        }

          AId::X  | AId::Y
        | AId::Dx | AId::Dy => {
            // Some attributes can contain different data based on the element type.
            match tag_name {
                  EId::Text
                | EId::Tref
                | EId::Tspan => {
                    AttributeValue::String(value.to_string())
                }
                  EId::FePointLight
                | EId::FeSpotLight => {
                    AttributeValue::Number(parse_number(value)?)
                }
                _ => {
                    AttributeValue::Length(svgtypes::Length::from_str(value)?)
                }
            }
        }

          AId::X1 | AId::Y1
        | AId::X2 | AId::Y2
        | AId::R
        | AId::Rx | AId::Ry
        | AId::Cx | AId::Cy
        | AId::Fx | AId::Fy
        | AId::RefX | AId::RefY
        | AId::Width | AId::Height
        | AId::MarkerWidth | AId::MarkerHeight
        | AId::StartOffset => {
            AttributeValue::Length(svgtypes::Length::from_str(value)?)
        }

        AId::Offset => {
            if let EId::FeFuncR | EId::FeFuncG | EId::FeFuncB | EId::FeFuncA = tag_name {
                AttributeValue::Number(parse_number(value)?)
            } else {
                // offset = <number> | <percentage>
                let l = svgtypes::Length::from_str(value)?;
                if l.unit == svgtypes::LengthUnit::None || l.unit == svgtypes::LengthUnit::Percent {
                    AttributeValue::Length(l)
                } else {
                    return Err(svgtypes::Error::InvalidValue);
                }
            }
        }

          AId::StrokeDashoffset
        | AId::StrokeWidth => {
            AttributeValue::Length(svgtypes::Length::from_str(value)?)
        }

          AId::Opacity
        | AId::FillOpacity
        | AId::FloodOpacity
        | AId::StrokeOpacity
        | AId::StopOpacity => {
            let n = parse_number(value)?;
            let n = crate::f64_bound(0.0, n, 1.0);
            AttributeValue::Opacity(n.into())
        }

          AId::Amplitude
        | AId::Azimuth
        | AId::Bias
        | AId::DiffuseConstant
        | AId::Divisor
        | AId::Elevation
        | AId::Exponent
        | AId::Intercept
        | AId::K1
        | AId::K2
        | AId::K3
        | AId::K4
        | AId::LimitingConeAngle
        | AId::NumOctaves
        | AId::PointsAtX
        | AId::PointsAtY
        | AId::PointsAtZ
        | AId::Scale
        | AId::Seed
        | AId::Slope
        | AId::SpecularConstant
        | AId::SpecularExponent
        | AId::StrokeMiterlimit
        | AId::SurfaceScale
        | AId::TargetX
        | AId::TargetY
        | AId::Z => {
            AttributeValue::Number(parse_number(value)?)
        }

        AId::StrokeDasharray => {
            match value {
                "none" => AttributeValue::None,
                _ => AttributeValue::String(value.to_string()),
            }
        }

        AId::Fill => {
            match svgtypes::Paint::from_str(value) {
                Ok(svgtypes::Paint::None) => AttributeValue::None,
                Ok(svgtypes::Paint::Inherit) => unreachable!(),
                Ok(svgtypes::Paint::CurrentColor) => AttributeValue::CurrentColor,
                Ok(svgtypes::Paint::Color(color)) => AttributeValue::Color(color),
                Ok(svgtypes::Paint::FuncIRI(link, fallback)) => {
                    AttributeValue::Paint(link.to_string(), fallback)
                }
                Err(_) => {
                    warn!("Failed to parse fill value: '{}'. Fallback to black.", value);
                    AttributeValue::Color(svgtypes::Color::black())
                }
            }
        }

        AId::Stroke => {
            match svgtypes::Paint::from_str(value)? {
                svgtypes::Paint::None => AttributeValue::None,
                svgtypes::Paint::Inherit => unreachable!(),
                svgtypes::Paint::CurrentColor => AttributeValue::CurrentColor,
                svgtypes::Paint::Color(color) => AttributeValue::Color(color),
                svgtypes::Paint::FuncIRI(link, fallback) => {
                    AttributeValue::Paint(link.to_string(), fallback)
                }
            }
        }

          AId::ClipPath
        | AId::Filter
        | AId::MarkerEnd
        | AId::MarkerMid
        | AId::MarkerStart
        | AId::Mask => {
            match value {
                "none" => AttributeValue::None,
                _ => {
                    let mut s = svgtypes::Stream::from(value);
                    let link = s.parse_func_iri()?;
                    AttributeValue::Link(link.to_string())
                }
            }
        }

        AId::Color => {
            AttributeValue::Color(svgtypes::Color::from_str(value)?)
        }

          AId::FloodColor
        | AId::LightingColor
        | AId::StopColor => {
            match value {
                "currentColor" => AttributeValue::CurrentColor,
                _ => AttributeValue::Color(svgtypes::Color::from_str(value)?),
            }
        }

        AId::D => {
            let segments = parse_path(value);
            if segments.len() >= 2 {
                AttributeValue::Path(Rc::new(segments))
            } else {
                return Err(svgtypes::Error::InvalidValue);
            }
        }

          AId::Transform
        | AId::GradientTransform
        | AId::PatternTransform => {
            AttributeValue::Transform(svgtypes::Transform::from_str(value)?)
        }

        AId::FontSize => {
            match svgtypes::Length::from_str(value) {
                Ok(l) => AttributeValue::Length(l),
                Err(_) => AttributeValue::String(value.to_string()),
            }
        }

          AId::Display
        | AId::TextDecoration => {
            match value {
                "none" => AttributeValue::None,
                _ => AttributeValue::String(value.to_string()),
            }
        }
          AId::LetterSpacing
        | AId::WordSpacing => {
            match value {
                "normal" => AttributeValue::String(value.to_string()),
                _ => AttributeValue::Length(svgtypes::Length::from_str(value)?),
            }
        }

        AId::BaselineShift => {
            match value {
                "baseline" | "sub" | "super" => AttributeValue::String(value.to_string()),
                _ => AttributeValue::Length(svgtypes::Length::from_str(value)?),
            }
        }

        AId::Orient => {
            match value {
                "auto" => AttributeValue::String(value.to_string()),
                _ => AttributeValue::Angle(svgtypes::Angle::from_str(value)?),
            }
        }

        AId::ViewBox => {
            AttributeValue::ViewBox(svgtypes::ViewBox::from_str(value)?)
        }

        AId::PreserveAspectRatio => {
            AttributeValue::AspectRatio(svgtypes::AspectRatio::from_str(value)?)
        }

          AId::BaseFrequency
        | AId::KernelMatrix
        | AId::Radius
        | AId::Rotate
        | AId::TableValues
        | AId::Values => {
            AttributeValue::NumberList(svgtypes::NumberList::from_str(value)?)
        }

        AId::EnableBackground => {
            AttributeValue::EnableBackground(parse_enable_background(value)?)
        }

        _ => {
            AttributeValue::String(value.to_string())
        }
    })
}

#[inline(never)]
fn parse_number(value: &str) -> Result<f64, svgtypes::Error> {
    let mut s = svgtypes::Stream::from(value);
    let n = s.parse_number()?;

    if !s.at_end() {
        return Err(svgtypes::Error::InvalidNumber(0));
    }

    Ok(n)
}

#[inline(never)]
fn parse_path(text: &str) -> tree::PathData {
    // Previous MoveTo coordinates.
    let mut prev_mx = 0.0;
    let mut prev_my = 0.0;

    // Previous SmoothQuadratic coordinates.
    let mut prev_tx = 0.0;
    let mut prev_ty = 0.0;

    // Previous coordinates.
    let mut prev_x = 0.0;
    let mut prev_y = 0.0;

    let mut prev_seg = svgtypes::PathSegment::MoveTo { abs: true, x: 0.0, y: 0.0 };

    let mut path = tree::PathData::with_capacity(32);

    for segment in svgtypes::PathParser::from(text) {
        let segment = match segment {
            Ok(v) => v,
            Err(_) => break,
        };

        match segment {
            svgtypes::PathSegment::MoveTo { abs, mut x, mut y } => {
                if !abs {
                    // When we get 'm'(relative) segment, which is not first segment - then it's
                    // relative to a previous 'M'(absolute) segment, not to the first segment.
                    if let Some(tree::PathSegment::ClosePath) = path.last() {
                        x += prev_mx;
                        y += prev_my;
                    } else {
                        x += prev_x;
                        y += prev_y;
                    }
                }

                path.push_move_to(x, y);
                prev_seg = segment;
            }
            svgtypes::PathSegment::LineTo { abs, mut x, mut y } => {
                if !abs {
                    x += prev_x;
                    y += prev_y;
                }

                path.push_line_to(x, y);
                prev_seg = segment;
            }
            svgtypes::PathSegment::HorizontalLineTo { abs, mut x } => {
                if !abs {
                    x += prev_x;
                }

                path.push_line_to(x, prev_y);
                prev_seg = segment;
            }
            svgtypes::PathSegment::VerticalLineTo { abs, mut y } => {
                if !abs {
                    y += prev_y;
                }

                path.push_line_to(prev_x, y);
                prev_seg = segment;
            }
            svgtypes::PathSegment::CurveTo { abs, mut x1, mut y1, mut x2, mut y2, mut x, mut y } => {
                if !abs {
                    x1 += prev_x;
                    y1 += prev_y;
                    x2 += prev_x;
                    y2 += prev_y;
                    x += prev_x;
                    y += prev_y;
                }

                path.push_curve_to(x1, y1, x2, y2, x, y);

                // Remember as absolute.
                prev_seg = svgtypes::PathSegment::CurveTo { abs: true, x1, y1, x2, y2, x, y };
            }
            svgtypes::PathSegment::SmoothCurveTo { abs, mut x2, mut y2, mut x, mut y } => {
                // 'The first control point is assumed to be the reflection of the second control
                // point on the previous command relative to the current point.
                // (If there is no previous command or if the previous command
                // was not an C, c, S or s, assume the first control point is
                // coincident with the current point.)'
                let (x1, y1) = match prev_seg {
                    svgtypes::PathSegment::CurveTo { x2, y2, x, y, .. } |
                    svgtypes::PathSegment::SmoothCurveTo { x2, y2, x, y, .. } => {
                        (x * 2.0 - x2, y * 2.0 - y2)
                    }
                    _ => {
                        (prev_x, prev_y)
                    }
                };

                if !abs {
                    x2 += prev_x;
                    y2 += prev_y;
                    x += prev_x;
                    y += prev_y;
                }

                path.push_curve_to(x1, y1, x2, y2, x, y);

                // Remember as absolute.
                prev_seg = svgtypes::PathSegment::SmoothCurveTo { abs: true, x2, y2, x, y };
            }
            svgtypes::PathSegment::Quadratic { abs, mut x1, mut y1, mut x, mut y } => {
                if !abs {
                    x1 += prev_x;
                    y1 += prev_y;
                    x += prev_x;
                    y += prev_y;
                }

                path.push_quad_to(x1, y1, x, y);

                // Remember as absolute.
                prev_seg = svgtypes::PathSegment::Quadratic { abs: true, x1, y1, x, y };
            }
            svgtypes::PathSegment::SmoothQuadratic { abs, mut x, mut y } => {
                // 'The control point is assumed to be the reflection of
                // the control point on the previous command relative to
                // the current point. (If there is no previous command or
                // if the previous command was not a Q, q, T or t, assume
                // the control point is coincident with the current point.)'
                let (x1, y1) = match prev_seg {
                    svgtypes::PathSegment::Quadratic { x1, y1, x, y, .. } => {
                        (x * 2.0 - x1, y * 2.0 - y1)
                    }
                    svgtypes::PathSegment::SmoothQuadratic { x, y, .. } => {
                        (x * 2.0 - prev_tx, y * 2.0 - prev_ty)
                    }
                    _ => {
                        (prev_x, prev_y)
                    }
                };

                prev_tx = x1;
                prev_ty = y1;

                if !abs {
                    x += prev_x;
                    y += prev_y;
                }

                path.push_quad_to(x1, y1, x, y);

                // Remember as absolute.
                prev_seg = svgtypes::PathSegment::SmoothQuadratic { abs: true, x, y };
            }
            svgtypes::PathSegment::EllipticalArc {
                abs, rx, ry, x_axis_rotation, large_arc, sweep, mut x, mut y
            } => {
                if !abs {
                    x += prev_x;
                    y += prev_y;
                }

                path.push_arc_to(rx, ry, x_axis_rotation, large_arc, sweep, x, y);
                prev_seg = segment;
            }
            svgtypes::PathSegment::ClosePath { .. } => {
                if let Some(tree::PathSegment::ClosePath) = path.last() {
                    // Do not add sequential ClosePath segments.
                    // Otherwise it will break marker rendering.
                } else {
                    path.push_close_path();
                }

                prev_seg = segment;
            }
        }

        // Remember last position.
        if let Some(seg) = path.last() {
            match *seg {
                tree::PathSegment::MoveTo { x, y } => {
                    prev_x = x;
                    prev_y = y;
                    prev_mx = x;
                    prev_my = y;
                }
                tree::PathSegment::LineTo { x, y } => {
                    prev_x = x;
                    prev_y = y;
                }
                tree::PathSegment::CurveTo { x, y, .. } => {
                    prev_x = x;
                    prev_y = y;
                }
                tree::PathSegment::ClosePath => {
                    // ClosePath moves us to the last MoveTo coordinate,
                    // not previous.
                    prev_x = prev_mx;
                    prev_y = prev_my;
                }
            }
        }
    }

    path.shrink_to_fit();
    path
}

fn parse_enable_background(value: &str) -> Result<tree::EnableBackground, svgtypes::Error> {
    let mut s = svgtypes::Stream::from(value);
    s.skip_spaces();
    if s.starts_with(b"new") {
        s.advance(3);
        s.skip_spaces();
        if s.at_end() {
            return Ok(tree::EnableBackground(None));
        }

        let x = s.parse_list_number()?;
        let y = s.parse_list_number()?;
        let w = s.parse_list_number()?;
        let h = s.parse_list_number()?;

        s.skip_spaces();
        if !s.at_end() {
            return Err(svgtypes::Error::InvalidValue);
        }

        let r = Rect::new(x, y, w, h).ok_or(svgtypes::Error::InvalidValue)?;
        Ok(tree::EnableBackground(Some(r)))
    } else {
        Err(svgtypes::Error::InvalidValue)
    }
}

fn resolve_inherit(
    parent_id: NodeId,
    tag_name: EId,
    aid: AId,
    doc: &mut Document,
) -> bool {
    if aid.is_inheritable() {
        // Inheritable attributes can inherit a value from an any ancestor.
        let node_id = doc.get(parent_id).find_node_with_attribute(aid).map(|n| n.id);
        if let Some(node_id) = node_id {
            if let Some(attr) = doc.get(node_id).attributes().iter().find(|a| a.name == aid).cloned() {
                doc.attrs.push(Attribute {
                    name: aid,
                    value: attr.value,
                });

                return true;
            }
        }
    } else {
        // Non-inheritable attributes can inherit a value only from a direct parent.
        if let Some(attr) = doc.get(parent_id).attributes().iter().find(|a| a.name == aid).cloned() {
            doc.attrs.push(Attribute {
                name: aid,
                value: attr.value,
            });

            return true;
        }
    }

    // Fallback to a default value if possible.
    let value = match aid {
          AId::ImageRendering
        | AId::ShapeRendering
        | AId::TextRendering => "auto",

          AId::ClipPath
        | AId::Filter
        | AId::MarkerEnd
        | AId::MarkerMid
        | AId::MarkerStart
        | AId::Mask
        | AId::Stroke
        | AId::StrokeDasharray
        | AId::TextDecoration => "none",

          AId::FontStretch
        | AId::FontStyle
        | AId::FontVariant
        | AId::FontWeight
        | AId::LetterSpacing
        | AId::WordSpacing => "normal",

          AId::Fill
        | AId::FloodColor
        | AId::StopColor => "black",

          AId::FillOpacity
        | AId::FloodOpacity
        | AId::Opacity
        | AId::StopOpacity
        | AId::StrokeOpacity => "1",

          AId::ClipRule
        | AId::FillRule => "nonzero",

        AId::BaselineShift =>               "baseline",
        AId::ColorInterpolationFilters =>   "linearRGB",
        AId::Direction =>                   "ltr",
        AId::Display =>                     "inline",
        AId::FontSize =>                    "medium",
        AId::StrokeDashoffset =>            "0",
        AId::StrokeLinecap =>               "butt",
        AId::StrokeLinejoin =>              "miter",
        AId::StrokeMiterlimit =>            "4",
        AId::StrokeWidth =>                 "1",
        AId::TextAnchor =>                  "start",
        AId::Visibility =>                  "visible",
        AId::WritingMode =>                 "lr-tb",
        _ => return false,
    };

    doc.append_attribute(tag_name, aid, value);
    true
}

fn resolve_href<'a>(
    node: roxmltree::Node<'a, 'a>,
) -> Option<roxmltree::Node<'a, 'a>> {
    let link_value = node.attribute((XLINK_NS, "href"))?;
    let link_id = svgtypes::Stream::from(link_value).parse_iri().ok()?;
    node.document().descendants().find(|n| n.attribute("id") == Some(link_id))
}

fn parse_svg_use_element(
    node: roxmltree::Node,
    origin: roxmltree::Node,
    parent_id: NodeId,
    style_sheet: &simplecss::StyleSheet,
    doc: &mut Document,
) -> Option<()> {
    let link = resolve_href(node)?;

    if link == node || link == origin {
        warn!("Recursive 'use' detected. '{}' will be skipped.",
              node.attribute((SVG_NS, "id")).unwrap_or_default());
        return None;
    }

    let tag_name = parse_tag_name(link)?;

    // TODO: this
    // We don't support 'use' elements linked to 'svg' element.
    if tag_name == EId::Svg {
        warn!("'use' elements linked to an 'svg' element are not supported. Skipped.");
        return None;
    }

    // Check that none of the linked node's children reference current `use` node
    // via other `use` node.
    //
    // Example:
    // <g id="g1">
    //     <use xlink:href="#use1" id="use2"/>
    // </g>
    // <use xlink:href="#g1" id="use1"/>
    //
    // `use2` should be removed.
    //
    // Also, child should not reference its parent:
    // <g id="g1">
    //     <use xlink:href="#g1" id="use1"/>
    // </g>
    //
    // `use1` should be removed.
    let mut is_recursive = false;
    for link_child in link.descendants().skip(1).filter(|n| n.has_tag_name((SVG_NS, "use"))) {
        if let Some(link2) = resolve_href(link_child) {
            if link2 == node || link2 == link {
                is_recursive = true;
                break;
            }
        }
    }

    if is_recursive {
        warn!("Recursive 'use' detected. '{}' will be skipped.",
              node.attribute((SVG_NS, "id")).unwrap_or_default());
        return None;
    }

    parse_xml_node(link, node, parent_id, style_sheet, true, doc);
    Some(())
}

fn parse_svg_text_element(
    parent: roxmltree::Node,
    parent_id: NodeId,
    style_sheet: &simplecss::StyleSheet,
    doc: &mut Document,
) {
    debug_assert_eq!(parent.tag_name().name(), "text");

    let space = if doc.get(parent_id).has_attribute(AId::Space) {
        get_xmlspace(doc, parent_id, XmlSpace::Default)
    } else {
        if let Some(node) = doc.get(parent_id).ancestors().find(|n| n.has_attribute(AId::Space)) {
            get_xmlspace(doc, node.id, XmlSpace::Default)
        } else {
            XmlSpace::Default
        }
    };

    parse_svg_text_element_impl(parent, parent_id, style_sheet, space, doc);

    trim_text_nodes(parent_id, space, doc);
}

fn parse_svg_text_element_impl(
    parent: roxmltree::Node,
    parent_id: NodeId,
    style_sheet: &simplecss::StyleSheet,
    space: XmlSpace,
    doc: &mut Document,
) {
    for node in parent.children() {
        if node.is_text() {
            let text = trim_text(node.text().unwrap(), space);
            doc.append(parent_id, NodeKind::Text(text));
            continue;
        }

        let mut tag_name = try_opt_continue!(parse_tag_name(node));

        if tag_name == EId::A {
            // Treat links as a simple text.
            tag_name = EId::Tspan;
        }

        if !matches!(tag_name, EId::Tspan | EId::Tref | EId::TextPath) {
            continue;
        }

        // `textPath` must be a direct `text` child.
        if tag_name == EId::TextPath && parent.tag_name().name() != "text" {
            continue;
        }

        // We are converting `tref` into `tspan` to simplify later use.
        let mut is_tref = false;
        if tag_name == EId::Tref {
            tag_name = EId::Tspan;
            is_tref = true;
        }

        let node_id = parse_svg_element(node, parent_id, tag_name, style_sheet, false, doc);
        let space = get_xmlspace(doc, node_id, space);

        if is_tref {
            if let Some(href) = node.attribute((XLINK_NS, "href")) {
                if let Some(text) = resolve_tref_text(node.document(), href) {
                    let text = trim_text(&text, space);
                    doc.append(node_id, NodeKind::Text(text));
                }
            }
        } else {
            parse_svg_text_element_impl(node, node_id, style_sheet, space, doc);
        }
    }
}

fn resolve_tref_text(
    xml: &roxmltree::Document,
    href: &str,
) -> Option<String> {
    let id = svgtypes::Stream::from(href).parse_iri().ok()?;

    // Find linked element in the original tree.
    let node = xml.descendants().find(|n| n.attribute("id") == Some(id))?;

    // `tref` should be linked to an SVG element.
    parse_tag_name(node)?;

    // 'All character data within the referenced element, including character data enclosed
    // within additional markup, will be rendered.'
    //
    // So we don't care about attributes and everything. Just collecting text nodes data.
    //
    // Note: we have to filter nodes by `is_text()` first since `text()` will look up
    // for text nodes in element children therefore we will get duplicates.
    let text: String = node.descendants().filter(|n| n.is_text()).filter_map(|n| n.text()).collect();
    if text.is_empty() { None } else { Some(text) }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum XmlSpace {
    Default,
    Preserve,
}

fn get_xmlspace(doc: &Document, node_id: NodeId, default: XmlSpace) -> XmlSpace {
    match doc.get(node_id).attribute(AId::Space) {
        Some("preserve") => XmlSpace::Preserve,
        Some(_) => XmlSpace::Default,
        _ => default,
    }
}

trait StrTrim {
    fn remove_first_space(&mut self);
    fn remove_last_space(&mut self);
}

impl StrTrim for String {
    fn remove_first_space(&mut self) {
        debug_assert_eq!(self.chars().next().unwrap(), ' ');
        self.drain(0..1);
    }

    fn remove_last_space(&mut self) {
        debug_assert_eq!(self.chars().rev().next().unwrap(), ' ');
        self.pop();
    }
}

/// Prepares text nodes according to the spec: https://www.w3.org/TR/SVG11/text.html#WhiteSpace
///
/// This function handles:
/// - 'xml:space' processing
/// - tabs and newlines removing/replacing
/// - spaces trimming
fn trim_text_nodes(text_elem_id: NodeId, xmlspace: XmlSpace, doc: &mut Document) {
    let mut nodes = Vec::new(); // TODO: allocate only once
    collect_text_nodes(doc.get(text_elem_id), 0, &mut nodes);

    // `trim` method has already collapsed all spaces into a single one,
    // so we have to check only for one leading or trailing space.

    if nodes.len() == 1 {
        // Process element with a single text node child.

        let node_id = nodes[0].0.clone();

        if xmlspace == XmlSpace::Default {
            if let NodeKind::Text(ref mut text) = doc.nodes[node_id.0].kind {
                match text.len() {
                    0 => {} // An empty string. Do nothing.
                    1 => {
                        // If string has only one character and it's a space - clear this string.
                        if text.as_bytes()[0] == b' ' {
                            text.clear();
                        }
                    }
                    _ => {
                        // 'text' has at least 2 bytes, so indexing is safe.
                        let c1 = text.as_bytes()[0];
                        let c2 = text.as_bytes()[text.len() - 1];

                        if c1 == b' ' {
                            text.remove_first_space();
                        }

                        if c2 == b' ' {
                            text.remove_last_space();
                        }
                    }
                }
            }
        } else {
            // Do nothing when xml:space=preserve.
        }
    } else if nodes.len() > 1 {
        // Process element with many text node children.

        // We manage all text nodes as a single text node
        // and trying to remove duplicated spaces across nodes.
        //
        // For example    '<text>Text <tspan> text </tspan> text</text>'
        // is the same is '<text>Text <tspan>text</tspan> text</text>'

        let mut i = 0;
        let len = nodes.len() - 1;
        let mut last_non_empty: Option<NodeId> = None;
        while i < len {
            // Process pairs.
            let (mut node1_id, depth1) = nodes[i].clone();
            let (node2_id, depth2) = nodes[i + 1].clone();

            if doc.get(node1_id).text().is_empty() {
                if let Some(n) = last_non_empty {
                    node1_id = n;
                }
            }

            // Parent of the text node is always an element node and always exist,
            // so unwrap is safe.
            let xmlspace1 = get_xmlspace(doc, doc.get(node1_id).parent().unwrap().id, xmlspace);
            let xmlspace2 = get_xmlspace(doc, doc.get(node2_id).parent().unwrap().id, xmlspace);

            // >text<..>text<
            //  1  2    3  4
            let (c1, c2, c3, c4) = {
                let text1 = doc.get(node1_id).text();
                let text2 = doc.get(node2_id).text();

                let bytes1 = text1.as_bytes();
                let bytes2 = text2.as_bytes();

                let c1 = bytes1.first().cloned();
                let c2 = bytes1.last().cloned();
                let c3 = bytes2.first().cloned();
                let c4 = bytes2.last().cloned();

                (c1, c2, c3, c4)
            };

            // NOTE: xml:space processing is mostly an undefined behavior,
            // because everyone do it differently.
            // We're mimicking the Chrome behavior.

            // Remove space from the second text node if both nodes has bound spaces.
            // From: '<text>Text <tspan> text</tspan></text>'
            // To:   '<text>Text <tspan>text</tspan></text>'
            //
            // See text-tspan-02-b.svg for details.
            if c2 == Some(b' ') && c2 == c3 {
                if depth1 < depth2 {
                    if xmlspace2 == XmlSpace::Default {
                        if let NodeKind::Text(ref mut text) = doc.nodes[node2_id.0].kind {
                            text.remove_first_space();
                        }
                    }
                } else {
                    if xmlspace1 == XmlSpace::Default && xmlspace2 == XmlSpace::Default {
                        if let NodeKind::Text(ref mut text) = doc.nodes[node1_id.0].kind {
                            text.remove_last_space();
                        }
                    } else if xmlspace1 == XmlSpace::Preserve && xmlspace2 == XmlSpace::Default {
                        if let NodeKind::Text(ref mut text) = doc.nodes[node2_id.0].kind {
                            text.remove_first_space();
                        }
                    }
                }
            }

            let is_first = i == 0;
            let is_last  = i == len - 1;

            if     is_first
                && c1 == Some(b' ')
                && xmlspace1 == XmlSpace::Default
                && !doc.get(node1_id).text().is_empty()
            {
                // Remove a leading space from a first text node.
                if let NodeKind::Text(ref mut text) = doc.nodes[node1_id.0].kind {
                    text.remove_first_space();
                }
            } else if
                   is_last
                && c4 == Some(b' ')
                && !doc.get(node2_id).text().is_empty()
                && xmlspace2 == XmlSpace::Default
            {
                // Remove a trailing space from a last text node.
                // Also check that 'text2' is not empty already.
                if let NodeKind::Text(ref mut text) = doc.nodes[node2_id.0].kind {
                    text.remove_last_space();
                }
            }

            if     is_last
                && c2 == Some(b' ')
                && !doc.get(node1_id).text().is_empty()
                && doc.get(node2_id).text().is_empty()
                && doc.get(node1_id).text().ends_with(' ')
            {
                if let NodeKind::Text(ref mut text) = doc.nodes[node1_id.0].kind {
                    text.remove_last_space();
                }
            }

            if !doc.get(node1_id).text().trim().is_empty() {
                last_non_empty = Some(node1_id);
            }

            i += 1;
        }
    }

    // TODO: find a way to remove all empty text nodes
}

fn collect_text_nodes(parent: Node, depth: usize, nodes: &mut Vec<(NodeId, usize)>) {
    for child in parent.children() {
        if child.is_text() {
            nodes.push((child.id, depth));
        } else if child.is_element() {
            collect_text_nodes(child, depth + 1, nodes);
        }
    }
}

fn trim_text(text: &str, space: XmlSpace) -> String {
    let mut s = String::with_capacity(text.len());

    let mut prev = '0';
    for c in text.chars() {
        // \r, \n and \t should be converted into spaces.
        let c = match c {
            '\r' | '\n' | '\t' => ' ',
            _ => c,
        };

        // Skip continuous spaces.
        if space == XmlSpace::Default && c == ' ' && c == prev {
            continue;
        }

        prev = c;

        s.push(c);
    }

    s
}

fn resolve_css<'a>(xml: &'a roxmltree::Document<'a>) -> simplecss::StyleSheet<'a> {
    let mut sheet = simplecss::StyleSheet::new();

    for node in xml.descendants().filter(|n| n.has_tag_name("style")) {
        match node.attribute("type") {
            Some("text/css") => {}
            Some(_) => continue,
            None => {}
        }

        sheet.parse_more(try_opt_continue!(node.text()));
    }

    sheet
}

struct XmlNode<'a, 'input: 'a>(roxmltree::Node<'a, 'input>);

impl simplecss::Element for XmlNode<'_, '_> {
    fn parent_element(&self) -> Option<Self> {
        self.0.parent_element().map(XmlNode)
    }

    fn prev_sibling_element(&self) -> Option<Self> {
        self.0.prev_sibling_element().map(XmlNode)
    }

    fn has_local_name(&self, local_name: &str) -> bool {
        self.0.tag_name().name() == local_name
    }

    fn attribute_matches(&self, local_name: &str, operator: simplecss::AttributeOperator) -> bool {
        match self.0.attribute(local_name) {
            Some(value) => operator.matches(value),
            None => false,
        }
    }

    fn pseudo_class_matches(&self, class: simplecss::PseudoClass) -> bool {
        match class {
            simplecss::PseudoClass::FirstChild => self.prev_sibling_element().is_none(),
            // TODO: lang
            _ => false, // Since we are querying a static SVG we can ignore other pseudo-classes.
        }
    }
}

fn fix_recursive_patterns(doc: &mut Document) {
    while let Some(node_id) = find_recursive_pattern(AId::Fill, doc) {
        let idx = doc.get(node_id).attribute_id(AId::Fill).unwrap();
        doc.attrs[idx.0].value = AttributeValue::None;
    }

    while let Some(node_id) = find_recursive_pattern(AId::Stroke, doc) {
        let idx = doc.get(node_id).attribute_id(AId::Stroke).unwrap();
        doc.attrs[idx.0].value = AttributeValue::None;
    }
}

fn find_recursive_pattern(
    aid: AId,
    doc: &mut Document,
) -> Option<NodeId> {
    for pattern_node in doc.root().descendants().filter(|n| n.has_tag_name(EId::Pattern)) {
        for node in pattern_node.descendants() {
            if let Some(&AttributeValue::Paint(ref link_id, _)) = node.attribute(aid) {
                if link_id == pattern_node.element_id() {
                    // If a pattern child has a link to the pattern itself
                    // then we have to replace it with `none`.
                    // Otherwise we will get endless loop/recursion and stack overflow.
                    return Some(node.id);
                } else {
                    // Check that linked node children doesn't link this pattern.
                    for node2 in doc.element_by_id(link_id).unwrap().descendants() {
                        if let Some(&AttributeValue::Paint(ref link_id2, _)) = node2.attribute(aid) {
                            if link_id2 == pattern_node.element_id() {
                                return Some(node2.id);
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

fn fix_recursive_links(
    eid: EId,
    aid: AId,
    doc: &mut Document,
) {
    while let Some(node_id) = find_recursive_link(eid, aid, doc) {
        let idx = doc.get(node_id).attribute_id(aid).unwrap();
        doc.attrs[idx.0].value = AttributeValue::None;
    }
}

fn find_recursive_link(
    eid: EId,
    aid: AId,
    doc: &Document,
) -> Option<NodeId> {
    for node in doc.root().descendants().filter(|n| n.has_tag_name(eid)) {
        for child in node.descendants() {
            if let Some(link) = child.attribute::<Node>(aid) {
                if link == node {
                    // If an element child has a link to the element itself
                    // then we have to replace it with `none`.
                    // Otherwise we will get endless loop/recursion and stack overflow.
                    return Some(child.id);
                } else {
                    // Check that linked node children doesn't link this element.
                    for node2 in link.descendants() {
                        if let Some(link2) = node2.attribute::<Node>(aid) {
                            if link2 == node {
                                return Some(node2.id);
                            }
                        }
                    }
                }
            }
        }
    }

    None
}
