// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::path;

use usvg::NodeExt;

macro_rules! timed {
    ($args:expr, $name:expr, $task:expr) => {
        if $args.perf {
            let now = std::time::Instant::now();
            let res = $task;
            println!("{}: {:.2}ms", $name, now.elapsed().as_micros() as f64 / 1000.0);
            res
        } else {
            $task
        }
    };
}

fn main() {
    if let Err(e) = process() {
        eprintln!("Error: {}.", e);
        std::process::exit(1);
    }
}

fn process() -> Result<(), String> {
    let args = match parse_args() {
        Ok(args) => args,
        Err(e) => {
            println!("{}", HELP);
            return Err(e);
        }
    };

    // Do not print warning during the ID querying.
    //
    // Some crates still can print to stdout/stderr, but we can't do anything about it.
    if !(args.query_all || args.quiet) {
        if let Ok(()) = log::set_logger(&LOGGER) {
            log::set_max_level(log::LevelFilter::Warn);
        }
    }

    let svg_data = timed!(args, "Reading", {
        if args.in_svg == "-" {
            use std::io::Read;
            let mut buf = Vec::new();
            let stdin = std::io::stdin();
            let mut handle = stdin.lock();
            handle.read_to_end(&mut buf).map_err(|_| "failed to read stdin")?;
            buf
        } else {
            std::fs::read(&args.in_svg).map_err(|_| "failed to open the provided file")?
        }
    });

    let tree = timed!(args, "Parsing",
        usvg::Tree::from_data(&svg_data, &args.usvg.to_ref()).map_err(|e| e.to_string())
    )?;

    if args.query_all {
        return query_all(&tree);
    }

    // Dump before rendering in case of panic.
    if let Some(ref dump_path) = args.dump {
        dump_svg(&tree, dump_path)?;
    }

    let out_png = match args.out_png {
        Some(ref path) => path.clone(),
        None => return Ok(()),
    };

    // Render.
    render_svg(args, &tree, &out_png)
}

const HELP: &str = "\
resvg is an SVG rendering application.

USAGE:
  resvg [OPTIONS] <in-svg> <out-png>  # from file to file
  resvg [OPTIONS] - <out-png>         # from stdin to file

  resvg in.svg out.png
  resvg -z 4 in.svg out.png
  resvg --query-all in.svg

OPTIONS:
      --help                    Prints this help
  -V, --version                 Prints version

  -w, --width LENGTH            Sets the width in pixels
  -h, --height LENGTH           Sets the height in pixels
  -z, --zoom FACTOR             Zooms the image by a factor
      --dpi DPI                 Sets the resolution
                                [default: 96] [possible values: 10..4000]
  --background COLOR            Sets the background color
                                Examples: red, #fff, #fff000

  --languages LANG              Sets a comma-separated list of languages that
                                will be used during the 'systemLanguage'
                                attribute resolving
                                Examples: 'en-US', 'en-US, ru-RU', 'en, ru'
                                [default: en]
  --shape-rendering HINT        Selects the default shape rendering method
                                [default: geometricPrecision]
                                [possible values: optimizeSpeed, crispEdges,
                                geometricPrecision]
  --text-rendering HINT         Selects the default text rendering method
                                [default: optimizeLegibility]
                                [possible values: optimizeSpeed, optimizeLegibility,
                                geometricPrecision]
  --image-rendering HINT        Selects the default image rendering method
                                [default: optimizeQuality]
                                [possible values: optimizeQuality, optimizeSpeed]
  --resources-dir DIR           Sets a directory that will be used during
                                relative paths resolving.
                                Expected to be the same as the directory that
                                contains the SVG file, but can be set to any.
                                [default: input file directory]

  --font-family FAMILY          Sets the default font family that will be
                                used when no 'font-family' is present
                                [default: Times New Roman]
  --font-size SIZE              Sets the default font size that will be
                                used when no 'font-size' is present
                                [default: 12] [possible values: 1..192]
  --serif-family FAMILY         Sets the 'serif' font family
                                [default: Times New Roman]
  --sans-serif-family FAMILY    Sets the 'sans-serif' font family
                                [default: Arial]
  --cursive-family FAMILY       Sets the 'cursive' font family
                                [default: Comic Sans MS]
  --fantasy-family FAMILY       Sets the 'fantasy' font family
                                [default: Impact]
  --monospace-family FAMILY     Sets the 'monospace' font family
                                [default: Courier New]
  --use-font-file PATH          Load a specified font file into the fonts database.
                                Will be used during text to path conversion.
                                This option can be set multiple times
  --use-fonts-dir PATH          Loads all fonts from the specified directory
                                into the fonts database.
                                Will be used during text to path conversion.
                                This option can be set multiple times
  --skip-system-fonts           Disables system fonts loading.
                                You should add some fonts manually using
                                --use-font-file and/or --use-fonts-dir
                                Otherwise, text elements will not be processes
  --list-fonts                  Lists successfully loaded font faces.
                                Useful for debugging


  --query-all                   Queries all valid SVG ids with bounding boxes
  --export-id ID                Renders an object only with a specified ID
  --export-area-page            Use an image size instead of an object size during ID exporting

  --export-area-drawing         Use drawing's tight bounding box instead of image size.
                                Used during normal rendering and not during --export-id

  --perf                        Prints performance stats
  --quiet                       Disables warnings
  --dump-svg PATH               Saves the preprocessed SVG into the selected file

ARGS:
  <in-svg>                      Input file
  <out-png>                     Output file
";

#[derive(Debug)]
struct CliArgs {
    width: Option<u32>,
    height: Option<u32>,
    zoom: Option<f32>,
    dpi: u32,
    background: Option<usvg::Color>,

    languages: Vec<String>,
    shape_rendering: usvg::ShapeRendering,
    text_rendering: usvg::TextRendering,
    image_rendering: usvg::ImageRendering,
    resources_dir: Option<path::PathBuf>,

    font_family: Option<String>,
    font_size: u32,
    serif_family: Option<String>,
    sans_serif_family: Option<String>,
    cursive_family: Option<String>,
    fantasy_family: Option<String>,
    monospace_family: Option<String>,
    font_files: Vec<path::PathBuf>,
    font_dirs: Vec<path::PathBuf>,
    skip_system_fonts: bool,
    list_fonts: bool,

    query_all: bool,
    export_id: Option<String>,
    export_area_page: bool,

    export_area_drawing: bool,

    perf: bool,
    quiet: bool,
    dump_svg: Option<String>,

    input: String,
    output: Option<path::PathBuf>,
}

fn collect_args() -> Result<CliArgs, pico_args::Error> {
    let mut input = pico_args::Arguments::from_env();

    if input.contains("--help") {
        print!("{}", HELP);
        std::process::exit(0);
    }

    if input.contains(["-V", "--version"]) {
        println!("{}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }

    Ok(CliArgs {
        width:              input.opt_value_from_fn(["-w", "--width"], parse_length)?,
        height:             input.opt_value_from_fn(["-h", "--height"], parse_length)?,
        zoom:               input.opt_value_from_fn(["-z", "--zoom"], parse_zoom)?,
        dpi:                input.opt_value_from_fn("--dpi", parse_dpi)?.unwrap_or(96),
        background:         input.opt_value_from_str("--background")?,

        languages:          input.opt_value_from_fn("--languages", parse_languages)?
            .unwrap_or_else(|| vec!["en".to_string()]), // TODO: use system language
        shape_rendering:    input.opt_value_from_str("--shape-rendering")?.unwrap_or_default(),
        text_rendering:     input.opt_value_from_str("--text-rendering")?.unwrap_or_default(),
        image_rendering:    input.opt_value_from_str("--image-rendering")?.unwrap_or_default(),
        resources_dir:      input.opt_value_from_str("--resources-dir").unwrap_or_default(),

        font_family:        input.opt_value_from_str("--font-family")?,
        font_size:          input.opt_value_from_fn("--font-size", parse_font_size)?.unwrap_or(12),
        serif_family:       input.opt_value_from_str("--serif-family")?,
        sans_serif_family:  input.opt_value_from_str("--sans-serif-family")?,
        cursive_family:     input.opt_value_from_str("--cursive-family")?,
        fantasy_family:     input.opt_value_from_str("--fantasy-family")?,
        monospace_family:   input.opt_value_from_str("--monospace-family")?,
        font_files:         input.values_from_str("--use-font-file")?,
        font_dirs:          input.values_from_str("--use-fonts-dir")?,
        skip_system_fonts:  input.contains("--skip-system-fonts"),
        list_fonts:         input.contains("--list-fonts"),

        query_all:          input.contains("--query-all"),
        export_id:          input.opt_value_from_str("--export-id")?,
        export_area_page:   input.contains("--export-area-page"),

        export_area_drawing:input.contains("--export-area-drawing"),

        perf:               input.contains("--perf"),
        quiet:              input.contains("--quiet"),
        dump_svg:           input.opt_value_from_str("--dump-svg")?,

        input:              input.free_from_str()?,
        output:             input.opt_free_from_str()?,
    })
}

fn parse_dpi(s: &str) -> Result<u32, String> {
    let n: u32 = s.parse().map_err(|_| "invalid number")?;

    if (10..=4000).contains(&n) {
        Ok(n)
    } else {
        Err("DPI out of bounds".to_string())
    }
}

fn parse_length(s: &str) -> Result<u32, String> {
    let n: u32 = s.parse().map_err(|_| "invalid length")?;

    if n > 0 {
        Ok(n)
    } else {
        Err("LENGTH cannot be zero".to_string())
    }
}

fn parse_zoom(s: &str) -> Result<f32, String> {
    let n: f32 = s.parse().map_err(|_| "invalid zoom factor")?;

    if n > 0.0 {
        Ok(n)
    } else {
        Err("ZOOM should be positive".to_string())
    }
}

fn parse_font_size(s: &str) -> Result<u32, String> {
    let n: u32 = s.parse().map_err(|_| "invalid number")?;

    if n > 0 && n <= 192 {
        Ok(n)
    } else {
        Err("font size out of bounds".to_string())
    }
}

fn parse_languages(s: &str) -> Result<Vec<String>, String> {
    let mut langs = Vec::new();
    for lang in s.split(',') {
        langs.push(lang.trim().to_string());
    }

    if langs.is_empty() {
        return Err("languages list cannot be empty".to_string());
    }

    Ok(langs)
}

struct Args {
    in_svg: String,
    out_png: Option<path::PathBuf>,
    query_all: bool,
    export_id: Option<String>,
    export_area_page: bool,
    export_area_drawing: bool,
    dump: Option<path::PathBuf>,
    perf: bool,
    quiet: bool,
    usvg: usvg::Options,
    fit_to: usvg::FitTo,
    background: Option<usvg::Color>,
}

fn parse_args() -> Result<Args, String> {
    let mut args = collect_args().map_err(|e| e.to_string())?;

    let fontdb = timed!(args, "FontDB init", load_fonts(&mut args));
    if args.list_fonts {
        for face in fontdb.faces() {
            if let usvg::fontdb::Source::File(ref path) = &face.source {
                println!(
                    "{}: '{}', {}, {:?}, {:?}, {:?}",
                    path.display(), face.family, face.index,
                    face.style, face.weight.0, face.stretch
                );
            }
        }
    }

    if !args.query_all && args.output.is_none() {
        return Err("<out-png> must be set".to_string());
    }

    if args.input == "-" && args.resources_dir.is_none() {
        println!("Warning: Make sure to set --resources-dir when reading SVG from stdin.");
    }

    if args.export_area_page && args.export_id.is_none() {
        println!("Warning: --export-area-page has no effect without --export-id.");
    }

    if !args.export_area_drawing && args.export_id.is_some() {
        println!("Warning: --export-area-drawing has no effect when --export-id is set.");
    }

    let in_svg = args.input.clone();
    let out_png = args.output.clone();

    let dump = args.dump_svg.as_ref().map(|v| v.into());
    let export_id = args.export_id.as_ref().map(|v| v.to_string());

    // We don't have to keep named groups when we don't need them
    // because it will slow down rendering.
    let keep_named_groups = args.query_all || export_id.is_some();

    let mut fit_to = usvg::FitTo::Original;
    let mut default_size = usvg::Size::new(100.0, 100.0).unwrap();
    if let (Some(w), Some(h)) = (args.width, args.height) {
        default_size = usvg::Size::new(w as f64, h as f64).unwrap();
        fit_to = usvg::FitTo::Size(w, h);
    } else if let Some(w) = args.width {
        default_size = usvg::Size::new(w as f64, 100.0).unwrap();
        fit_to = usvg::FitTo::Width(w);
    } else if let Some(h) = args.height {
        default_size = usvg::Size::new(100.0, h as f64).unwrap();
        fit_to = usvg::FitTo::Height(h);
    } else if let Some(z) = args.zoom {
        fit_to = usvg::FitTo::Zoom(z);
    }

    let resources_dir = match args.resources_dir {
        Some(v) => Some(v),
        None => {
            // Get input file absolute directory.
            std::fs::canonicalize(&in_svg).ok().and_then(|p| p.parent().map(|p| p.to_path_buf()))
        }
    };

    let usvg = usvg::Options {
        resources_dir,
        dpi: args.dpi as f64,
        font_family: args.font_family.take().unwrap_or_else(|| "Times New Roman".to_string()),
        font_size: args.font_size as f64,
        languages: args.languages,
        shape_rendering: args.shape_rendering,
        text_rendering: args.text_rendering,
        image_rendering: args.image_rendering,
        keep_named_groups,
        default_size,
        fontdb,
    };

    Ok(Args {
        in_svg,
        out_png,
        query_all: args.query_all,
        export_id,
        export_area_page: args.export_area_page,
        export_area_drawing: args.export_area_drawing,
        dump,
        perf: args.perf,
        quiet: args.quiet,
        usvg,
        fit_to,
        background: args.background,
    })
}

fn load_fonts(args: &mut CliArgs) -> usvg::fontdb::Database {
    let mut fontdb = usvg::fontdb::Database::new();
    if !args.skip_system_fonts {
        fontdb.load_system_fonts();
    }

    for path in &args.font_files {
        if let Err(e) = fontdb.load_font_file(path) {
            log::warn!("Failed to load '{}' cause {}.", path.display(), e);
        }
    }

    for path in &args.font_dirs {
        fontdb.load_fonts_dir(path);
    }

    let take_or = |family: Option<String>, fallback: &str|
        family.unwrap_or_else(|| fallback.to_string());

    fontdb.set_serif_family(take_or(args.serif_family.take(), "Times New Roman"));
    fontdb.set_sans_serif_family(take_or(args.sans_serif_family.take(), "Arial"));
    fontdb.set_cursive_family(take_or(args.cursive_family.take(), "Comic Sans MS"));
    fontdb.set_fantasy_family(take_or(args.fantasy_family.take(), "Impact"));
    fontdb.set_monospace_family(take_or(args.monospace_family.take(), "Courier New"));

    fontdb
}

fn query_all(tree: &usvg::Tree) -> Result<(), String> {
    let mut count = 0;
    for node in tree.root().descendants() {
        if tree.is_in_defs(&node) {
            continue;
        }

        if node.id().is_empty() {
            continue;
        }

        count += 1;

        fn round_len(v: f64) -> f64 {
            (v * 1000.0).round() / 1000.0
        }

        if let Some(bbox) = node.calculate_bbox() {
            println!(
                "{},{},{},{},{}", node.id(),
                round_len(bbox.x()), round_len(bbox.y()),
                round_len(bbox.width()), round_len(bbox.height())
            );
        }
    }

    if count == 0 {
        return Err("the file has no valid ID's".to_string());
    }

    Ok(())
}

#[cfg(feature = "dump-svg")]
fn dump_svg(tree: &usvg::Tree, path: &path::Path) -> Result<(), String> {
    use std::io::Write;

    let mut f = std::fs::File::create(path)
        .map_err(|_| format!("failed to create a file {:?}", path))?;

    f.write_all(tree.to_string(&usvg::XmlOptions::default()).as_bytes())
        .map_err(|_| format!("failed to write a file {:?}", path))?;

    Ok(())
}

#[cfg(not(feature = "dump-svg"))]
fn dump_svg(_: &usvg::Tree, _: &path::Path) -> Result<(), String> {
    log::warn!("The dump-svg feature is not enabled.");
    Ok(())
}

fn render_svg(args: Args, tree: &usvg::Tree, out_png: &path::Path) -> Result<(), String> {
    let now = std::time::Instant::now();

    let img = if let Some(ref id) = args.export_id {
        let node = match tree.root().descendants().find(|n| &*n.id() == id) {
            Some(node) => node,
            None => return Err(format!("SVG doesn't have '{}' ID", id)),
        };

        let bbox = node.calculate_bbox().and_then(|r| r.to_rect())
            .ok_or_else(|| "node has zero size".to_string())?;

        let size = args.fit_to.fit_to(bbox.to_screen_size())
            .ok_or_else(|| "target size is zero".to_string())?;

        // Unwrap is safe, because `size` is already valid.
        let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();

        if !args.export_area_page {
            if let Some(background) = args.background {
                pixmap.fill(tiny_skia::Color::from_rgba8(
                    background.red, background.green, background.blue, 255));
            }
        }

        resvg::render_node(tree, &node, args.fit_to, pixmap.as_mut());

        if args.export_area_page {
            // TODO: add offset support to render_node() so we would not need an additional pixmap

            let size = args.fit_to.fit_to(tree.svg_node().size.to_screen_size())
                .ok_or_else(|| "target size is zero".to_string())?;

            // Unwrap is safe, because `size` is already valid.
            let mut page_pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();

            if let Some(background) = args.background {
                page_pixmap.fill(tiny_skia::Color::from_rgba8(
                    background.red, background.green, background.blue, 255));
            }

            page_pixmap.draw_pixmap(
                bbox.x() as i32,
                bbox.y() as i32,
                pixmap.as_ref(),
                &tiny_skia::PixmapPaint::default(),
                tiny_skia::Transform::default(),
                None
            );
            page_pixmap
        } else {
            pixmap
        }
    } else {
        let size = args.fit_to.fit_to(tree.svg_node().size.to_screen_size())
            .ok_or_else(|| "target size is zero".to_string())?;

        // Unwrap is safe, because `size` is already valid.
        let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();

        if !args.export_area_drawing {
            if let Some(background) = args.background {
                pixmap.fill(tiny_skia::Color::from_rgba8(
                    background.red, background.green, background.blue, 255));
            }
        }

        resvg::render(tree, args.fit_to, pixmap.as_mut());

        if args.export_area_drawing {
            let (_, _, pixmap) = resvg::trim_transparency(pixmap)
                .ok_or_else(|| "target size is zero".to_string())?;

            if let Some(background) = args.background {
                let mut bg = pixmap.clone();
                bg.fill(tiny_skia::Color::from_rgba8(
                    background.red, background.green, background.blue, 255));
                bg.draw_pixmap(
                    0,
                    0,
                    pixmap.as_ref(),
                    &tiny_skia::PixmapPaint::default(),
                    tiny_skia::Transform::default(),
                    None
                );
                bg
            } else {
                pixmap
            }
        } else {
            pixmap
        }
    };

    if args.perf {
        println!("Rendering: {:.2}ms", now.elapsed().as_micros() as f64 / 1000.0);
    }

    timed!(args, "Saving", img.save_png(out_png).map_err(|e| e.to_string()))
}


/// A simple stderr logger.
static LOGGER: SimpleLogger = SimpleLogger;
struct SimpleLogger;
impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::LevelFilter::Warn
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let target = if !record.target().is_empty() {
                record.target()
            } else {
                record.module_path().unwrap_or_default()
            };

            let line = record.line().unwrap_or(0);

            match record.level() {
                log::Level::Error => eprintln!("Error (in {}:{}): {}", target, line, record.args()),
                log::Level::Warn  => eprintln!("Warning (in {}:{}): {}", target, line, record.args()),
                log::Level::Info  => eprintln!("Info (in {}:{}): {}", target, line, record.args()),
                log::Level::Debug => eprintln!("Debug (in {}:{}): {}", target, line, record.args()),
                log::Level::Trace => eprintln!("Trace (in {}:{}): {}", target, line, record.args()),
            }
        }
    }

    fn flush(&self) {}
}
