// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom::{
    Length,
};

use super::prelude::*;


pub fn resolve_font_size(doc: &Document, opt: &Options) {
    _resolve_font_size(&doc.root(), opt);
}

pub fn _resolve_font_size(parent: &Node, opt: &Options) {
    for (_, mut node) in parent.children().svg() {
        // We have to resolve `font-size` for all elements
        // and not only for 'text content' based,
        // because it will be used during `em`/`ex` units conversion.
        //
        // https://www.w3.org/TR/2008/REC-CSS2-20080411/fonts.html#propdef-font-size

        let font_size = match node.attributes().get(AId::FontSize) {
            Some(v) => {
                v.value.clone()
            }
            None => {
                // If not set - lookup in parent nodes or use default.
                let mut len = node.find_attribute(AId::FontSize)
                                  .unwrap_or(Length::new_number(opt.font_size));

                // If `font-size` is not set and the parent one is `em` or `ex`
                // then the current `'`font-size`'` is `1em` or `2ex` respectively.
                // This way do not introduce an additional scaling.
                //
                // Example:
                // <g font-size='12'>
                //   <g font-size='3em'>
                //     <g>
                //
                // The values are '12', '3em' and 'None'.
                // And the expected results are '12', '36' and '36'.
                // But if we simply copy the '3em' to the 'None' place we will
                // get '12', '36' and '108'.
                if len.unit == Unit::Em {
                    len.num = 1.0;
                } else if len.unit == Unit::Ex {
                    // The same coefficient as in convert_units::convert.
                    len.num = 2.0;
                }

                AValue::Length(len)
            }
        };

        let font_size = match font_size {
            AValue::Length(len) => {
                if len.unit == Unit::Percent {
                    process_percent_font_size(parent, len, opt)
                } else {
                    len
                }
            }
            AValue::String(ref name) => {
                process_named_font_size(parent, name, &font_size, opt)
            }
            _ => {
                warn!("Invalid 'font-size' value: {}.", font_size);
                Length::new_number(opt.font_size)
            }
        };

        // We have to mark this attribute as invisible,
        // otherwise it will break the `use` resolving.
        if !node.has_attribute(AId::FontSize) {
            node.set_attribute(("resolved-font-size", 1));
        }

        node.set_attribute((AId::FontSize, font_size));

        if node.has_children() {
            _resolve_font_size(&node, opt);
        }
    }
}

// If `font-size` has percent units that it's value
// is relative to the parent node `font-size`.
fn process_percent_font_size(parent: &Node, len: Length, opt: &Options) -> Length {
    if parent.is_root() {
        Length::new_number(opt.font_size)
    } else {
        let parent_len = parent.find_attribute(AId::FontSize)
                               .unwrap_or(Length::new_number(opt.font_size));

        let n = len.num * parent_len.num * 0.01;
        Length::new_number(n)
    }
}

fn process_named_font_size(
    parent: &Node,
    name: &str,
    font_size: &AValue,
    opt: &Options,
) -> Length {
    let factor = match name {
        "xx-small" => -3,
        "x-small" => -2,
        "small" => -1,
        "medium" => 0,
        "large" => 1,
        "x-large" => 2,
        "xx-large" => 3,
        "smaller" => -1,
        "larger" => 1,
        _ => {
            warn!("Invalid 'font-size' value: {}.", font_size);
            0
        }
    };

    let parent_len = parent.find_attribute(AId::FontSize)
                           .unwrap_or(Length::new_number(opt.font_size));

    // 'On a computer screen a scaling factor of 1.2
    // is suggested between adjacent indexes'
    let n = parent_len.num * 1.2f64.powi(factor);
    Length::new_number(n)
}
