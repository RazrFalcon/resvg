// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::path::PathBuf;


/// Processing options.
#[derive(Clone, Debug)]
pub struct Options {
    /// SVG image path.
    ///
    /// Used to resolve relative image paths.
    pub path: Option<PathBuf>,

    /// Target DPI.
    ///
    /// Impact units conversion.
    pub dpi: f64,

    /// A default font family.
    pub font_family: String,

    /// A default font size.
    pub font_size: f64,

    /// A list of languages that will be used to resolve the `systemLanguage`
    /// conditional attribute.
    ///
    /// Format: en, en-US.
    pub languages: Vec<String>,

    /// Keep named groups.
    ///
    /// If set to `true`, all non-empty groups with `id` attribute will not
    /// be removed.
    pub keep_named_groups: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            path: None,
            dpi: 96.0,
            // Default font is user-agent dependent so we can use whatever we like.
            font_family: "Times New Roman".to_owned(),
            font_size: 12.0,
            languages: vec!["en".to_string()],
            keep_named_groups: false,
        }
    }
}
