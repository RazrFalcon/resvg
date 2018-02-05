// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Implementation of the rendering tree.

use std::fmt;

// external
use svgdom;

// self
pub use self::node::*;
pub use self::attribute::*;


mod attribute;
mod dump;
mod node;


pub(crate) const DEFS_DEPTH: usize = 2;
const DEFS_IDX: usize = 1;


struct NodeData {
    depth: usize,
    kind: NodeKind,
}


/// Container for a preprocessed SVG.
///
/// Unlike svgdom's `Document` this one is immutable for a backend code
/// and contains only supported, resolved elements and attributes.
pub struct RenderTree {
    nodes: Vec<NodeData>,
}

impl RenderTree {
    /// Creates a new `RenderTree`.
    pub fn new(svg: Svg) -> Self {
        let mut doc = RenderTree {
            nodes: Vec::new(),
        };

        doc.nodes.push(NodeData {
            depth: 0,
            kind: NodeKind::Svg(svg),
        });

        doc.nodes.push(NodeData {
            depth: 1,
            kind: NodeKind::Defs,
        });

        doc
    }

    /// Returns the root node.
    pub fn root(&self) -> NodeRef {
        self.node_at(0)
    }

    /// Returns the `svg` node data.
    pub fn svg_node(&self) -> &Svg {
        if let NodeKind::Svg(ref svg) = self.nodes[0].kind {
            svg
        } else {
            unreachable!();
        }
    }

    /// Returns the `defs` node.
    pub fn defs(&self) -> DefsChildren {
        DefsChildren {
            nodes: &self.nodes,
            idx: DEFS_IDX + 1,
        }
    }

    pub(crate) fn append_node(&mut self, depth: usize, kind: NodeKind) -> usize {
        self.nodes.push(NodeData {
            depth,
            kind,
        });

        self.nodes.len() - 1
    }

    pub(crate) fn remove_node(&mut self, idx: usize) {
        self.nodes.truncate(idx);
    }

    pub(crate) fn node_at(&self, idx: usize) -> NodeRef {
        NodeRef {
            nodes: &self.nodes,
            idx: idx,
        }
    }

    pub(crate) fn defs_at(&self, idx: usize) -> DefsNodeRef {
        self.defs().nth(idx).unwrap()
    }

    pub(crate) fn defs_index(&self, id: &str) -> Option<usize> {
        self.defs().position(|e| e.kind().id() == id)
    }

    /// Converts the document to `svgdom::Document`.
    ///
    /// Used to save document to file for debug purposes.
    pub fn to_svgdom(&self) -> svgdom::Document {
        dump::conv_doc(self)
    }
}

impl fmt::Debug for RenderTree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Document start")?;
        for node in &self.nodes {
            for _ in 0..node.depth {
                write!(f, " ")?;
            }

            match node.kind {
                NodeKind::Svg(_) => writeln!(f, "Svg")?,
                NodeKind::Defs => writeln!(f, "Defs")?,
                NodeKind::LinearGradient(_) => writeln!(f, "LinearGradient")?,
                NodeKind::RadialGradient(_) => writeln!(f, "RadialGradient")?,
                NodeKind::Stop(_) => writeln!(f, "Stop")?,
                NodeKind::ClipPath(_) => writeln!(f, "ClipPath")?,
                NodeKind::Pattern(_) => writeln!(f, "Pattern")?,
                NodeKind::Path(_) => writeln!(f, "Path")?,
                NodeKind::Text(_) => writeln!(f, "Text")?,
                NodeKind::TextChunk(_) => writeln!(f, "TextChunk")?,
                NodeKind::TSpan(_) => writeln!(f, "TSpan")?,
                NodeKind::Image(_) => writeln!(f, "Image")?,
                NodeKind::Group(_) => writeln!(f, "Group")?,
            }
        }

        writeln!(f, "Document end")
    }
}


/// A reference to a `Document` node.
#[derive(Clone, Copy)]
pub struct NodeRef<'a> {
    nodes: &'a [NodeData],
    idx: usize,
}

impl<'a> NodeRef<'a> {
    /// Returns node's kind.
    pub fn kind(&self) -> NodeKindRef {
        match self.nodes[self.idx].kind {
            NodeKind::Path(ref e) => NodeKindRef::Path(e),
            NodeKind::Text(ref e) => NodeKindRef::Text(e),
            NodeKind::Image(ref e) => NodeKindRef::Image(e),
            NodeKind::Group(ref e) => NodeKindRef::Group(e),
            _ => unreachable!(),
        }
    }

    /// Returns an iterator over children nodes.
    ///
    /// Will return only `path`, `text`, `image` and `g`.
    pub fn children(&self) -> Children {
        Children {
            nodes: &self.nodes,
            depth: self.nodes[self.idx].depth + 1,
            idx: self.idx + 1,
        }
    }

    /// Returns an iterator over text chunks.
    ///
    /// # Panic
    ///
    /// Will panic if the current node is not a `Text` node.
    pub fn text_chunks(&self) -> TextChunks {
        if let NodeKind::Text(_) = self.nodes[self.idx].kind {
        } else {
            panic!("must be invoked only on Text node");
        }

        TextChunks {
            nodes: &self.nodes,
            depth: self.nodes[self.idx].depth + 1,
            idx: self.idx + 1,
        }
    }

    /// Returns an iterator over text spans.
    ///
    /// # Panic
    ///
    /// Will panic if the current node is not a `TextChunk` node.
    pub fn text_spans(&self) -> TextSpans {
        if let NodeKind::TextChunk(_) = self.nodes[self.idx].kind {
        } else {
            panic!("must be invoked only on TextChunk node");
        }

        TextSpans {
            nodes: &self.nodes,
            depth: self.nodes[self.idx].depth + 1,
            idx: self.idx + 1,
        }
    }
}


/// A reference to a defs child node.
///
/// Will contain only nodes that can be represented as `DefsNodeKindRef`.
#[derive(Clone, Copy)]
pub struct DefsNodeRef<'a> {
    nodes: &'a [NodeData],
    idx: usize,
}

impl<'a> DefsNodeRef<'a> {
    /// Converts this node to `NodeRef`.
    pub fn to_node_ref(self) -> NodeRef<'a> {
        NodeRef {
            nodes: self.nodes,
            idx: self.idx,
        }
    }

    /// Returns node's kind.
    pub fn kind(&self) -> DefsNodeKindRef {
        match self.nodes[self.idx].kind {
            NodeKind::LinearGradient(ref e) => DefsNodeKindRef::LinearGradient(e),
            NodeKind::RadialGradient(ref e) => DefsNodeKindRef::RadialGradient(e),
            NodeKind::ClipPath(ref e) => DefsNodeKindRef::ClipPath(e),
            NodeKind::Pattern(ref e) => DefsNodeKindRef::Pattern(e),
            _ => unreachable!(),
        }
    }

    /// Returns an iterator over children nodes.
    ///
    /// Will return only `path`, `text`, `image` and `g`.
    ///
    /// # Panic
    ///
    /// Will panic if the current node is not a `ClipPath` or a `Pattern`.
    pub fn children(&self) -> Children {
        match self.nodes[self.idx].kind {
              NodeKind::ClipPath(_)
            | NodeKind::Pattern(_) => {}
            _ => panic!("must be invoked only on nodes with shape-based children nodes"),
        }

        Children {
            nodes: &self.nodes,
            depth: self.nodes[self.idx].depth + 1,
            idx: self.idx + 1,
        }
    }

    /// Returns an iterator over `Stop` nodes.
    ///
    /// Will return only `stop`.
    ///
    /// # Panic
    ///
    /// Will panic if the current node is not a gradient.
    pub fn stops(&self) -> Stops {
        Stops {
            nodes: self.nodes,
            idx: self.idx + 1,
        }
    }
}


/// An iterator of `NodeRef`s to the children of a given node.
///
/// Returns only nodes that can be represented via `NodeKindRef`.
pub struct Children<'a> {
    nodes: &'a [NodeData],
    depth: usize,
    idx: usize,
}

impl<'a> Iterator for Children<'a> {
    type Item = NodeRef<'a>;

    fn next(&mut self) -> Option<NodeRef<'a>> {
        debug_assert!(self.idx != 0);

        if self.idx == self.nodes.len() {
            return None;
        }

        let n = &self.nodes[self.idx];

        if n.depth < self.depth {
            return None;
        }

        let is_valid = n.depth == self.depth && n.kind.is_shape();
        if !is_valid {
            self.idx += 1;
            return self.next();
        }

        let idx = self.idx;
        self.idx += 1;

        Some(NodeRef {
            nodes: self.nodes,
            idx: idx,
        })
    }
}


/// An iterator of `DefsNodeRef`s to the children of a `defs` node.
pub struct DefsChildren<'a> {
    nodes: &'a [NodeData],
    idx: usize,
}

impl<'a> Iterator for DefsChildren<'a> {
    type Item = DefsNodeRef<'a>;

    fn next(&mut self) -> Option<DefsNodeRef<'a>> {
        if self.idx == self.nodes.len() {
            return None;
        }

        let n = &self.nodes[self.idx];

        if n.depth < DEFS_DEPTH {
            return None;
        }

        let is_valid = n.depth == DEFS_DEPTH;
        if !is_valid {
            self.idx += 1;
            return self.next();
        }

        let idx = self.idx;
        self.idx += 1;

        Some(DefsNodeRef {
            nodes: self.nodes,
            idx: idx,
        })
    }
}


/// An iterator of `Stop`s to the children of a gradient node.
pub struct Stops<'a> {
    nodes: &'a [NodeData],
    idx: usize,
}

impl<'a> Iterator for Stops<'a> {
    type Item = Stop;

    fn next(&mut self) -> Option<Stop> {
        if self.idx == self.nodes.len() {
            return None;
        }

        if self.nodes[self.idx].depth != DEFS_DEPTH + 1 {
            return None;
        }

        if let NodeKind::Stop(s) = self.nodes[self.idx].kind {
            self.idx += 1;
            Some(s)
        } else {
            None
        }
    }
}


/// An iterator of `TextChunk`s to the children of a `Text` node.
pub struct TextChunks<'a> {
    nodes: &'a [NodeData],
    depth: usize,
    idx: usize,
}

impl<'a> Iterator for TextChunks<'a> {
    type Item = (NodeRef<'a>, &'a TextChunk);

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx == self.nodes.len() {
            return None;
        }

        let n = &self.nodes[self.idx];

        if n.depth < self.depth {
            return None;
        }

        let is_valid = n.depth == self.depth;
        if !is_valid {
            self.idx += 1;
            return self.next();
        }

        if let NodeKind::TextChunk(ref s) = self.nodes[self.idx].kind {
            let n = NodeRef {
                nodes: self.nodes,
                idx: self.idx,
            };

            self.idx += 1;

            Some((n, s))
        } else {
            None
        }
    }
}


/// An iterator of `TSpan`s to the children of a `TextChunk` node.
pub struct TextSpans<'a> {
    nodes: &'a [NodeData],
    depth: usize,
    idx: usize,
}

impl<'a> Iterator for TextSpans<'a> {
    type Item = &'a TSpan;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx == self.nodes.len() {
            return None;
        }

        if self.nodes[self.idx].depth != self.depth {
            return None;
        }

        if let NodeKind::TSpan(ref s) = self.nodes[self.idx].kind {
            self.idx += 1;
            Some(s)
        } else {
            None
        }
    }
}
