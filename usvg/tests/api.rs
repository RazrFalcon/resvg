extern crate usvg;
extern crate rustc_version;

use std::mem;

use rustc_version::{Version, version_meta};

#[test]
fn node_kind_size_1() {
    let size = if version_meta().unwrap().semver == Version::parse("1.22.0").unwrap() {
        264
    } else {
        // Newer rust versions has a better enum packing.
        256
    };
    assert!(mem::size_of::<usvg::NodeKind>() <= size);
}
