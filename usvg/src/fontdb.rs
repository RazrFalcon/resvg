use std::path::{Path, PathBuf};

pub use ttf_parser::os2::{Weight, Width as Stretch};


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
}

impl Database {
    pub fn new() -> Self {
        Database {
            fonts: Vec::new(),
        }
    }

    pub fn populate(&mut self) {
        if !self.fonts.is_empty() {
            return;
        }

        let mut id = 0;
        for font in load_all_fonts() {
            if let Some(item) = resolve_font(font, ID(id)) {
                self.fonts.push(item);
                id += 1;
            }
        }
    }

    pub fn font(&self, id: ID) -> &FontItem {
        &self.fonts[id.0 as usize]
    }

    pub fn fonts(&self) -> &[FontItem] {
        &self.fonts
    }

    pub fn select_best_match(&self, family_names: &[&str], properties: Properties) -> Option<ID> {
        for family_name in family_names {
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

    pub fn outline(&self, id: ID, glyph_id: u16) -> Option<svgdom::Path> {
        // We can't simplify this code because of lifetimes.
        let item = self.font(id);
        let file = std::fs::File::open(&item.path).ok()?;
        let mmap = unsafe { memmap::MmapOptions::new().map(&file).ok()? };
        let font = ttf_parser::Font::from_data(&mmap, item.face_index).ok()?;

        let font_glyph = font.glyph(glyph_id).ok()?;

        let mut builder = PathBuilder(svgdom::Path::new());
        font_glyph.outline(&mut builder);
        Some(builder.0)
    }

    pub fn has_char(&self, id: ID, c: char) -> bool {
        self._has_char(id, c).unwrap_or(false)
    }

    fn _has_char(&self, id: ID, c: char) -> Option<bool> {
        // We can't simplify this code because of lifetimes.
        let item = self.font(id);
        let file = std::fs::File::open(&item.path).ok()?;
        let mmap = unsafe { memmap::MmapOptions::new().map(&file).ok()? };
        let font = ttf_parser::Font::from_data(&mmap, item.face_index).ok()?;

        font.glyph_index(c).ok()?;

        Some(true)
    }

    pub fn load_font(&self, id: ID) -> Option<Font> {
        // We can't simplify this code because of lifetimes.
        let item = self.font(id);
        let file = std::fs::File::open(&item.path).ok()?;
        let mmap = unsafe { memmap::MmapOptions::new().map(&file).ok()? };
        let font = ttf_parser::Font::from_data(&mmap, item.face_index).ok()?;

        // Some fonts can have `units_per_em` set to zero, which will break out calculations.
        // `ttf_parser` will check this for us.
        let units_per_em = font.units_per_em()?;

        let ascent = font.ascender();
        let descent = font.descender();

        let x_height = match font.x_height() {
            Some(h) => h,
            None => {
                // If not set - fallback to height * 45%.
                // 45% is what Firefox uses.
                (f32::from(ascent - descent) * 0.45) as i16
            }
        };

        let underline = font.underline();

        Some(Font {
            id,
            units_per_em,
            ascent,
            descent,
            x_height,
            underline_position: underline.position,
            underline_thickness: underline.thickness,
        })
    }
}

fn resolve_font(path: FontPath, id: ID) -> Option<FontItem> {
    let file = std::fs::File::open(&path.path).ok()?;
    let mmap = unsafe { memmap::MmapOptions::new().map(&file).ok()? };
    let font = ttf_parser::Font::from_data(&mmap, path.index).ok()?;

    let family = match path.family {
        Some(f) => f,
        None => font.family_name()?,
    };

    let mut style = Style::Normal;
    if font.is_italic().unwrap_or(false) {
        style = Style::Italic;
    } else if font.is_oblique().unwrap_or(false) {
        style = Style::Oblique;
    }

    let weight = font.weight().unwrap_or(Weight::Normal);
    let stretch = font.width().unwrap_or(Stretch::Normal);
    let properties = Properties { style, weight, stretch };

    Some(FontItem {
        id,
        path: path.path,
        face_index: path.index,
        family,
        properties,
    })
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

/// From https://github.com/pcwalton/font-kit
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

struct PathBuilder(svgdom::Path);

impl ttf_parser::glyf::OutlineBuilder for PathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push(svgdom::PathSegment::MoveTo { abs: true, x: x as f64, y: y as f64 });
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push(svgdom::PathSegment::LineTo { abs: true, x: x as f64, y: y as f64 });
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.push(svgdom::PathSegment::Quadratic {
            abs: true, x1: x1 as f64, y1: y1 as f64, x: x as f64, y: y as f64
        });
    }

    fn close(&mut self) {
        self.0.push(svgdom::PathSegment::ClosePath { abs: true });
    }
}

#[derive(Debug)]
struct FontPath {
    path: PathBuf,
    index: u32,
    family: Option<String>,
}

#[cfg(target_os = "linux")]
fn load_all_fonts() -> Vec<FontPath> {
    fn load() -> Option<Vec<FontPath>> {
        let output = std::process::Command::new("fc-list")
            .arg("--format=%{index} %{file}\n")
            .output().ok();
        let output = try_opt_warn_or!(output, None, "Failed to run 'fc-list'.");
        let stdout = std::str::from_utf8(&output.stdout).ok()?;
        let mut paths = Vec::new();

        for line in stdout.split('\n') {
            if line.is_empty() {
                continue;
            }

            let index: u32 = match line[0..1].parse() {
                Ok(v) => v,
                Err(_) => continue,
            };

            let path = line[2..].into();
            paths.push(FontPath {
                path,
                index,
                family: None,
            });
        }

        Some(paths)
    }

    fn match_font(name: &str) -> Option<FontPath> {
        let output = std::process::Command::new("fc-match")
            .arg(name)
            .arg("--format=%{index} %{file}")
            .output().ok();
        let output = try_opt_warn_or!(output, None, "Failed to run 'fc-match'.");
        let stdout = std::str::from_utf8(&output.stdout).ok()?;

        let index: u32 = stdout[0..1].parse().ok()?;
        let path = stdout[2..].into();
        Some(FontPath {
            path,
            index,
            family: Some(name.to_string()),
        })
    }

    let mut fonts = load().unwrap_or_default();

    // TODO: very slow
    let generic_families = &["serif", "sans-serif", "monospace", "cursive", "fantasy"];
    for family in generic_families {
        if let Some(font) = match_font(family) {
            fonts.push(font);
        }
    }

    fonts
}

#[cfg(target_os = "windows")]
fn load_all_fonts() -> Vec<FontPath> {
    let mut paths = Vec::new();
    load_fonts_from("C:\\Windows\\Fonts\\", &mut paths);

    let _ = load_font(Path::new("C:\\Windows\\Fonts\\times.ttf"), Some("serif"), &mut paths);
    let _ = load_font(Path::new("C:\\Windows\\Fonts\\arial.ttf"), Some("sans-serif"), &mut paths);
    let _ = load_font(Path::new("C:\\Windows\\Fonts\\cour.ttf"), Some("monospace"), &mut paths);
    let _ = load_font(Path::new("C:\\Windows\\Fonts\\comic.ttf"), Some("cursive"), &mut paths);
    let _ = load_font(Path::new("C:\\Windows\\Fonts\\impact.ttf"), Some("fantasy"), &mut paths);

    paths
}

#[cfg(target_os = "macos")]
fn load_all_fonts() -> Vec<FontPath> {
    let mut paths = Vec::new();
    load_fonts_from("/Library/Fonts", &mut paths);
    load_fonts_from("/System/Library/Fonts", &mut paths);

    let _ = load_font(Path::new("/Library/Fonts/Times New Roman.ttf"), Some("serif"), &mut paths);
    let _ = load_font(Path::new("/Library/Fonts/Arial.ttf"), Some("sans-serif"), &mut paths);
    let _ = load_font(Path::new("/Library/Fonts/Courier New.ttf"), Some("monospace"), &mut paths);
    let _ = load_font(Path::new("/Library/Fonts/Comic Sans MS.ttf"), Some("cursive"), &mut paths);
    let _ = load_font(Path::new("/Library/Fonts/Papyrus.ttc"), Some("fantasy"), &mut paths);

    paths
}

#[allow(dead_code)]
fn load_fonts_from(dir: &str, paths: &mut Vec<FontPath>) {
    let fonts_dir = try_opt!(std::fs::read_dir(dir).ok());
    for entry in fonts_dir {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_file() {
                let _ = load_font(&path, None, paths);
            }
        }
    }
}

#[allow(dead_code)]
fn load_font(
    path: &Path,
    family: Option<&str>,
    paths: &mut Vec<FontPath>,
) -> Result<(), Box<std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let mmap = unsafe { memmap::MmapOptions::new().map(&file)? };

    if ttf_parser::Font::is_collection(&mmap) {
        for index in 0..ttf_parser::Font::fonts_number(&mmap).unwrap() {
            paths.push(FontPath {
                path: path.to_owned(),
                index,
                family: family.map(|s| s.to_string()),
            });
        }
    } else {
        paths.push(FontPath {
            path: path.to_owned(),
            index: 0,
            family: family.map(|s| s.to_string()),
        });
    }

    Ok(())
}
