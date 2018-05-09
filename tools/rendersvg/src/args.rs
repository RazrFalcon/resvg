// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// We don't use `clap` to reduce executable size.

use std::process;
use std::path;
use std::str::FromStr;

use getopts;

use resvg::{
    usvg,
    FitTo,
    Options,
};

pub fn print_help() {
    print!("\
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

    let mut opts = getopts::Options::new();
    opts.optflag("", "help", "");
    opts.optflag("V", "version", "");

    opts.optflag("", "perf", "");
    opts.optflag("", "pretend", "");
    opts.optflag("", "quiet", "");
    opts.optopt("", "dump-svg", "", "");

    opts.optflag("", "query-all", "");
    opts.optopt("", "export-id", "", "");

    opts.optopt("", "backend", "", "");
    opts.optopt("", "background", "", "");
    opts.optopt("", "dpi", "", "");
    opts.optopt("w", "width", "", "");
    opts.optopt("h", "height", "", "");
    opts.optopt("z", "zoom", "", "");

    let args = match opts.parse(&args[1..]) {
        Ok(v) => v,
        Err(e) => return Err(e.to_string().into()),
    };

    if args.opt_present("help") {
        print_help();
        process::exit(0);
    }

    if args.opt_present("version") {
        println!("{}", env!("CARGO_PKG_VERSION"));
        process::exit(0);
    }

    let positional_count = if args.opt_present("query-all") { 1 } else { 2 };

    if args.free.len() != positional_count {
        return Err(format!("<in-svg> and <out-png> must be set"));
    }

    let in_svg: path::PathBuf = args.free[0].to_string().into();

    let out_png = if !args.opt_present("query-all") {
        Some(args.free[1].to_string().into())
    } else {
        None
    };

    let backend_name = args.opt_str("backend").unwrap_or(default_backend().to_string());
    let dump = args.opt_str("dump-svg").map(|v| v.into());
    let export_id = args.opt_str("export-id").map(|v| v.to_string());

    let app_args = Args {
        in_svg: in_svg.clone(),
        out_png,
        backend_name,
        query_all: args.opt_present("query-all"),
        export_id,
        dump,
        pretend: args.opt_present("pretend"),
        perf: args.opt_present("perf"),
        quiet: args.opt_present("quiet"),
    };

    // We don't have to keep named groups when we don't need them
    // because it will slow down rendering.
    let keep_named_groups = app_args.query_all || app_args.export_id.is_some();

    let mut fit_to = FitTo::Original;
    if let Some(w) = get_type(&args, "width", "LENGTH")? {
        if w == 0 {
            return Err(format!("Invalid LENGTH"));
        }

        fit_to = FitTo::Width(w);
    } else if let Some(h) = get_type(&args, "height", "LENGTH")? {
        if h == 0 {
            return Err(format!("Invalid LENGTH"));
        }

        fit_to = FitTo::Height(h);
    } else if let Some(z) = get_type(&args, "zoom", "FACTOR")? {
        if !(z > 0.0) {
            return Err(format!("Invalid FACTOR"));
        }

        fit_to = FitTo::Zoom(z);
    }

    let background = get_type(&args, "background", "COLOR")?;

    let dpi = get_type(&args, "dpi", "DPI")?.unwrap_or(96);
    if dpi < 10 || dpi > 4000 {
        return Err(format!("DPI out of bounds"));
    }

    let opt = Options {
        usvg: usvg::Options {
            path: Some(in_svg.into()),
            dpi: dpi as f64,
            keep_named_groups,
        },
        fit_to,
        background,
    };

    Ok((app_args, opt))
}

fn get_type<T: FromStr>(args: &getopts::Matches, name: &str, type_name: &str) -> Result<Option<T>, String> {
    match args.opt_str(name) {
        Some(v) => {
            let t = v.parse().map_err(|_| format!("Invalid {}: '{}'", type_name, v))?;
            Ok(Some(t))
        }
        None => Ok(None),
    }
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
