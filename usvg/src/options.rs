// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{ImageHrefResolver, ImageRendering, ScreenSize, ShapeRendering, Size, TextRendering};

/// Image fit options.
///
/// All variants will preserve the original aspect.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FitTo {
    /// Keep original size.
    Original,
    /// Scale to width.
    Width(u32),
    /// Scale to height.
    Height(u32),
    /// Scale to size.
    Size(u32, u32),
    /// Zoom by factor.
    Zoom(f32),
}

impl FitTo {
    /// Returns `size` preprocessed according to `FitTo`.
    pub fn fit_to(&self, size: ScreenSize) -> Option<ScreenSize> {
        let sizef = size.to_size();

        match *self {
            FitTo::Original => Some(size),
            FitTo::Width(w) => {
                let h = (w as f64 * sizef.height() / sizef.width()).ceil();
                ScreenSize::new(w, h as u32)
            }
            FitTo::Height(h) => {
                let w = (h as f64 * sizef.width() / sizef.height()).ceil();
                ScreenSize::new(w as u32, h)
            }
            FitTo::Size(w, h) => Some(
                sizef
                    .scale_to(Size::new(w as f64, h as f64)?)
                    .to_screen_size(),
            ),
            FitTo::Zoom(z) => Size::new(sizef.width() * z as f64, sizef.height() * z as f64)
                .map(|s| s.to_screen_size()),
        }
    }
}

/// Processing options.
#[derive(Debug)]
pub struct Options {
    /// Directory that will be used during relative paths resolving.
    ///
    /// Expected to be the same as the directory that contains the SVG file,
    /// but can be set to any.
    ///
    /// Default: `None`
    pub resources_dir: Option<std::path::PathBuf>,

    /// Target DPI.
    ///
    /// Impacts units conversion.
    ///
    /// Default: 96.0
    pub dpi: f64,

    /// A default font family.
    ///
    /// Will be used when no `font-family` attribute is set in the SVG.
    ///
    /// Default: Times New Roman
    pub font_family: String,

    /// A default font size.
    ///
    /// Will be used when no `font-size` attribute is set in the SVG.
    ///
    /// Default: 12
    pub font_size: f64,

    /// A list of languages.
    ///
    /// Will be used to resolve a `systemLanguage` conditional attribute.
    ///
    /// Format: en, en-US.
    ///
    /// Default: `[en]`
    pub languages: Vec<String>,

    /// Specifies the default shape rendering method.
    ///
    /// Will be used when an SVG element's `shape-rendering` property is set to `auto`.
    ///
    /// Default: GeometricPrecision
    pub shape_rendering: ShapeRendering,

    /// Specifies the default text rendering method.
    ///
    /// Will be used when an SVG element's `text-rendering` property is set to `auto`.
    ///
    /// Default: OptimizeLegibility
    pub text_rendering: TextRendering,

    /// Specifies the default image rendering method.
    ///
    /// Will be used when an SVG element's `image-rendering` property is set to `auto`.
    ///
    /// Default: OptimizeQuality
    pub image_rendering: ImageRendering,

    /// Keep named groups.
    ///
    /// If set to `true`, all non-empty groups with `id` attribute will not
    /// be removed.
    ///
    /// Default: false
    pub keep_named_groups: bool,

    /// Default viewport size to assume if there is no `viewBox` attribute and
    /// the `width` or `height` attributes are relative.
    ///
    /// Default: `(100, 100)`
    pub default_size: Size,

    /// Specifies the way `xlink:href` in `<image>` elements should be handled.
    ///
    /// Default: see type's documentation for details
    pub image_href_resolver: ImageHrefResolver,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            resources_dir: None,
            dpi: 96.0,
            // Default font is user-agent dependent so we can use whichever we like.
            font_family: "Times New Roman".to_owned(),
            font_size: 12.0,
            languages: vec!["en".to_string()],
            shape_rendering: ShapeRendering::default(),
            text_rendering: TextRendering::default(),
            image_rendering: ImageRendering::default(),
            keep_named_groups: false,
            default_size: Size::new(100.0, 100.0).unwrap(),
            image_href_resolver: ImageHrefResolver::default(),
        }
    }
}

impl Options {
    /// Converts a relative path into absolute relative to the SVG file itself.
    ///
    /// If `OptionsRef::resources_dir` is not set, returns itself.
    pub fn get_abs_path(&self, rel_path: &std::path::Path) -> std::path::PathBuf {
        match self.resources_dir {
            Some(ref dir) => dir.join(rel_path),
            None => rel_path.into(),
        }
    }
}
