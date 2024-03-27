// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::{collections::HashMap, sync::Arc};

use crate::{ImageRendering, ShapeRendering, Size, TextRendering, Tree};

use super::image::PreloadedImageData;

/// Processing options.
#[derive(Debug)]
pub struct Options<'a> {
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
    pub dpi: f32,

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
    pub font_size: f32,

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

    /// Default viewport size to assume if there is no `viewBox` attribute and
    /// the `width` or `height` attributes are relative.
    ///
    /// Default: `(100, 100)`
    pub default_size: Size,

    /// FFRAMES requirement
    /// Preloded decoded image data
    pub image_data: Option<&'a HashMap<String, Arc<PreloadedImageData>>>,

    /// FFRAMES requirement
    /// Preloaded decoded SVG data
    pub sub_svg_data: Option<&'a HashMap<String, Arc<Tree>>>,
}

impl Default for Options<'_> {
    fn default() -> Options<'static> {
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
            default_size: Size::from_wh(100.0, 100.0).unwrap(),
            image_data: None,
            sub_svg_data: None,
        }
    }
}

impl Options<'_> {
    /// Converts a relative path into absolute relative to the SVG file itself.
    ///
    /// If `Options::resources_dir` is not set, returns itself.
    pub fn get_abs_path(&self, rel_path: &std::path::Path) -> std::path::PathBuf {
        match self.resources_dir {
            Some(ref dir) => dir.join(rel_path),
            None => rel_path.into(),
        }
    }
}
