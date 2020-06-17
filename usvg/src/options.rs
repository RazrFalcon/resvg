// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::path::PathBuf;

use crate::{ImageRendering, ShapeRendering, TextRendering, Size, ScreenSize};


/// Image fit options.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FitTo {
    /// Keep original size.
    Original,
    /// Scale to width.
    Width(u32),
    /// Scale to height.
    Height(u32),
    /// Zoom by factor.
    Zoom(f32),
}

impl FitTo {
    /// Returns `size` preprocessed according to `FitTo`.
    pub fn fit_to(&self, size: ScreenSize) -> Option<ScreenSize> {
        let sizef = size.to_size();

        match *self {
            FitTo::Original => {
                Some(size)
            }
            FitTo::Width(w) => {
                let h = (w as f64 * sizef.height() / sizef.width()).ceil();
                ScreenSize::new(w, h as u32)
            }
            FitTo::Height(h) => {
                let w = (h as f64 * sizef.width() / sizef.height()).ceil();
                ScreenSize::new(w as u32, h)
            }
            FitTo::Zoom(z) => {
                Size::new(sizef.width() * z as f64, sizef.height() * z as f64)
                    .map(|s| s.to_screen_size())
            }
        }
    }
}


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

    /// Specifies the default shape rendering method.
    ///
    /// Will be used when an SVG element's `shape-rendering` property is set to `auto`.
    pub shape_rendering: ShapeRendering,

    /// Specifies the default text rendering method.
    ///
    /// Will be used when an SVG element's `text-rendering` property is set to `auto`.
    pub text_rendering: TextRendering,

    /// Specifies the default image rendering method.
    ///
    /// Will be used when an SVG element's `image-rendering` property is set to `auto`.
    pub image_rendering: ImageRendering,

    /// Keep named groups.
    ///
    /// If set to `true`, all non-empty groups with `id` attribute will not
    /// be removed.
    pub keep_named_groups: bool,
}

impl Options {
    /// Converts a relative path into absolute relative to the SVG file itself.
    ///
    /// If `Options::path` is not set, returns itself.
    pub fn get_abs_path(&self, rel_path: &std::path::Path) -> std::path::PathBuf {
        match self.path {
            Some(ref path) => path.parent().unwrap().join(rel_path),
            None => rel_path.into(),
        }
    }
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
            shape_rendering: ShapeRendering::default(),
            text_rendering: TextRendering::default(),
            image_rendering: ImageRendering::default(),
            keep_named_groups: false,
        }
    }
}
