// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom;

// self
use tree;
use tree::NodeExt;
use super::prelude::*;
use super::{
    path,
    text,
    shapes,
    IriResolveResult,
};


pub fn convert(
    node: &svgdom::Node,
    tree: &mut tree::Tree,
) -> tree::Node {
    let attrs = node.attributes();

    let mut clip_path = None;
    if let Some(&AValue::FuncLink(ref link)) = attrs.get_type(AId::ClipPath) {
        if link.is_tag_name(EId::ClipPath) {
            clip_path = Some(link.id().to_string());
        }
    }

    tree.append_to_defs(
        tree::NodeKind::ClipPath(tree::ClipPath {
            id: node.id().clone(),
            units: super::convert_element_units(&attrs, AId::ClipPathUnits),
            transform: attrs.get_transform(AId::Transform).unwrap_or_default(),
            clip_path,
        })
    )
}

pub fn convert_children(
    parent_svg: &svgdom::Node,
    parent: &mut tree::Node,
    opt: &Options,
    tree: &mut tree::Tree,
) {
    for (id, node) in parent_svg.children().svg() {
        match id {
              EId::Rect
            | EId::Polyline
            | EId::Polygon
            | EId::Circle
            | EId::Ellipse => {
                if let Some(d) = shapes::convert(&node) {
                    path::convert(&node, d, parent.clone(), tree);
                }
            }
            EId::Path => {
                let attrs = node.attributes();
                if let Some(d) = attrs.get_path(AId::D) {
                    path::convert(&node, d.clone(), parent.clone(), tree);
                }
            }
            EId::Text => {
                text::convert(&node, opt, parent.clone(), tree);
            }
            EId::Line => {
                // `line` doesn't impact rendering because stroke is always disabled
                // for `clipPath` children. So we can ignore it completely.
            }
            EId::G => {
                // By the SVG spec, `clipPath` cannot contain a `g` element,
                // but since in `usvg` `clip-path` attribute can be set only for groups
                // we have to create one 'temporarily'.
                // It will be available in the tree, but will be ignored
                // during `Tree::to_svgdom` conversion.

                let attrs = node.attributes();
                let ts = attrs.get_transform(AId::Transform).unwrap_or_default();
                let opacity = attrs.get_number(AId::Opacity).map(|v| v.into());

                let clip_path = match super::resolve_iri(&node, EId::ClipPath, AId::ClipPath, tree) {
                    IriResolveResult::Id(id) => Some(id),
                    IriResolveResult::Skip => continue,
                    IriResolveResult::None => None,
                };

                let mask = match super::resolve_iri(&node, EId::Mask, AId::Mask, tree) {
                    IriResolveResult::Id(id) => Some(id),
                    IriResolveResult::Skip => continue,
                    IriResolveResult::None => None,
                };

                let filter = match super::resolve_iri(&node, EId::Filter, AId::Filter, tree) {
                    IriResolveResult::Id(id) => Some(id),
                    IriResolveResult::Skip => continue,
                    IriResolveResult::None => None,
                };

                if clip_path.is_none() && mask.is_none() && filter.is_none() {
                    continue;
                }

                let mut g_node = parent.append_kind(tree::NodeKind::Group(tree::Group {
                    id: node.id().clone(),
                    transform: ts,
                    opacity,
                    clip_path,
                    mask,
                    filter,
                }));

                convert_children(&node, &mut g_node, opt, tree);
            }
            _ => {
                warn!("Skipping the '{}' clipPath invalid child element '{}'.",
                      node.id(), id);
                continue;
            }
        }
    }
}
