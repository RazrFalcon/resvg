// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::path::PathBuf;

use svgdom::{
    Color,
};


/// Image fit options.
#[derive(Debug, Copy, Clone, PartialEq)]
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
    /// SVG image path.
    ///
    /// Used to resolve relative image paths.
    pub path: Option<PathBuf>,
    /// Target DPI.
    ///
    /// Impact units converting and text rendering.
    pub dpi: f64,
    /// Fits image to the specified height.
    ///
    /// Does not affect rendering to canvas.
    pub fit_to: FitTo,
    /// Image background color.
    ///
    /// Sets image background color. Does not affect rendering to canvas.
    ///
    /// `None` equals to transparent.
    pub background: Option<Color>,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            path: None,
            dpi: 96.0,
            fit_to: FitTo::Original,
            background: None,
        }
    }
}
