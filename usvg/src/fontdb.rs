use std::path::{Path, PathBuf};
use std::fs;

pub use ttf_parser::{GlyphId, Weight, Width as Stretch};

use crate::tree;
use crate::utils;


#[cfg(target_os = "linux")]
const GENERIC_FAMILIES: &[&str] = &["serif", "sans-serif", "monospace", "cursive", "fantasy"];


#[derive(Clone, Debug)]
pub struct FontItem {
    pub id: ID,
    pub path: PathBuf,
    pub face_index: u32,
    pub family: String,
    pub properties: Properties,
}


#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ID(u16); // 65k fonts if more than enough!

pub struct Database {
    fonts: Vec<FontItem>,
    #[allow(dead_code)]
    has_generic_fonts: bool,
}

impl Database {
    pub fn new() -> Self {
        Database {
            fonts: Vec::new(),
            has_generic_fonts: false,
        }
    }

    pub fn populate(&mut self) {
        if self.fonts.is_empty() {
            load_all_fonts(&mut self.fonts);
        }
    }

    #[inline(never)]
    #[cfg(target_os = "linux")]
    fn collect_generic_fonts(&mut self) {
        if self.fonts.is_empty() {
            return;
        }

        for family in GENERIC_FAMILIES {
            self.collect_generic_font(family);
        }

        self.has_generic_fonts = true;
    }

    #[cfg(target_os = "linux")]
    fn collect_generic_font(&mut self, generic_family: &str) -> Option<()> {
        let output = std::process::Command::new("fc-match")
            .arg(generic_family)
            .arg("--format=%{family}")
            .output().ok();
        let output = try_opt_warn_or!(output, None, "Failed to run 'fc-match'.");
        let family = std::str::from_utf8(&output.stdout).ok()?.trim();

        duplicate_family(family, generic_family, &mut self.fonts);

        Some(())
    }

    pub fn font(&self, id: ID) -> &FontItem {
        &self.fonts[id.0 as usize]
    }

    pub fn fonts(&self) -> &[FontItem] {
        &self.fonts
    }

    #[inline(never)]
    pub fn select_best_match(
        &mut self,
        family_names: &[&str],
        properties: Properties,
    ) -> Option<ID> {
        for family_name in family_names {
            // A generic font families querying is very slow on Linux (50-200ms),
            // so do it only when necessary.
            #[cfg(target_os = "linux")]
            {
                if !self.has_generic_fonts && GENERIC_FAMILIES.contains(family_name) {
                    self.collect_generic_fonts();
                }
            }

            let mut ids = Vec::new();
            let mut candidates = Vec::new();
            for item in self.fonts.iter().filter(|font| &font.family == family_name) {
                ids.push(item.id);
                candidates.push(item.properties);
            }

            if let Some(index) = find_best_match(&candidates, properties) {
                return Some(ids[index]);
            }
        }

        None
    }

    #[inline(never)]
    pub fn outline(&self, id: ID, glyph_id: GlyphId) -> Option<tree::PathData> {
        // We can't simplify this code because of lifetimes.
        let item = self.font(id);
        let file = fs::File::open(&item.path).ok()?;
        let mmap = unsafe { memmap2::MmapOptions::new().map(&file).ok()? };
        let font = ttf_parser::Font::from_data(&mmap, item.face_index).ok()?;

        let mut builder = PathBuilder { path: tree::PathData::with_capacity(16) };
        font.outline_glyph(glyph_id, &mut builder).ok()?;
        Some(builder.path)
    }

    #[inline(never)]
    pub fn has_char(&self, id: ID, c: char) -> bool {
        self._has_char(id, c).unwrap_or(false)
    }

    fn _has_char(&self, id: ID, c: char) -> Option<bool> {
        // We can't simplify this code because of lifetimes.
        let item = self.font(id);
        let file = fs::File::open(&item.path).ok()?;
        let mmap = unsafe { memmap2::MmapOptions::new().map(&file).ok()? };
        let font = ttf_parser::Font::from_data(&mmap, item.face_index).ok()?;

        font.glyph_index(c).ok()?;

        Some(true)
    }

    #[inline(never)]
    pub fn load_font(&self, id: ID) -> Option<Font> {
        // We can't simplify this code because of lifetimes.
        let item = self.font(id);
        let file = fs::File::open(&item.path).ok()?;
        let mmap = unsafe { memmap2::MmapOptions::new().map(&file).ok()? };
        let font = ttf_parser::Font::from_data(&mmap, item.face_index).ok()?;

        // Some fonts can have `units_per_em` set to zero, which will break out calculations.
        // `ttf_parser` will check this for us.
        let units_per_em = font.units_per_em()?;

        let ascent = font.ascender();
        let descent = font.descender();

        let x_height = match font.x_height() {
            Some(height) => height,
            None => {
                // If not set - fallback to height * 45%.
                // 45% is what Firefox uses.
                (f32::from(ascent - descent) * 0.45) as i16
            }
        };

        let underline = match font.underline_metrics() {
            Some(metrics) => metrics,
            None => {
                ttf_parser::LineMetrics {
                    position: -(units_per_em as i16) / 9,
                    thickness: units_per_em as i16 / 12,
                }
            }
        };

        let line_through_position = match font.strikeout_metrics() {
            Some(metrics) => metrics.position,
            None => x_height / 2,
        };

        // 0.2 and 0.4 are generic offsets used by some applications (Inkscape/librsvg).
        let mut subscript_offset = (units_per_em as f32 / 0.2).round() as i16;
        let mut superscript_offset = (units_per_em as f32 / 0.4).round() as i16;
        if let Some(metrics) = font.subscript_metrics() {
            subscript_offset = metrics.y_offset;
        }

        if let Some(metrics) = font.superscript_metrics() {
            superscript_offset = metrics.y_offset;
        }

        Some(Font {
            id,
            units_per_em,
            ascent,
            descent,
            x_height,
            underline_position: underline.position,
            underline_thickness: underline.thickness,
            line_through_position,
            subscript_offset,
            superscript_offset,
        })
    }
}


#[derive(Clone, Copy)]
pub struct Font {
    pub id: ID,

    /// Guarantee to be > 0.
    units_per_em: u16,

    // All values below are in font units.
    ascent: i16,
    descent: i16,
    x_height: i16,

    underline_position: i16,
    underline_thickness: i16,

    // line-through thickness should be the the same as underline thickness
    // according to the TrueType spec:
    // https://docs.microsoft.com/en-us/typography/opentype/spec/os2#ystrikeoutsize
    line_through_position: i16,

    subscript_offset: i16,
    superscript_offset: i16,
}

impl Font {
    #[inline]
    pub fn scale(&self, font_size: f64) -> f64 {
        font_size / self.units_per_em as f64
    }

    #[inline]
    pub fn ascent(&self, font_size: f64) -> f64 {
        self.ascent as f64 * self.scale(font_size)
    }

    #[inline]
    pub fn descent(&self, font_size: f64) -> f64 {
        self.descent as f64 * self.scale(font_size)
    }

    #[inline]
    pub fn height(&self, font_size: f64) -> f64 {
        self.ascent(font_size) - self.descent(font_size)
    }

    #[inline]
    pub fn x_height(&self, font_size: f64) -> f64 {
        self.x_height as f64 * self.scale(font_size)
    }

    #[inline]
    pub fn underline_position(&self, font_size: f64) -> f64 {
        self.underline_position as f64 * self.scale(font_size)
    }

    #[inline]
    pub fn underline_thickness(&self, font_size: f64) -> f64 {
        self.underline_thickness as f64 * self.scale(font_size)
    }

    #[inline]
    pub fn line_through_position(&self, font_size: f64) -> f64 {
        self.line_through_position as f64 * self.scale(font_size)
    }

    #[inline]
    pub fn subscript_offset(&self, font_size: f64) -> f64 {
        self.subscript_offset as f64 * self.scale(font_size)
    }

    #[inline]
    pub fn superscript_offset(&self, font_size: f64) -> f64 {
        self.superscript_offset as f64 * self.scale(font_size)
    }
}


#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub struct Properties {
    pub style: Style,
    pub weight: Weight,
    pub stretch: Stretch,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Style {
    Normal,
    Italic,
    Oblique,
}

impl Default for Style {
    fn default() -> Style {
        Style::Normal
    }
}

impl_enum_from_str!(Style,
    "normal"    => Style::Normal,
    "italic"    => Style::Italic,
    "oblique"   => Style::Oblique
);

/// From https://github.com/pcwalton/font-kit
#[inline(never)]
fn find_best_match(
    candidates: &[Properties],
    query: Properties,
) -> Option<usize> {
    let weight = query.weight.to_number();

    // Step 4.
    let mut matching_set: Vec<usize> = (0..candidates.len()).collect();
    if matching_set.is_empty() {
        return None;
    }

    // Step 4a (`font-stretch`).
    let matching_stretch = if matching_set
        .iter()
        .any(|&index| candidates[index].stretch == query.stretch)
    {
        // Exact match.
        query.stretch
    } else if query.stretch <= Stretch::Normal {
        // Closest width, first checking narrower values and then wider values.
        match matching_set
            .iter()
            .filter(|&&index| candidates[index].stretch < query.stretch)
            .min_by_key(|&&index| {
                query.stretch.to_number() - candidates[index].stretch.to_number()
            }) {
            Some(&matching_index) => candidates[matching_index].stretch,
            None => {
                let matching_index = *matching_set
                    .iter()
                    .min_by_key(|&&index| {
                        candidates[index].stretch.to_number() - query.stretch.to_number()
                    })
                    .unwrap();
                candidates[matching_index].stretch
            }
        }
    } else {
        // Closest width, first checking wider values and then narrower values.
        match matching_set
            .iter()
            .filter(|&&index| candidates[index].stretch > query.stretch)
            .min_by_key(|&&index| {
                candidates[index].stretch.to_number() - query.stretch.to_number()
            }) {
            Some(&matching_index) => candidates[matching_index].stretch,
            None => {
                let matching_index = *matching_set
                    .iter()
                    .min_by_key(|&&index| {
                        query.stretch.to_number() - candidates[index].stretch.to_number()
                    })
                    .unwrap();
                candidates[matching_index].stretch
            }
        }
    };
    matching_set.retain(|&index| candidates[index].stretch == matching_stretch);

    // Step 4b (`font-style`).
    let style_preference = match query.style {
        Style::Italic => [Style::Italic, Style::Oblique, Style::Normal],
        Style::Oblique => [Style::Oblique, Style::Italic, Style::Normal],
        Style::Normal => [Style::Normal, Style::Oblique, Style::Italic],
    };
    let matching_style = *style_preference
        .iter()
        .filter(|&query_style| {
            matching_set
                .iter()
                .any(|&index| candidates[index].style == *query_style)
        })
        .next()
        .unwrap();
    matching_set.retain(|&index| candidates[index].style == matching_style);

    // Step 4c (`font-weight`).
    //
    // The spec doesn't say what to do if the weight is between 400 and 500 exclusive, so we
    // just use 450 as the cutoff.
    let matching_weight = if weight >= 400
        && weight < 450
        && matching_set
        .iter()
        .any(|&index| candidates[index].weight.to_number() == 500)
    {
        // Check 500 first.
        Weight::Medium
    } else if weight >= 450 && weight <= 500 && matching_set
        .iter()
        .any(|&index| candidates[index].weight.to_number() == 400)
    {
        // Check 400 first.
        Weight::Normal
    } else if weight <= 500 {
        // Closest weight, first checking thinner values and then fatter ones.
        match matching_set
            .iter()
            .filter(|&&index| candidates[index].weight.to_number() <= weight)
            .min_by_key(|&&index| weight - candidates[index].weight.to_number())
            {
                Some(&matching_index) => candidates[matching_index].weight,
                None => {
                    let matching_index = *matching_set
                        .iter()
                        .min_by_key(|&&index| {
                            candidates[index].weight.to_number() - weight
                        })
                        .unwrap();
                    candidates[matching_index].weight
                }
            }
    } else {
        // Closest weight, first checking fatter values and then thinner ones.
        match matching_set
            .iter()
            .filter(|&&index| candidates[index].weight.to_number() >= weight)
            .min_by_key(|&&index| candidates[index].weight.to_number() - weight)
            {
                Some(&matching_index) => candidates[matching_index].weight,
                None => {
                    let matching_index = *matching_set
                        .iter()
                        .min_by_key(|&&index| {
                            weight - candidates[index].weight.to_number()
                        })
                        .unwrap();
                    candidates[matching_index].weight
                }
            }
    };
    matching_set.retain(|&index| candidates[index].weight == matching_weight);

    // Ignore step 4d.

    // Return the result.
    matching_set.into_iter().next()
}

struct PathBuilder {
    path: tree::PathData,
}

impl ttf_parser::OutlineBuilder for PathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path.push_move_to(x as f64, y as f64);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.path.push_line_to(x as f64, y as f64);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.path.push_quad_to(
            x1 as f64, y1 as f64,
            x as f64, y as f64,
        );
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.path.push_curve_to(
            x1 as f64, y1 as f64,
            x2 as f64, y2 as f64,
            x as f64, y as f64
        );
    }

    fn close(&mut self) {
        self.path.push_close_path();
    }
}


#[cfg(target_os = "linux")]
fn load_all_fonts(fonts: &mut Vec<FontItem>) {
    load_fonts_from("/usr/share/fonts/", fonts);
    load_fonts_from("/usr/local/share/fonts/", fonts);

    if let Ok(ref home) = std::env::var("HOME") {
        let path = Path::new(home).join("/.local/share/fonts");
        load_fonts_from(path.to_str().unwrap(), fonts);
    }
}

#[cfg(target_os = "windows")]
fn load_all_fonts(fonts: &mut Vec<FontItem>) {
    load_fonts_from("C:\\Windows\\Fonts\\", fonts);

    duplicate_family("Times New Roman", "serif", fonts);
    duplicate_family("Arial", "sans-serif", fonts);
    duplicate_family("Courier New", "monospace", fonts);
    duplicate_family("Comic Sans MS", "cursive", fonts);
    duplicate_family("Impact", "fantasy", fonts);
}

#[cfg(target_os = "macos")]
fn load_all_fonts(fonts: &mut Vec<FontItem>) {
    load_fonts_from("/Library/Fonts", fonts);
    load_fonts_from("/System/Library/Fonts", fonts);

    duplicate_family("Times New Roman", "serif", fonts);
    duplicate_family("Arial", "sans-serif", fonts);
    duplicate_family("Courier New", "monospace", fonts);
    duplicate_family("Comic Sans MS", "cursive", fonts);
    duplicate_family("Papyrus", "fantasy", fonts);
}

fn load_fonts_from(dir: &str, fonts: &mut Vec<FontItem>) {
    let fonts_dir = try_opt!(std::fs::read_dir(dir).ok());
    for entry in fonts_dir {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_file() {
                match utils::file_extension(&path) {
                    Some("ttf") | Some("ttc") | Some("TTF") | Some("TTC") |
                    Some("otf") | Some("otc") | Some("OTF") | Some("OTC") => {
                        let _ = load_font(&path, fonts);
                    }
                    _ => {}
                }
            } else if path.is_dir() {
                load_fonts_from(path.to_str().unwrap(), fonts);
            }
        }
    }
}

fn load_font(
    path: &Path,
    fonts: &mut Vec<FontItem>,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = fs::File::open(path)?;
    let mmap = unsafe { memmap2::MmapOptions::new().map(&file)? };

    let n = ttf_parser::fonts_in_collection(&mmap).unwrap_or(1);
    for index in 0..n {
        if let Some(item) = resolve_font(&mmap, path, index, fonts.len()) {
            fonts.push(item);
        }
    }

    Ok(())
}

fn resolve_font(
    data: &[u8],
    path: &Path,
    index: u32,
    id: usize,
) -> Option<FontItem> {
    let font = ttf_parser::Font::from_data(data, index).ok()?;

    let family = font.family_name()?;

    let style = if font.is_italic() {
        Style::Italic
    } else if font.is_oblique() {
        Style::Oblique
    } else {
        Style::Normal
    };

    let weight = font.weight();
    let stretch = font.width();

    let properties = Properties { style, weight, stretch };

    Some(FontItem {
        id: ID(id as u16),
        path: path.to_path_buf(),
        face_index: index,
        family,
        properties,
    })
}

fn duplicate_family(
    old_family: &str,
    new_family: &str,
    fonts: &mut Vec<FontItem>,
) {
    let mut i = 0;
    while i < fonts.len() {
        if fonts[i].family == old_family {
            let mut new_font = fonts[i].clone();
            new_font.family = new_family.to_string();
            fonts.push(new_font);
        }

        i += 1;
    }
}
