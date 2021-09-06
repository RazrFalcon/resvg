// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::svgtree::{self, AId};
use crate::{FilterInput, FilterKind, FilterPrimitive};

/// A tile filter primitive.
///
/// `feTile` element in the SVG.
#[derive(Clone, Debug)]
pub struct FeTile {
    /// Identifies input for the given filter primitive.
    ///
    /// `in` in the SVG.
    pub input: FilterInput,
}

pub(crate) fn convert(fe: svgtree::Node, primitives: &[FilterPrimitive]) -> FilterKind {
    FilterKind::FeTile(FeTile {
        input: super::resolve_input(fe, AId::In, primitives),
    })
}
