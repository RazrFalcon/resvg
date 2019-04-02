// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// We don't use `clap` to reduce executable size.

use std::process;
use std::path;

use gumdrop::Options;

use resvg::{
    self,
    usvg,
    FitTo,
};

pub fn print_help() {
    print!("\
rendersvg is an SVG rendering application.

USAGE:
    rendersvg [OPTIONS] <in-svg> <out-png>

    rendersvg in.svg out.png
    rendersvg -z 4 in.svg out.png
    rendersvg --query-all in.svg

OPTIONS:
        --help                  Prints help information
    -V, --version               Prints version information

        --backend BACKEND       Sets the rendering backend.
                                Has no effect if built with only one backend
                                [default: {}] [possible values: {}]
    -f, --format FORMAT         Save format
                                [default: {} [possible values: {}]
    -o, --output                Output filename [optional: defaults to stdout]
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
        --pretend               Does all the steps except rendering
        --quiet                 Disables warnings
        --dump-svg <PATH>       Saves the preprocessed SVG to the selected file

ARGS:
    <in-svg>                    Input file
", default_backend(), backends().join(", "),
   "svg", formats().join(", ")
   );
}

#[derive(Debug, Options)]
struct CliArgs {
    #[options(no_short)]
    help: bool,

    #[options(short = "V")]
    version: bool,

    #[options(no_short, meta = "BACKEND")]
    backend: Option<String>,

    #[options(short ="f", meta = "FORMAT")]
    out_format: Option<String>,

    #[options(short ="o", meta = "FORMAT")]
    output: Option<String>,

    #[options(short = "w", meta = "LENGTH", parse(try_from_str = "parse_length"))]
    width: Option<u32>,

    #[options(short = "h", meta = "LENGTH", parse(try_from_str = "parse_length"))]
    height: Option<u32>,

    #[options(short = "z", meta = "ZOOM", parse(try_from_str = "parse_zoom"))]
    zoom: Option<f32>,

    #[options(no_short, meta = "DPI", default = "96", parse(try_from_str = "parse_dpi"))]
    dpi: u32,

    #[options(no_short, meta = "COLOR", parse(try_from_str = "parse_color"))]
    background: Option<usvg::Color>,

    #[options(no_short, meta = "FAMILY", default = "Times New Roman")]
    font_family: String,

    #[options(no_short, meta = "SIZE", default = "12", parse(try_from_str = "parse_font_size"))]
    font_size: u32,

    #[options(no_short, meta = "LANG", parse(try_from_str = "parse_languages"))]
    languages: Option<Vec<String>>,

    #[options(no_short, meta = "HINT", default = "geometricPrecision", parse(try_from_str))]
    shape_rendering: usvg::ShapeRendering,

    #[options(no_short, meta = "HINT", default = "optimizeLegibility", parse(try_from_str))]
    text_rendering: usvg::TextRendering,

    #[options(no_short, meta = "HINT", default = "optimizeQuality", parse(try_from_str))]
    image_rendering: usvg::ImageRendering,

    #[options(no_short)]
    query_all: bool,

    #[options(no_short, meta = "ID")]
    export_id: Option<String>,

    #[options(no_short)]
    perf: bool,

    #[options(no_short)]
    pretend: bool,

    #[options(no_short)]
    quiet: bool,

    #[options(no_short, meta = "PATH")]
    dump_svg: Option<String>,

    #[options(free)]
    free: Vec<String>,
}

fn parse_color(s: &str) -> Result<usvg::Color, &'static str> {
    s.parse().map_err(|_| "invalid zoom factor")
}

fn parse_dpi(s: &str) -> Result<u32, &'static str> {
    let n: u32 = s.parse().map_err(|_| "invalid number")?;

    if n >= 10 && n <= 4000 {
        Ok(n)
    } else {
        Err("DPI out of bounds")
    }
}

fn parse_length(s: &str) -> Result<u32, &'static str> {
    let n: u32 = s.parse().map_err(|_| "invalid length")?;

    if n > 0 {
        Ok(n)
    } else {
        Err("LENGTH cannot be zero")
    }
}

fn parse_zoom(s: &str) -> Result<f32, &'static str> {
    let n: f32 = s.parse().map_err(|_| "invalid zoom factor")?;

    if n > 0.0 {
        Ok(n)
    } else {
        Err("ZOOM should be positive")
    }
}

fn parse_font_size(s: &str) -> Result<u32, &'static str> {
    let n: u32 = s.parse().map_err(|_| "invalid number")?;

    if n > 0 && n <= 192 {
        Ok(n)
    } else {
        Err("font size out of bounds")
    }
}

fn parse_languages(s: &str) -> Result<Vec<String>, &'static str> {
    let mut langs = Vec::new();
    for lang in s.split(',') {
        langs.push(lang.trim().to_string());
    }

    if langs.is_empty() {
        return Err("languages list cannot be empty");
    }

    Ok(langs)
}

pub struct Args {
    pub in_svg: path::PathBuf,
    pub out_file: String,
    pub out_format: String,
    pub backend_name: String,
    pub query_all: bool,
    pub export_id: Option<String>,
    pub dump: Option<path::PathBuf>,
    pub pretend: bool,
    pub perf: bool,
    pub quiet: bool,
}

pub fn parse() -> Result<(Args, resvg::Options), String> {
    let args: Vec<String> = ::std::env::args().collect();
    let args = match CliArgs::parse_args_default(&args[1..]) {
        Ok(v) => v,
        Err(e) => return Err(format!("{}", e)),
    };

    if args.help {
        print_help();
        process::exit(0);
    }

    if args.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        process::exit(0);
    }

    let positional_count = 1;

    if args.free.len() != positional_count {
        return Err(format!("<in-svg>"));
    }

    let in_svg: path::PathBuf = args.free[0].to_string().into();

    let out_file = args.output.unwrap_or(String::from("-"));
    let backend_name = args.backend.unwrap_or(default_backend().to_string());
    let dump = args.dump_svg.map(|v| v.into());
    let export_id = args.export_id.map(|v| v.to_string());
    let out_format = args.out_format.unwrap_or(default_format().to_string());

    let app_args = Args {
        in_svg: in_svg.clone(),
        out_format,
        out_file,
        backend_name,
        query_all: args.query_all,
        export_id,
        dump,
        pretend: args.pretend,
        perf: args.perf,
        quiet: args.quiet,
    };

    // We don't have to keep named groups when we don't need them
    // because it will slow down rendering.
    let keep_named_groups = app_args.query_all || app_args.export_id.is_some();

    let mut fit_to = FitTo::Original;
    if let Some(w) = args.width {
        fit_to = FitTo::Width(w);
    } else if let Some(h) = args.height {
        fit_to = FitTo::Height(h);
    } else if let Some(z) = args.zoom {
        fit_to = FitTo::Zoom(z);
    }

    let languages = match args.languages.as_ref() {
        Some(v) => v.clone(),
        None => vec!["en".to_string()], // TODO: use system language
    };

    let opt = resvg::Options {
        usvg: usvg::Options {
            path: Some(in_svg.into()),
            dpi: args.dpi as f64,
            font_family: args.font_family.clone(),
            font_size: args.font_size as f64,
            languages,
            shape_rendering: args.shape_rendering,
            text_rendering: args.text_rendering,
            image_rendering: args.image_rendering,
            keep_named_groups,
        },
        fit_to,
        background: args.background,
    };

    Ok((app_args, opt))
}

#[allow(unreachable_code)]
fn default_backend() -> &'static str {
    #[cfg(feature = "cairo-backend")]
    { return "cairo" }

    #[cfg(feature = "qt-backend")]
    { return "qt" }

    unreachable!();
}

fn backends() -> Vec<&'static str> {
    let mut list = Vec::new();

    #[cfg(feature = "cairo-backend")]
    { list.push("cairo"); }

    #[cfg(feature = "qt-backend")]
    { list.push("qt"); }

    list
}

fn default_format() -> &'static str {
    return "svg";
}

fn formats() -> Vec<&'static str> {
    let mut list = Vec::new();

    { list.push("svg"); }
    { list.push("pdf"); }

    list
}