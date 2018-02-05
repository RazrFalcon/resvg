// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom::{
    AttributeType,
    Document,
    ElementType,
    Node,
};

// self
use short::{
    AId,
    AValue,
    EId,
    Unit,
};
use super::{
    DEFAULT_FONT_FAMILY,
};


pub fn resolve_style_attributes(doc: &Document) {
    resolve_inherit(&doc.root());
}

fn resolve_inherit(parent: &Node) {
    for (id, mut node) in parent.children().svg() {
        // Commented stuff is not supported yet, so there is no point in resolving it.

        if node.is_text_content() {
            // resolve(&mut node, AId::Direction)?;
            // resolve(&mut node, AId::FontSize)?;
            // resolve(&mut node, AId::FontSizeAdjust)?;
            resolve(&mut node, AId::FontStretch);
            resolve(&mut node, AId::FontStyle);
            resolve(&mut node, AId::FontVariant);
            resolve(&mut node, AId::FontWeight);
            // resolve(&mut node, AId::GlyphOrientationHorizontal)?;
            // resolve(&mut node, AId::GlyphOrientationVertical)?;
            // resolve(&mut node, AId::Kerning)?;
            // resolve(&mut node, AId::LetterSpacing)?;
            resolve(&mut node, AId::TextAnchor);
            // resolve(&mut node, AId::TextRendering)?;
            // resolve(&mut node, AId::WordSpacing)?;
            // resolve(&mut node, AId::WritingMode)?;

            resolve_font_family(&mut node);
        }

        // if node.is_container() || node.is_graphic() {
            // resolve(&mut node, AId::ColorInterpolation);
            // resolve(&mut node, AId::ColorRendering);
        // }

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

        // if node.is_shape() {
        //     resolve(&mut node, AId::ShapeRendering);
        // }

        if node.is_graphic() && node.parent().unwrap().is_tag_name(EId::ClipPath) {
            resolve(&mut node, AId::ClipRule);
        }

        // if node.parent().unwrap().has_tag_name(EId::Filter) {
        //     resolve(&mut node, AId::ColorInterpolationFilters);
        // }

        // if id == EId::Image {
            // resolve(&mut node, AId::ColorProfile);
            // resolve(&mut node, AId::ImageRendering);
        // }

        // if id == EId::Path || id == EId::Line || id == EId::Polyline || id == EId::Polygon {
        //     resolve(&mut node, AId::Marker);
        //     resolve(&mut node, AId::MarkerStart);
        //     resolve(&mut node, AId::MarkerMid);
        //     resolve(&mut node, AId::MarkerEnd);
        // }

        if node.has_children() {
            resolve_inherit(&node);
        }
    }
}

fn resolve(node: &mut Node, aid: AId) {
    debug_assert!(aid.is_inheritable(), "'{}' is not inheritable", aid);

    if !node.has_attribute(aid) {
        if let Some(n) = node.parents().find(|n| n.has_attribute(aid)) {
            // unwrap is safe, because we know that node contains an attribute
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

fn resolve_font_family(node: &mut Node) {
    let aid = AId::FontFamily;
    if !node.has_attribute(aid) {
        if let Some(n) = node.parents().find(|n| n.has_attribute(aid)) {
            // unwrap is safe, because we know that node contains an attribute
            node.set_attribute(n.attributes().get(aid).cloned().unwrap());
        } else {
            // `font-family` depends on user agent, so we use our own font
            // TODO: maybe use a system font
            warn!("'font-family' is not set. Fallback to '{}'.", DEFAULT_FONT_FAMILY);
            node.set_attribute((aid, DEFAULT_FONT_FAMILY));
        }
    }
}
