// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.


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

/// Rendering options.
pub struct Options {
    /// `usvg` preprocessor options.
    pub usvg: usvg::Options,

    /// Fits the image using specified options.
    ///
    /// Does not affect rendering to canvas.
    pub fit_to: FitTo,

    /// An image background color.
    ///
    /// Sets an image background color. Does not affect rendering to canvas.
    ///
    /// `None` equals to transparent.
    pub background: Option<usvg::Color>,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            usvg: usvg::Options::default(),
            fit_to: FitTo::Original,
            background: None,
        }
    }
}
