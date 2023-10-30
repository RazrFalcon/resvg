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
use std::rc::Rc;
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
        let mut remapped_tree = usvg_tree::Tree {
            root: self.root.make_deep_copy(),
            size: self.size,
            view_box: self.view_box
        };
        let mut id_map = IdMap::default();
        remapped_tree.remap_ids(&mut id_map);
        writer::convert(& remapped_tree, opt)
    }
}

pub(crate) trait IdRemapping {
    fn remap_ids(&mut self, id_map: &mut IdMap);
}

#[derive(Default)]
pub(crate) struct IdMap {
    id_map: HashMap<String, String>,
    path: u64,
    group: u64,
    text: u64,
    filter: u64,
    mask: u64,
    clip_path: u64,
    pattern: u64,
    radial_gradient: u64,
    linear_gradient: u64,
    image: u64
}

impl IdMap {
    pub fn bump_path(&mut self, old_id: &str) -> String {
        IdMap::bump_impl(old_id, &mut self.id_map, &mut self.path, "p")
    }

    pub fn bump_group(&mut self, old_id: &str) -> String {
        IdMap::bump_impl(old_id, &mut self.id_map, &mut self.group, "g")
    }

    pub fn bump_text(&mut self, old_id: &str) -> String {
        IdMap::bump_impl(old_id, &mut self.id_map, &mut self.text, "t")
    }

    pub fn bump_filter(&mut self, old_id: &str) -> String {
        IdMap::bump_impl(old_id, &mut self.id_map, &mut self.filter, "f")
    }

    pub fn bump_mask(&mut self, old_id: &str) -> String {
        IdMap::bump_impl(old_id, &mut self.id_map, &mut self.mask, "m")
    }

    pub fn bump_clip_path(&mut self, old_id: &str) -> String {
        IdMap::bump_impl(old_id, &mut self.id_map, &mut self.clip_path, "cp")
    }

    pub fn bump_pattern(&mut self, old_id: &str) -> String {
        IdMap::bump_impl(old_id, &mut self.id_map, &mut self.pattern, "pat")
    }

    pub fn bump_radial_gradient(&mut self, old_id: &str) -> String {
        IdMap::bump_impl(old_id, &mut self.id_map, &mut self.radial_gradient, "rg")
    }

    pub fn bump_linear_gradient(&mut self, old_id: &str) -> String {
        IdMap::bump_impl(old_id, &mut self.id_map, &mut self.linear_gradient, "lg")
    }

    pub fn bump_image(&mut self, old_id: &str) -> String {
        IdMap::bump_impl(old_id, &mut self.id_map, &mut self.image, "i")
    }

    fn bump_impl(old_id: &str, id_map: &mut HashMap<String, String>, field: &mut u64, format_str: &str) -> String {
        let mut bump = || {
            *field += 1;
            let result = format!("{}{}", format_str, field);
            result
        };

        if !old_id.is_empty() {
            id_map.entry(old_id.to_string()).or_insert_with(bump).clone()
        }   else {
            bump()
        }
    }
}

impl IdRemapping for usvg_tree::Tree {
    fn remap_ids(&mut self, id_map: &mut IdMap) {
        remap_ids_impl(&self.root, id_map);
    }
}

fn remap_ids_impl(node: &Node, map: &mut IdMap) {
    let map_paint = |map: &mut IdMap, paint: Paint| {
        let paint = match paint {
            Paint::Color(_) => paint,
            Paint::LinearGradient(lg) => {
                let mut new_lg = (*lg).clone();
                new_lg.id = map.bump_linear_gradient(&lg.id);
                Paint::LinearGradient(Rc::new(new_lg))
            },
            Paint::RadialGradient(rg) => {
                let mut new_rg = (*rg).clone();
                new_rg.id = map.bump_radial_gradient(&rg.id);
                Paint::RadialGradient(Rc::new(new_rg))
            }
            Paint::Pattern(pattern) => {
                let mut new_pattern = (*pattern).clone();
                new_pattern.id = map.bump_pattern(&pattern.id);
                Paint::Pattern(Rc::new(new_pattern))
            }
            _ => paint
        };
        paint
    };

    for node in node.descendants() {
        match *node.borrow_mut() {
            NodeKind::Path(ref mut path) => {
                path.id = map.bump_path(&path.id);
                path.fill = path.fill.clone().map(|mut f| {
                    f.paint = map_paint(map, f.paint);
                    f
                });
                path.stroke = path.stroke.clone().map(|mut s| {
                    s.paint = map_paint(map, s.paint);
                    s
                });
            },
            NodeKind::Image(ref mut image) => {
                image.id = map.bump_image(&image.id);
            },
            NodeKind::Text(ref mut text) => {
                text.id = map.bump_text(&text.id);
                for chunk in &mut text.chunks {
                    for span in &mut chunk.spans {
                        span.fill = span.fill.clone().map(|mut f| {
                            f.paint = map_paint(map, f.paint);
                            f
                        });
                        span.stroke = span.stroke.clone().map(|mut s| {
                            s.paint = map_paint(map, s.paint);
                            s
                        });
                    }
                }
            },
            NodeKind::Group(ref mut group) => {
                group.id = map.bump_group(&group.id);
                let filters = group.filters.iter().map(|filter| {
                    let mut new_filter = (**filter).clone();
                    new_filter.id = map.bump_filter(&filter.id);
                    Rc::new(new_filter)
                }).collect();
                group.filters = filters;

                group.mask = group.mask.as_ref().map(|mask| {
                    let mut new_mask = (**mask).clone();
                    new_mask.id = map.bump_mask(&mask.id);
                    new_mask.mask = new_mask.mask.as_ref().map(|mask| {
                        let mut new_mask = (**mask).clone();
                        new_mask.id = map.bump_mask(&mask.id);
                        Rc::new(new_mask)
                    });

                    Rc::new(new_mask)
                });

                group.clip_path = group.clip_path.as_ref().map(|clip_path| {
                    let mut new_clip_path = (**clip_path).clone();
                    new_clip_path.id = map.bump_clip_path(&clip_path.id);
                    new_clip_path.clip_path = new_clip_path.clip_path.as_ref().map(|clip_path| {
                        let mut new_clip_path = (**clip_path).clone();
                        new_clip_path.id = map.bump_clip_path(&clip_path.id);
                        Rc::new(new_clip_path)
                    });

                    Rc::new(new_clip_path)
                });
            }
        }

        node.subroots(|node| remap_ids_impl(&node, map));
    }
}
