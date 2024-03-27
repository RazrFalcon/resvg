// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::Text;

pub mod layout;
pub mod old;

pub(crate) fn convert(text: &mut Text, fontdb: &fontdb::Database) -> Option<()> {
    old::convert(text, fontdb)
}
