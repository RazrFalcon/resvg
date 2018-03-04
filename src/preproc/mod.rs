// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom;

// self
use {
    ErrorKind,
    Options,
    Result,
};


mod conv_units;
mod fix_gradient_stops;
mod fix_xlinks;
mod group_defs;
mod prepare_clip_path;
mod prepare_text_decoration;
mod prepare_text_nodes;
mod regroup;
mod resolve_curr_color;
mod resolve_font_size;
mod resolve_font_weight;
mod resolve_gradient_attrs;
mod resolve_gradient_stops;
mod resolve_inherit;
mod resolve_pattern_children;
mod resolve_style_attrs;
mod resolve_svg_size;
mod resolve_tref;
mod resolve_use;
mod resolve_visibility;
mod rm_invalid_font_size;
mod rm_invalid_gradients;
mod rm_invalid_patterns;
mod rm_invalid_ts;
mod rm_invisible_elems;
mod rm_unused_defs;
mod ungroup_a;
mod ungroup_groups;
mod ungroup_switch;


use self::conv_units::convert_units;
use self::fix_gradient_stops::fix_gradient_stops;
use self::fix_xlinks::fix_xlinks;
use self::group_defs::group_defs;
use self::prepare_clip_path::prepare_clip_path;
use self::prepare_text_decoration::prepare_text_decoration;
use self::prepare_text_nodes::prepare_text_nodes;
use self::regroup::regroup_elements;
use self::resolve_curr_color::resolve_current_color;
use self::resolve_font_size::resolve_font_size;
use self::resolve_font_weight::resolve_font_weight;
use self::resolve_gradient_attrs::*;
use self::resolve_gradient_stops::resolve_gradient_stops;
use self::resolve_inherit::resolve_inherit;
use self::resolve_pattern_children::resolve_pattern_children;
use self::resolve_style_attrs::resolve_style_attributes;
use self::resolve_svg_size::resolve_svg_size;
use self::resolve_tref::resolve_tref;
use self::resolve_use::resolve_use;
use self::resolve_visibility::resolve_visibility;
use self::rm_invalid_font_size::remove_invalid_font_size;
use self::rm_invalid_gradients::remove_invalid_gradients;
use self::rm_invalid_patterns::remove_invalid_patterns;
use self::rm_invalid_ts::remove_invalid_transform;
use self::rm_invisible_elems::remove_invisible_elements;
use self::rm_unused_defs::remove_unused_defs;
use self::ungroup_a::ungroup_a;
use self::ungroup_groups::ungroup_groups;
use self::ungroup_switch::ungroup_switch;


// Default font is user-agent dependent so we can use whatever we like.
pub const DEFAULT_FONT_FAMILY: &str = "Times New Roman";
pub const DEFAULT_FONT_SIZE: f64 = 12.0;


pub fn prepare_doc(doc: &mut svgdom::Document, opt: &Options) -> Result<()> {
    let mut svg = if let Some(svg) = doc.svg_element() {
        svg
    } else {
        // Technically unreachable, because svgdom will return a parser error
        // if input SVG doesn't have an 'svg' node.
        return Err(ErrorKind::MissingSvgNode.into());
    };

    let svg = &mut svg;

    // Detect image size. If it failed there is no point in continuing.
    if !resolve_svg_size(svg) {
        return Err(ErrorKind::SizeDeterminationUnsupported.into());
    }

    // TODO: remove duplicated defs

    resolve_inherit(doc);
    resolve_current_color(doc);

    group_defs(doc, svg);

    resolve_font_size(doc);
    resolve_font_weight(doc);

    convert_units(svg, opt);

    fix_xlinks(doc);

    resolve_linear_gradient_attributes(doc);
    resolve_radial_gradient_attributes(doc);
    resolve_gradient_stops(doc);
    remove_invalid_gradients(doc);
    fix_gradient_stops(doc);

    resolve_pattern_attributes(doc);
    resolve_pattern_children(doc);
    remove_invalid_patterns(doc);

    remove_unused_defs(svg);

    // 'use' should be resolved before style attributes,
    // because 'use' can propagate own style.
    resolve_use(doc);

    ungroup_a(doc);

    prepare_text_decoration(doc);
    resolve_visibility(svg);
    resolve_style_attributes(doc);

    prepare_clip_path(doc);

    resolve_tref(doc);

    ungroup_switch(doc);

    remove_invalid_transform(doc);
    remove_invisible_elements(doc);

    ungroup_groups(svg, opt);
    regroup_elements(doc, svg);

    prepare_text_nodes(doc);
    remove_invalid_font_size(doc);

    Ok(())
}
