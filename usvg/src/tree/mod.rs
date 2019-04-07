// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Implementation of the nodes tree.

extern crate rctree;

use std::cell::Ref;
use std::path;

// external
use svgdom;
use libflate;

// self
pub use self::nodes::*;
pub use self::attributes::*;
use {
    Error,
    Options,
};

mod attributes;
mod export;
mod nodes;
mod numbers;

/// Basic traits for tree manipulations.
pub mod prelude {
    pub use IsDefault;
    pub use IsValidLength;
    pub use tree::FuzzyEq;
    pub use tree::FuzzyZero;
    pub use super::NodeExt;
}

/// Alias for `rctree::Node<NodeKind>`.
pub type Node = rctree::Node<NodeKind>;

// TODO: impl a Debug
/// A nodes tree container.
#[allow(missing_debug_implementations)]
#[derive(Clone)]
pub struct Tree {
    root: Node,
}

impl Tree {
    /// Parses `Tree` from the SVG data.
    ///
    /// Can contain an SVG string or a gzip compressed data.
    pub fn from_data(data: &[u8], opt: &Options) -> Result<Self, Error> {
        if data.starts_with(&[0x1f, 0x8b]) {
            let text = deflate(data, data.len())?;
            Self::from_str(&text, opt)
        } else {
            let text = ::std::str::from_utf8(data).map_err(|_| Error::NotAnUtf8Str)?;
            Self::from_str(text, opt)
        }
    }

    /// Parses `Tree` from the SVG string.
    pub fn from_str(text: &str, opt: &Options) -> Result<Self, Error> {
        let dom_opt = svgdom::ParseOptions {
            skip_invalid_attributes: true,
            skip_invalid_css: true,
            skip_unresolved_classes: true,
        };

        let doc = svgdom::Document::from_str_with_opt(text, &dom_opt)
            .map_err(|e| Error::ParsingFailed(e))?;

        Self::from_dom(doc, &opt)
    }

    /// Parses `Tree` from the `svgdom::Document`.
    ///
    /// An empty `Tree` will be returned on any error.
    pub fn from_dom(mut doc: svgdom::Document, opt: &Options) -> Result<Self, Error> {
        super::convert::prepare_doc(&mut doc);
        super::convert::convert_doc(&doc, opt)
    }

    /// Parses `Tree` from the file.
    pub fn from_file<P: AsRef<path::Path>>(
        path: P,
        opt: &Options,
    ) -> Result<Self, Error> {
        let text = load_svg_file(path.as_ref())?;
        Self::from_str(&text, opt)
    }

    /// Creates a new `Tree`.
    pub fn create(svg: Svg) -> Self {
        let mut root_node = Node::new(NodeKind::Svg(svg));
        let defs_node = Node::new(NodeKind::Defs);
        root_node.append(defs_node);

        Tree {
            root: root_node,
        }
    }

    /// Returns the `Svg` node.
    pub fn root(&self) -> Node {
        self.root.clone()
    }

    /// Returns the `Svg` node value.
    pub fn svg_node(&self) -> Ref<Svg> {
        Ref::map(self.root.borrow(), |v| {
            match *v {
                NodeKind::Svg(ref svg) => svg,
                _ => unreachable!(),
            }
        })
    }

    /// Returns the `Defs` node.
    pub fn defs(&self) -> Node {
        self.root.first_child().unwrap()
    }

    /// Checks that `node` is part of the `Defs` children.
    pub fn is_in_defs(&self, node: &Node) -> bool {
        let defs = self.defs();
        node.ancestors().any(|n| n == defs)
    }

    /// Appends `NodeKind` to the `Defs` node.
    pub fn append_to_defs(&mut self, kind: NodeKind) -> Node {
        debug_assert!(self.defs_by_id(kind.id()).is_none(),
                      "Element #{} already exists in 'defs'.", kind.id());

        let new_node = Node::new(kind);
        self.defs().append(new_node.clone());
        new_node
    }

    /// Returns `defs` child node by ID.
    pub fn defs_by_id(&self, id: &str) -> Option<Node> {
        for n in self.defs().children() {
            if &*n.id() == id {
                return Some(n);
            }
        }

        None
    }

    /// Returns renderable node by ID.
    ///
    /// If an empty ID is provided, than this method will always return `None`.
    /// Even if tree has nodes with empty ID.
    pub fn node_by_id(&self, id: &str) -> Option<Node> {
        if id.is_empty() {
            return None;
        }

        for node in self.root().descendants() {
            if !self.is_in_defs(&node) {
                if &*node.id() == id {
                    return Some(node);
                }
            }
        }

        None
    }

    /// Converts the document to `svgdom::Document`.
    ///
    /// Used to save document to file for debug purposes.
    pub fn to_svgdom(&self) -> svgdom::Document {
        export::convert(self)
    }
}

/// Additional `Node` methods.
pub trait NodeExt {
    /// Returns node's ID.
    ///
    /// If a current node doesn't support ID - an empty string
    /// will be returned.
    fn id(&self) -> Ref<str>;

    /// Returns node's transform.
    ///
    /// If a current node doesn't support transformation - a default
    /// transform will be returned.
    fn transform(&self) -> Transform;

    /// Appends `kind` as a node child.
    ///
    /// Shorthand for `Node::append(Node::new(Box::new(kind)))`.
    fn append_kind(&mut self, kind: NodeKind) -> Node;

    /// Returns a node's tree.
    fn tree(&self) -> Tree;
}

impl NodeExt for Node {
    fn id(&self) -> Ref<str> {
        Ref::map(self.borrow(), |v| v.id())
    }

    fn transform(&self) -> Transform {
        self.borrow().transform()
    }

    fn append_kind(&mut self, kind: NodeKind) -> Node {
        let new_node = Node::new(kind);
        self.append(new_node.clone());
        new_node
    }

    fn tree(&self) -> Tree {
        Tree { root: self.root() }
    }
}


/// Loads SVG, SVGZ file content.
pub fn load_svg_file(path: &path::Path) -> Result<String, Error> {
    use std::fs;
    use std::io::Read;
    use std::path::Path;

    let mut file = fs::File::open(path).map_err(|_| Error::FileOpenFailed)?;
    let length = file.metadata().map_err(|_| Error::FileOpenFailed)?.len() as usize + 1;

    let ext = if let Some(ext) = Path::new(path).extension() {
        ext.to_str().map(|s| s.to_lowercase()).unwrap_or_default()
    } else {
        String::new()
    };

    match ext.as_str() {
        "svgz" => {
            deflate(&file, length)
        }
        "svg" => {
            let mut s = String::with_capacity(length);
            file.read_to_string(&mut s).map_err(|_| Error::NotAnUtf8Str)?;
            Ok(s)
        }
        _ => {
            Err(Error::InvalidFileSuffix)
        }
    }
}

fn deflate<R: ::std::io::Read>(inner: R, len: usize) -> Result<String, Error> {
    use std::io::Read;

    let mut decoder = libflate::gzip::Decoder::new(inner).map_err(|_| Error::MalformedGZip)?;
    let mut decoded = String::with_capacity(len * 2);
    decoder.read_to_string(&mut decoded).map_err(|_| Error::NotAnUtf8Str)?;
    Ok(decoded)
}

