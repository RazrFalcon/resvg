// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::io::Write;
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
            return Err(format!("{}", e));
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

    // Load file.
    let tree = timed!(args, "Preprocessing",
        usvg::Tree::from_file(&args.in_svg, &args.usvg).map_err(|e| e.to_string())
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

fn render_svg(args: Args, tree: &usvg::Tree, out_png: &path::Path) -> Result<(), String> {
    let opt = resvg_raqote::Options {
        usvg: args.usvg,
        fit_to: args.fit_to,
        background: args.background,
    };

    let img = if let Some(ref id) = args.export_id {
        if let Some(node) = tree.root().descendants().find(|n| &*n.id() == id) {
            timed!(args, "Rendering", resvg_raqote::render_node_to_image(&node, &opt))
        } else {
            return Err(format!("SVG doesn't have '{}' ID", id));
        }
    } else {
        timed!(args, "Rendering", resvg_raqote::render_to_image(&tree, &opt))
    };

    match img {
        Some(img) => {
            timed!(args, "Saving", img.write_png(out_png).map_err(|e| e.to_string())?);
            Ok(())
        }
        None => {
            Err("failed to allocate an image".to_string())
        }
    }
}

fn dump_svg(tree: &usvg::Tree, path: &path::Path) -> Result<(), String> {
    let mut f = std::fs::File::create(path)
        .map_err(|_| format!("failed to create a file {:?}", path))?;

    f.write_all(tree.to_string(usvg::XmlOptions::default()).as_bytes())
        .map_err(|_| format!("failed to write a file {:?}", path))?;

    Ok(())
}


const HELP: &str = "\
resvg-raqote is an SVG rendering application.

USAGE:
    resvg-raqote [OPTIONS] <in-svg> <out-png>

    resvg-raqote in.svg out.png
    resvg-raqote -z 4 in.svg out.png
    resvg-raqote --query-all in.svg

OPTIONS:
        --help                  Prints this help
    -V, --version               Prints version

    -w, --width LENGTH          Sets the width in pixels
    -h, --height LENGTH         Sets the height in pixels
    -z, --zoom FACTOR           Zooms the image by a factor
        --dpi DPI               Sets the resolution
                                [default: 96] [possible values: 10..4000]

        --background COLOR      Sets the background color.
                                Examples: red, #fff, #fff000
        --font-family FAMILY    Sets the default font family
                                [default: 'Times New Roman']
        --font-size SIZE        Sets the default font size
                                [default: 12] [possible values: 1..192]
        --languages LANG        Sets a comma-separated list of languages that
                                will be used during the 'systemLanguage'
                                attribute resolving.
                                Examples: 'en-US', 'en-US, ru-RU', 'en, ru'
                                [default: 'en']
        --shape-rendering HINT  Selects the default shape rendering method.
                                [default: geometricPrecision]
                                [possible values: optimizeSpeed, crispEdges,
                                geometricPrecision]
        --text-rendering HINT   Selects the default text rendering method.
                                [default: optimizeLegibility]
                                [possible values: optimizeSpeed,
                                optimizeLegibility, geometricPrecision]
        --image-rendering HINT  Selects the default image rendering method.
                                [default: optimizeQuality]
                                [possible values: optimizeQuality,
                                optimizeSpeed]

        --query-all             Queries all valid SVG ids with bounding boxes
        --export-id ID          Renders an object only with a specified ID

        --perf                  Prints performance stats
        --quiet                 Disables warnings
        --dump-svg PATH         Saves the preprocessed SVG into the selected file

ARGS:
    <in-svg>                    Input file
    <out-png>                   Output file
";

#[derive(Debug)]
struct CliArgs {
    help: bool,
    version: bool,
    width: Option<u32>,
    height: Option<u32>,
    zoom: Option<f32>,
    dpi: u32,
    background: Option<usvg::Color>,
    font_family: String,
    font_size: u32,
    languages: Vec<String>,
    shape_rendering: usvg::ShapeRendering,
    text_rendering: usvg::TextRendering,
    image_rendering: usvg::ImageRendering,
    query_all: bool,
    export_id: Option<String>,
    perf: bool,
    quiet: bool,
    dump_svg: Option<String>,
    free: Vec<String>,
}

fn collect_args() -> Result<CliArgs, pico_args::Error> {
    let mut input = pico_args::Arguments::from_env();
    Ok(CliArgs {
        help:               input.contains("--help"),
        version:            input.contains(["-V", "--version"]),
        width:              input.opt_value_from_fn(["-w", "--width"], parse_length)?,
        height:             input.opt_value_from_fn(["-h", "--height"], parse_length)?,
        zoom:               input.opt_value_from_fn(["-z", "--zoom"], parse_zoom)?,
        dpi:                input.opt_value_from_fn("--dpi", parse_dpi)?.unwrap_or(96),
        background:         input.opt_value_from_str("--background")?,
        font_family:        input.opt_value_from_str("--font-family")?
                                 .unwrap_or_else(|| "Times New Roman".to_string()),
        font_size:          input.opt_value_from_fn("--font-size", parse_font_size)?.unwrap_or(12),
        languages:          input.opt_value_from_fn("--languages", parse_languages)?
                                 .unwrap_or(vec!["en".to_string()]), // TODO: use system language
        shape_rendering:    input.opt_value_from_str("--shape-rendering")?.unwrap_or_default(),
        text_rendering:     input.opt_value_from_str("--text-rendering")?.unwrap_or_default(),
        image_rendering:    input.opt_value_from_str("--image-rendering")?.unwrap_or_default(),
        query_all:          input.contains("--query-all"),
        export_id:          input.opt_value_from_str("--export-id")?,
        perf:               input.contains("--perf"),
        quiet:              input.contains("--quiet"),
        dump_svg:           input.opt_value_from_str("--dump-svg")?,
        free:               input.free()?,
    })
}

fn parse_dpi(s: &str) -> Result<u32, String> {
    let n: u32 = s.parse().map_err(|_| "invalid number")?;

    if n >= 10 && n <= 4000 {
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
    in_svg: path::PathBuf,
    out_png: Option<path::PathBuf>,
    query_all: bool,
    export_id: Option<String>,
    dump: Option<path::PathBuf>,
    perf: bool,
    quiet: bool,
    usvg: usvg::Options,
    fit_to: usvg::FitTo,
    background: Option<usvg::Color>,
}

fn parse_args() -> Result<Args, String> {
    let args = collect_args().map_err(|e| e.to_string())?;

    if args.help {
        print!("{}", HELP);
        std::process::exit(0);
    }

    if args.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }

    let positional_count = if args.query_all { 1 } else { 2 };

    if args.free.len() != positional_count {
        return Err("<in-svg> and <out-png> must be set".to_string());
    }

    let in_svg: path::PathBuf = args.free[0].to_string().into();

    let out_png = if !args.query_all {
        Some(args.free[1].to_string().into())
    } else {
        None
    };

    let dump = args.dump_svg.map(|v| v.into());
    let export_id = args.export_id.map(|v| v.to_string());

    // We don't have to keep named groups when we don't need them
    // because it will slow down rendering.
    let keep_named_groups = args.query_all || export_id.is_some();

    let mut fit_to = usvg::FitTo::Original;
    if let Some(w) = args.width {
        fit_to = usvg::FitTo::Width(w);
    } else if let Some(h) = args.height {
        fit_to = usvg::FitTo::Height(h);
    } else if let Some(z) = args.zoom {
        fit_to = usvg::FitTo::Zoom(z);
    }

    let usvg = usvg::Options {
        path: Some(in_svg.clone()),
        dpi: args.dpi as f64,
        font_family: args.font_family.clone(),
        font_size: args.font_size as f64,
        languages: args.languages,
        shape_rendering: args.shape_rendering,
        text_rendering: args.text_rendering,
        image_rendering: args.image_rendering,
        keep_named_groups,
    };

    Ok(Args {
        in_svg: in_svg.clone(),
        out_png,
        query_all: args.query_all,
        export_id,
        dump,
        perf: args.perf,
        quiet: args.quiet,
        usvg,
        fit_to,
        background: args.background,
    })
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
            let target = if record.target().len() > 0 {
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
