// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::path;
use std::process;

use pico_args::Arguments;

use resvg::prelude::*;

pub fn print_help() {
    print!(
        "\
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
        --dump-svg PATH         Saves the preprocessed SVG to the selected file

ARGS:
    <in-svg>                    Input file
    <out-png>                   Output file
",
        default_backend(),
        backends().join(", ")
    );
}

#[derive(Debug)]
struct CliArgs {
    help: bool,
    version: bool,
    backend: String,
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
    pretend: bool,
    quiet: bool,
    dump_svg: Option<String>,
    free: Vec<String>,
}

fn collect_args() -> Result<CliArgs, pico_args::Error> {
    let mut input = Arguments::from_env();
    Ok(CliArgs {
        help: input.contains("--help"),
        version: input.contains(["-V", "--version"]),
        backend: input
            .value_from_str("--backend")?
            .unwrap_or(default_backend()),
        width: input.value_from_fn(["-w", "--width"], parse_length)?,
        height: input.value_from_fn(["-h", "--height"], parse_length)?,
        zoom: input.value_from_fn(["-z", "--zoom"], parse_zoom)?,
        dpi: input.value_from_fn("--dpi", parse_dpi)?.unwrap_or(96),
        background: input.value_from_str("--background")?,
        font_family: input
            .value_from_str("--font-family")?
            .unwrap_or_else(|| "Times New Roman".to_string()),
        font_size: input
            .value_from_fn("--font-size", parse_font_size)?
            .unwrap_or(12),
        languages: input
            .value_from_fn("--languages", parse_languages)?
            .unwrap_or(vec!["en".to_string()]), // TODO: use system language
        shape_rendering: input
            .value_from_str("--shape-rendering")?
            .unwrap_or_default(),
        text_rendering: input
            .value_from_str("--text-rendering")?
            .unwrap_or_default(),
        image_rendering: input
            .value_from_str("--image-rendering")?
            .unwrap_or_default(),
        query_all: input.contains("--query-all"),
        export_id: input.value_from_str("--export-id")?,
        perf: input.contains("--perf"),
        pretend: input.contains("--pretend"),
        quiet: input.contains("--quiet"),
        dump_svg: input.value_from_str("--dump-svg")?,
        free: input.free()?,
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

pub struct Args {
    pub in_svg: path::PathBuf,
    pub out_png: Option<path::PathBuf>,
    pub backend_name: String,
    pub query_all: bool,
    pub export_id: Option<String>,
    pub dump: Option<path::PathBuf>,
    pub pretend: bool,
    pub perf: bool,
    pub quiet: bool,
}

pub fn parse() -> Result<(Args, resvg::Options), String> {
    let args = collect_args().map_err(|e| e.to_string())?;

    if args.help {
        print_help();
        process::exit(0);
    }

    if args.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        process::exit(0);
    }

    let positional_count = if args.query_all { 1 } else { 2 };

    if args.free.len() != positional_count {
        return Err(format!("<in-svg> and <out-png> must be set"));
    }

    let in_svg: path::PathBuf = args.free[0].to_string().into();

    let out_png = if !args.query_all {
        Some(args.free[1].to_string().into())
    } else {
        None
    };

    let dump = args.dump_svg.map(|v| v.into());
    let export_id = args.export_id.map(|v| v.to_string());

    let app_args = Args {
        in_svg: in_svg.clone(),
        out_png,
        backend_name: args.backend,
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

    let opt = resvg::Options {
        usvg: usvg::Options {
            path: Some(in_svg.into()),
            dpi: args.dpi as f64,
            font_family: args.font_family.clone(),
            font_size: args.font_size as f64,
            languages: args.languages,
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
fn default_backend() -> String {
    #[cfg(feature = "cairo-backend")]
    {
        return "cairo".to_string();
    }

    #[cfg(feature = "qt-backend")]
    {
        return "qt".to_string();
    }

    #[cfg(feature = "skia-backend")]
    {
        return "skia".to_string();
    }

    #[cfg(feature = "raqote-backend")]
    {
        return "raqote".to_string();
    }

    unreachable!();
}

fn backends() -> Vec<&'static str> {
    let mut list = Vec::new();

    #[cfg(feature = "cairo-backend")]
    {
        list.push("cairo");
    }

    #[cfg(feature = "qt-backend")]
    {
        list.push("qt");
    }

    #[cfg(feature = "skia-backend")]
    {
        list.push("skia");
    }

    #[cfg(feature = "raqote-backend")]
    {
        list.push("raqote");
    }

    list
}
