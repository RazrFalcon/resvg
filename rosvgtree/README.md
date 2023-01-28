# rosvgtree
[![Crates.io](https://img.shields.io/crates/v/rosvgtree.svg)](https://crates.io/crates/rosvgtree)
[![Documentation](https://docs.rs/rosvgtree/badge.svg)](https://docs.rs/rosvgtree)
[![Rust 1.51+](https://img.shields.io/badge/rust-1.51+-orange.svg)](https://www.rust-lang.org)

Represent an [SVG] document as a read-only tree.

Note that while this is a public crate, it's designed with
[usvg](https://github.com/RazrFalcon/resvg/tree/master/usvg) in mind.
You should treat it is as `usvg` internals.

## Purpose

SVG is notoriously hard to parse. And while it is technically an XML superset,
parsing it using just an XML library would be hard.
Therefore we would be better off with a post-processed XML tree.

And this is exactly what `rosvgtree` does.
It creates a [`roxmltree`](https://github.com/RazrFalcon/roxmltree)-like tree,
but tailored to SVG parsing needs.

A complete list of post-processing steps can be found
[here](https://github.com/RazrFalcon/resvg/blob/master/rosvgtree/docs/post-processing.md).

## License

Licensed under either of

- [Apache License v2.0](LICENSE-APACHE)
- [MIT license](LICENSE-MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.


[SVG]: https://www.w3.org/TR/SVG11/Overview.html
