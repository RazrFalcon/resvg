// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// We don't use `clap` to reduce executable size.

use std::process;
use std::path;

use gumdrop::Options as CliOptions;

use resvg::{
    usvg,
    FitTo,
    Options,
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

        --perf                  Prints performance stats
        --pretend               Does all the steps except rendering
        --quiet                 Disables warnings
        --dump-svg=<PATH>       Saves the preprocessed SVG to the selected file

        --query-all             Queries all valid SVG ids with bounding boxes
        --export-id=<ID>        Renders an object only with a specified ID

        --backend=<BACKEND>     Sets the rendering backend.
                                Has no effect if built with only one backend
                                [default: {}] [possible values: {}]

        --background=<COLOR>    Sets the background color.
                                Examples: red, #fff, #fff000
        --dpi=<DPI>             Sets the resolution
                                [default: 96] [possible values: 10..4000]
    -w, --width=<LENGTH>        Sets the width in pixels
    -h, --height=<LENGTH>       Sets the height in pixels
    -z, --zoom=<FACTOR>         Zooms the image by a factor

ARGS:
    <in-svg>                    Input file
    <out-png>                   Output file
", default_backend(),
   backends().join(", "));
}

#[derive(Debug, CliOptions)]
struct CliArgs {
    #[options(no_short)]
    help: bool,

    #[options(short = "V")]
    version: bool,

    #[options(no_short)]
    perf: bool,

    #[options(no_short)]
    pretend: bool,

    #[options(no_short)]
    quiet: bool,

    #[options(no_short, meta = "PATH")]
    dump_svg: Option<String>,

    #[options(no_short)]
    query_all: bool,

    #[options(no_short, meta = "ID")]
    export_id: Option<String>,

    #[options(no_short, meta = "BACKEND")]
    backend: Option<String>,

    #[options(no_short, meta = "COLOR", parse(try_from_str = "parse_color"))]
    background: Option<usvg::Color>,

    #[options(no_short, meta = "DPI", default = "96", parse(try_from_str = "parse_dpi"))]
    dpi: u32,

    #[options(short = "w", meta = "LENGTH", parse(try_from_str = "parse_length"))]
    width: Option<u32>,

    #[options(short = "h", meta = "LENGTH", parse(try_from_str = "parse_length"))]
    height: Option<u32>,

    #[options(short = "z", meta = "ZOOM", parse(try_from_str = "parse_zoom"))]
    zoom: Option<f32>,

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

    if !(n > 0.0) {
        Ok(n)
    } else {
        Err("ZOOM should be positive")
    }
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

pub fn parse() -> Result<(Args, Options), String> {
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

    let backend_name = args.backend.unwrap_or(default_backend().to_string());
    let dump = args.dump_svg.map(|v| v.into());
    let export_id = args.export_id.map(|v| v.to_string());

    let app_args = Args {
        in_svg: in_svg.clone(),
        out_png,
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

    let opt = Options {
        usvg: usvg::Options {
            path: Some(in_svg.into()),
            dpi: args.dpi as f64,
            font_family: "Times New Roman".to_string(),
            font_size: 12.0,
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
