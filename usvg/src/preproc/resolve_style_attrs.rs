// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use super::prelude::*;

/// Resolves the root `svg` element attributes.
///
/// In the `usvg`, the root `svg` element can't have any style attributes,
/// so we have to create a new root group and move all non-inheritable attributes into it.
pub fn resolve_root_style_attributes(doc: &mut Document, svg: &mut Node) {
    // Create a new group only when needed.
    let has_any =
           svg.has_attribute(AId::ClipPath)
        || svg.has_attribute(AId::Filter)
        || svg.has_attribute(AId::Mask)
        || svg.has_attribute(AId::Opacity)
        || svg.has_attribute(AId::Transform);

    if !has_any {
        return;
    }

    let mut g = doc.create_element(EId::G);

    let children: Vec<_> = svg.children().collect();
    for child in children {
        g.append(child);
    }

    svg.append(g.clone());

    svg.move_attribute_to(AId::ClipPath, &mut g);
    svg.move_attribute_to(AId::Filter, &mut g);
    svg.move_attribute_to(AId::Mask, &mut g);
    svg.move_attribute_to(AId::Opacity, &mut g);
    svg.move_attribute_to(AId::Transform, &mut g);
}

pub fn resolve_style_attributes(doc: &Document, opt: &Options) {
    resolve_inherit(&doc.root(), opt);
}

fn resolve_inherit(parent: &Node, opt: &Options) {
    for (id, mut node) in parent.children().svg() {
        // Commented stuff is not supported yet, so there is no point in resolving it.

        if node.is_text_content() {
            resolve(&mut node, AId::FontStretch);
            resolve(&mut node, AId::FontStyle);
            resolve(&mut node, AId::FontVariant);
            resolve(&mut node, AId::FontWeight);
            resolve(&mut node, AId::TextAnchor);
            resolve(&mut node, AId::LetterSpacing);
            resolve(&mut node, AId::WordSpacing);
            resolve_font_family(&mut node, opt);
        }

        if node.is_shape() || node.is_text_content() || id == EId::G {
            resolve(&mut node, AId::Fill);
            resolve(&mut node, AId::FillOpacity);
            resolve(&mut node, AId::FillRule);
            resolve(&mut node, AId::Stroke);
            resolve(&mut node, AId::StrokeDasharray);
            resolve(&mut node, AId::StrokeDashoffset);
            resolve(&mut node, AId::StrokeLinecap);
            resolve(&mut node, AId::StrokeLinejoin);
            resolve(&mut node, AId::StrokeMiterlimit);
            resolve(&mut node, AId::StrokeOpacity);
            resolve(&mut node, AId::StrokeWidth);
        }

        if node.is_shape() {
            resolve(&mut node, AId::MarkerStart);
            resolve(&mut node, AId::MarkerMid);
            resolve(&mut node, AId::MarkerEnd);
        }

        if node.is_graphic() && node.parent().unwrap().is_tag_name(EId::ClipPath) {
            resolve(&mut node, AId::ClipRule);
        }

        if node.is_filter_primitive() {
            resolve(&mut node, AId::ColorInterpolationFilters);
        }

        if node.has_children() {
            resolve_inherit(&node, opt);
        }
    }
}

fn resolve(node: &mut Node, aid: AId) {
    debug_assert!(aid.is_inheritable(), "'{}' is not inheritable", aid);

    if !node.has_attribute(aid) {
        if let Some(n) = node.ancestors().skip(1).find(|n| n.has_attribute(aid)) {
            // Unwrap is safe, because we know that node contains an attribute.
            node.set_attribute(n.attributes().get(aid).cloned().unwrap());
        } else {
            resolve_default(node, aid);
        }
    }
}

fn resolve_default(node: &mut Node, aid: AId) {
    let mut v = match AValue::default_value(aid) {
        Some(v) => v,
        None => {
            // Technically unreachable.
            warn!("'{:?}' doesn't have a default value.", aid);
            return;
        }
    };

    // Convert length to number.
    // All default values have Unit::None, so it's safe
    // and we don't need preproc::conv_units.
    if let AValue::Length(len) = v {
        debug_assert!(len.unit == Unit::None);
        v = AValue::Number(len.num);
    }

    node.set_attribute((aid, v));
}

fn resolve_font_family(node: &mut Node, opt: &Options) {
    let aid = AId::FontFamily;
    if !node.has_attribute(aid) {
        if let Some(n) = node.ancestors().skip(1).find(|n| n.has_attribute(aid)) {
            // Unwrap is safe, because we know that node contains an attribute.
            node.set_attribute(n.attributes().get(aid).cloned().unwrap());
        } else {
            // `font-family` depends on user agent, so we use our own font.
            warn!("'font-family' is not set. Fallback to '{}'.", opt.font_family);
            node.set_attribute((aid, opt.font_family.clone()));
        }
    }
}
