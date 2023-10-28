// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/*!
`usvg` (micro SVG) is an [SVG] parser that tries to solve most of SVG complexity.

SVG is notoriously hard to parse. `usvg` presents a layer between an XML library and
a potential SVG rendering library. It will parse an input SVG into a strongly-typed tree structure
were all the elements, attributes, references and other SVG features are already resolved
and presented in a simplest way possible.
So a caller doesn't have to worry about most of the issues related to SVG parsing
and can focus just on the rendering part.

## Features

- All supported attributes are resolved.
  No need to worry about inheritable, implicit and default attributes
- CSS will be applied
- Only simple paths
  - Basic shapes (like `rect` and `circle`) will be converted into paths
  - Paths contain only absolute *MoveTo*, *LineTo*, *CurveTo* and *ClosePath* segments.
    ArcTo, implicit and relative segments will be converted
- `use` will be resolved and replaced with the reference content
- Nested `svg` will be resolved
- Invalid, malformed elements will be removed
- Relative length units (mm, em, etc.) will be converted into pixels/points
- External images will be loaded
- Internal, base64 images will be decoded
- All references (like `#elem` and `url(#elem)`) will be resolved
- `switch` will be resolved
- Text elements, which are probably the hardest part of SVG, will be completely resolved.
  This includes all the attributes resolving, whitespaces preprocessing (`xml:space`),
  text chunks and spans resolving
- Markers will be converted into regular elements. No need to place them manually
- All filters are supported. Including filter functions, like `filter="contrast(50%)"`
- Recursive elements will be detected an removed

## Limitations

- Unsupported SVG features will be ignored
- CSS support is minimal
- Only [static](http://www.w3.org/TR/SVG11/feature#SVG-static) SVG features,
  e.g. no `a`, `view`, `cursor`, `script`, no events and no animations
- Text elements must be converted into paths before writing to SVG.

[SVG]: https://en.wikipedia.org/wiki/Scalable_Vector_Graphics
*/

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![warn(missing_copy_implementations)]

mod writer;

use std::collections::HashMap;
use std::process::id;
pub use usvg_parser::*;
#[cfg(feature = "text")]
pub use usvg_text_layout::*;
pub use usvg_tree::*;

pub use writer::XmlOptions;

/// A trait to write `usvg::Tree` back to SVG.
pub trait TreeWriting {
    /// Writes `usvg::Tree` back to SVG.
    fn to_string(&self, opt: &XmlOptions) -> String;
}

impl TreeWriting for usvg_tree::Tree {
    fn to_string(&self, opt: &XmlOptions) -> String {
        writer::convert(self, opt)
    }
}

pub trait IdRemapping {
    fn remap_ids(&mut self);
}

#[derive(Default)]
struct IdMap {
    id_map: HashMap<String, String>,
    id_counters: IdCounters
}

#[derive(Default)]
struct IdCounters {
    path: u64,
    group: u64,
    mask: u64,
    clip_path: u64,
    pattern: u64,
    radial_gradient: u64,
    linear_gradient: u64,
    image: u64
}

impl IdMap {
    pub fn bump_path(&mut self, old_id: &str) -> String {
        self.id_map.entry(old_id.to_string()).or_insert_with(|| {
            let id_counters = &mut self.id_counters;
            id_counters.path += 1;
            format!("p{}", id_counters.path)
        }).clone()
    }

    pub fn bump_group(&mut self, old_id: &str) -> String {
        self.id_map.entry(old_id.to_string()).or_insert_with(|| {
            let id_counters = &mut self.id_counters;
            id_counters.group += 1;
            format!("g{}", id_counters.group)
        }).clone()
    }

    pub fn bump_mask(&mut self, old_id: &str) -> String {
        self.id_map.entry(old_id.to_string()).or_insert_with(|| {
            let id_counters = &mut self.id_counters;
            id_counters.mask += 1;
            format!("m{}", id_counters.mask)
        }).clone()
    }

    pub fn bump_clip_path(&mut self, old_id: &str) -> String {
        self.id_map.entry(old_id.to_string()).or_insert_with(|| {
            let id_counters = &mut self.id_counters;
            id_counters.clip_path += 1;
            format!("cp{}", id_counters.clip_path)
        }).clone()
    }

    pub fn bump_pattern(&mut self, old_id: &str) -> String {
        self.id_map.entry(old_id.to_string()).or_insert_with(|| {
            let id_counters = &mut self.id_counters;
            id_counters.pattern += 1;
            format!("pat{}", id_counters.pattern)
        }).clone()
    }

    pub fn bump_radial_gradient(&mut self, old_id: &str) -> String {
        self.id_map.entry(old_id.to_string()).or_insert_with(|| {
            let id_counters = &mut self.id_counters;
            id_counters.radial_gradient += 1;
            format!("rg{}", id_counters.radial_gradient)
        }).clone()
    }

    pub fn bump_linear_gradient(&mut self, old_id: &str) -> String {
        self.id_map.entry(old_id.to_string()).or_insert_with(|| {
            let id_counters = &mut self.id_counters;
            id_counters.linear_gradient += 1;
            format!("lg{}", id_counters.linear_gradient)
        }).clone()
    }

    pub fn bump_image(&mut self, old_id: &str) -> String {
        self.id_map.entry(old_id.to_string()).or_insert_with(|| {
            let id_counters = &mut self.id_counters;
            id_counters.image += 1;
            format!("i{}", id_counters.image)
        }).clone()
    }
}

impl IdRemapping for usvg_tree::Tree {
    fn remap_ids(&mut self) {
        remap_ids_impl(&self.root, &mut IdMap::default());
    }
}

fn remap_ids_impl(node: &Node, map: &mut IdMap) {
    for node in node.descendants() {
        match *node.borrow_mut() {
            NodeKind::Group(ref mut group) => {
                group.id = map.bump_group(&group.id);
            },
            _ => {}
        }
    }
}

pub fn print_node(root: &Node) {
    root.descendants().for_each(|n| println!("{:#?}", n.borrow()));
}
