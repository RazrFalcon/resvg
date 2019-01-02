// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use svgdom;

// self
use {
    Options,
};


mod clip_element;
mod conv_units;
mod fix_gradient_stops;
mod fix_links;
mod fix_recursive_links;
mod group_defs;
mod prepare_clip_path;
mod prepare_mask;
mod prepare_nested_svg;
mod prepare_text_decoration;
mod prepare_text_nodes;
mod regroup;
mod resolve_attrs_via_xlink;
mod resolve_children_via_xlink;
mod resolve_conditional;
mod resolve_curr_color;
mod resolve_display;
mod resolve_font_size;
mod resolve_font_weight;
mod resolve_inherit;
mod resolve_style_attrs;
mod resolve_svg_size;
mod resolve_tref;
mod resolve_use;
mod rm_invalid_font_size;
mod rm_invalid_gradients;
mod rm_invalid_ts;
mod rm_non_svg_data;
mod rm_unused_defs;
mod ungroup_a;
mod ungroup_groups;
mod prepare_marker;


use self::conv_units::*;
use self::fix_gradient_stops::*;
use self::fix_links::*;
use self::fix_recursive_links::*;
use self::group_defs::*;
use self::prepare_clip_path::*;
use self::prepare_mask::*;
use self::prepare_nested_svg::*;
use self::prepare_text_decoration::*;
use self::prepare_text_nodes::*;
use self::regroup::*;
use self::resolve_attrs_via_xlink::*;
use self::resolve_children_via_xlink::*;
use self::resolve_conditional::*;
use self::resolve_curr_color::*;
use self::resolve_display::*;
use self::resolve_font_size::*;
use self::resolve_font_weight::*;
use self::resolve_inherit::*;
use self::resolve_style_attrs::*;
use self::resolve_svg_size::*;
use self::resolve_tref::*;
use self::resolve_use::*;
use self::rm_invalid_font_size::*;
use self::rm_invalid_gradients::*;
use self::rm_invalid_ts::*;
use self::rm_non_svg_data::*;
use self::rm_unused_defs::*;
use self::ungroup_a::*;
use self::ungroup_groups::*;
use self::prepare_marker::*;


mod prelude {
    pub use svgdom::{
        AttributeType,
        Document,
        ElementType,
        FilterSvg,
        FilterSvgAttrs,
        FilterSvgAttrsMut,
        FuzzyEq,
        FuzzyZero,
        Node,
    };
    pub use geom::*;
    pub use short::*;
    pub use traits::*;
    pub use Options;
}


/// Prepares an input `Document`.
///
/// # Errors
///
/// - If `Document` doesn't have an SVG node - clears the `doc`.
/// - If `Document` size can't be determined - clears the `doc`.
///
/// Basically, any error, even a critical one, should be recoverable.
/// In worst case scenario clear the `doc`.
///
/// Must not panic!
pub fn prepare_doc(doc: &mut svgdom::Document, opt: &Options) {
    let mut svg = if let Some(svg) = doc.svg_element() {
        svg
    } else {
        // Technically unreachable, because svgdom will return a parser error
        // if input SVG doesn't have an `svg` node.
        warn!("Invalid SVG structure. The Document will be cleared.");
        *doc = svgdom::Document::new();
        return;
    };

    let svg = &mut svg;

    // Detect image size. If it failed there is no point in continuing.
    if !resolve_svg_size(svg) {
        warn!("File doesn't have 'width', 'height' and 'viewBox' attributes. \
               Automatic image size determination is not supported. \
               The Document will be cleared.");
        *doc = svgdom::Document::new();
        return;
    }

    // TODO: remove duplicated defs

    remove_non_svg_data(doc);
    remove_descriptive_elements(doc);

    resolve_root_style_attributes(doc, svg);

    fix_links(doc);
    fix_xlinks(doc);

    resolve_font_size(doc, opt);
    resolve_font_weight(doc);

    resolve_mask_attributes(doc);
    resolve_use_attributes(doc);
    resolve_svg_attributes(doc);

    resolve_linear_gradient_attributes(doc);
    resolve_radial_gradient_attributes(doc);

    resolve_pattern_attributes(doc);
    resolve_pattern_children(doc);

    resolve_filter_attributes(doc);
    resolve_filter_children(doc);

    convert_units(svg, opt);

    fix_radial_gradient_attributes(doc);

    // `use` should be resolved before style attributes,
    // because `use` can propagate own style.
    resolve_use(doc, opt);

    prepare_nested_svg(doc, svg);

    resolve_inherit(doc);
    resolve_current_color(doc);

    group_defs(doc, svg);

    resolve_gradient_stops(doc);
    fix_gradient_stops(doc);

    resolve_clip_path_attributes(doc);

    fix_recursive_links(doc);

    remove_unused_defs(doc);

    ungroup_a(doc);

    prepare_text_decoration(doc);
    resolve_style_attributes(doc, opt);

    rm_marker_attributes(doc);

    // Should be done only after style resolving.
    remove_invalid_gradients(doc);

    resolve_tref(doc);

    remove_xlinks(doc);

    resolve_conditional(doc, opt);

    remove_invalid_transform(doc);
    resolve_display(doc);

    prepare_clip_path_children(doc);

    ungroup_groups(doc, opt);
    regroup_elements(doc, svg);

    prepare_text_nodes(doc);
    remove_invalid_font_size(doc, opt);

    remove_unused_defs(doc);
}
