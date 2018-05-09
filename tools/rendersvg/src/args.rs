// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! A minimal arguments parser.
//!
//! We don't use `clap` to reduce executable size.
//! And we do by 30%/600KiB.

use std::process;
use std::env;
use std::path;
use std::str::FromStr;

use resvg::{
    usvg,
    FitTo,
    Options,
};

pub fn print_help() {
    print!("\
USAGE:
    rendersvg [OPTIONS] <in-svg> <out-png>

OPTIONS:
        --help                  Prints help information
        --version               Prints version information

        --perf                  Prints performance stats
        --pretend               Does all the steps except rendering
        --quiet                 Disables warnings
        --dump-svg=<PATH>       Saves the preprocessed SVG to the selected file

        --query-all             Queries all valid SVG ids with bounding boxes
        --export-id=<ID>        Renders an object only with a specified ID

        --backend=<backend>     Sets the rendering backend.
                                Has no effect if built with only one backend
                                [default: {}] [possible values: {}]

        --background=<COLOR>    Sets the background color.
                                Examples: red, #fff, #fff000, rgb(255, 0, 0)
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

static FLAGS: &[&str] = &[
    "--perf",
    "--pretend",
    "--quiet",
    "--query-all",
];

static OPTIONS: &[&str] = &[
    "--dump-svg",
    "--export-id",
    "--backend",
    "--background",
    "--dpi",
    "-w",
    "--width",
    "-h",
    "--height",
    "-z",
    "--zoom",
];

#[derive(Debug)]
struct ArgsList<'a> {
    flags: Vec<&'a str>,
    options: Vec<(&'a str, &'a str)>, // TODO: to map
    positional: Vec<&'a str>,
}

impl<'a> ArgsList<'a> {
    fn create_flag_only(name: &'a str) -> Self {
        ArgsList {
            flags: vec![name],
            options: Vec::new(),
            positional: Vec::new(),
        }
    }

    fn parse(args: &'a [&str]) -> Result<Self, String> {
        if args == &["--help"] || args == &["-h"] {
            return Ok(Self::create_flag_only("--help"));
        }

        if args == &["--version"] {
            return Ok(Self::create_flag_only("--version"));
        }

        if args.len() < 2 {
            return Err(format!("<in-svg> and <out-png> must be set"));
        }

        let mut flags = Vec::new();
        let mut options = Vec::new();
        let mut positional = Vec::new();

        // TODO: bad, should be defined somewhere else
        let positional_count = if args.contains(&"--query-all") { 1 } else { 2 };

        for i in 0..positional_count {
            let arg = args[args.len() - (positional_count - i)];
            if arg.starts_with('-') {
                return Err(format!("OPTIONS should be set before ARGS"));
            }

            positional.push(arg);
        }

        let args = &args[..args.len() - positional_count];

        let mut i = 0;
        while i < args.len() {
            let arg1 = args[i];
            let arg2 = args.get(i + 1).cloned().unwrap_or("");
            i += 1;

            if !arg1.starts_with('-') {
                return Err(format!("Invalid option '{}'", arg1));
            }

            if FLAGS.contains(&arg1) {
                if !arg2.starts_with('-') && !arg2.is_empty() {
                    return Err(format!("Invalid option '{} {}'", arg1, arg2));
                }

                flags.push(arg1);
                continue;
            }

            if arg1.starts_with("--") {
                match arg1.bytes().position(|c| c == b'=') {
                    Some(idx) => {
                        let name = &arg1[..idx];
                        let value = &arg1[(idx + 1)..];

                        if !OPTIONS.contains(&name) {
                            return Err(format!("Unknown option '{}'", arg1));
                        }

                        options.push((name, value));
                    }
                    None => {
                        if !OPTIONS.contains(&arg1) {
                            return Err(format!("Unknown option '{}'", arg1));
                        }

                        if arg2.is_empty() {
                            return Err(format!("Missing value for option '{}'", arg1));
                        }

                        options.push((arg1, arg2));
                        i += 1;
                    }
                }
            } else {
                if arg1.contains('=') {
                    return Err(format!("Invalid option '{}'", arg1));
                }

                if !OPTIONS.contains(&arg1) {
                    return Err(format!("Unknown option '{}'", arg1));
                }

                if arg2.is_empty() {
                    return Err(format!("Missing value for option '{}'", arg1));
                }

                options.push((arg1, arg2));
                i += 1;
            }
        }

        // Replace short options with long one.
        for v in &mut options {
            match v.0 {
                "-w" => v.0 = "--width",
                "-h" => v.0 = "--height",
                "-z" => v.0 = "--zoom",
                _ => {}
            }
        }

        Ok(ArgsList {
            flags,
            options,
            positional,
        })
    }

    fn has_flag(&self, name: &str) -> bool {
        self.flags.contains(&name)
    }

    fn get_option(&self, name: &str) -> Option<&str> {
        match self.options.iter().find(|v| v.0 == name) {
            Some(&(_, v)) => Some(v),
            None => None,
        }
    }

    fn get_type<T: FromStr>(&self, name: &str, type_name: &str) -> Result<Option<T>, String> {
        match self.get_option(name) {
            Some(v) => {
                let t = v.parse().map_err(|_| format!("Invalid {}: '{}'", type_name, v))?;
                Ok(Some(t))
            }
            None => Ok(None),
        }
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
    let args: Vec<_> = env::args().collect();
    let args: Vec<_> = args.iter().skip(1).map(|s| s.as_str()).collect();
    parse_args_list(ArgsList::parse(&args)?)
}

fn parse_args_list(args: ArgsList) -> Result<(Args, Options), String> {
    if args.has_flag("--help") {
        print_help();
        process::exit(0);
    }

    if args.has_flag("--version") {
        println!("{}", env!("CARGO_PKG_VERSION"));
        process::exit(0);
    }

    let in_svg: path::PathBuf = args.positional[0].to_string().into();

    let out_png = if !args.has_flag("--query-all") {
        Some(args.positional[1].to_string().into())
    } else {
        None
    };

    let backend_name = args.get_option("backend").unwrap_or(default_backend()).to_string();
    let dump = args.get_option("--dump-svg").map(|v| v.into());
    let export_id = args.get_option("--export-id").map(|v| v.to_string());

    let app_args = Args {
        in_svg: in_svg.clone(),
        out_png,
        backend_name,
        query_all: args.has_flag("--query-all"),
        export_id,
        dump,
        pretend: args.has_flag("--pretend"),
        perf: args.has_flag("--perf"),
        quiet: args.has_flag("--quiet"),
    };

    // We don't have to keep named groups when we don't need them
    // because it will slow down rendering.
    let keep_named_groups = app_args.query_all || app_args.export_id.is_some();

    let mut fit_to = FitTo::Original;
    if let Some(v) = args.get_option("--width") {
        let w = v.parse().map_err(|_| format!("Invalid LENGTH"))?;
        if w == 0 {
            return Err(format!("Invalid LENGTH"));
        }

        fit_to = FitTo::Width(w);
    } else if let Some(v) = args.get_option("--height") {
        let h = v.parse().map_err(|_| format!("Invalid LENGTH"))?;
        if h == 0 {
            return Err(format!("Invalid LENGTH"));
        }

        fit_to = FitTo::Height(h);
    } else if let Some(z) = args.get_type("--zoom", "FACTOR")? {
        if !(z > 0.0) {
            return Err(format!("Invalid FACTOR"));
        }

        fit_to = FitTo::Zoom(z);
    }

    let background = args.get_type("--background", "COLOR")?;

    let dpi = args.get_type("--dpi", "DPI")?.unwrap_or(96);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_1() {
        let args = ["in.svg", "out.png"];
        let args = ArgsList::parse(&args).unwrap();
        assert_eq!(args.flags,      vec!["";0]);
        assert_eq!(args.options,    vec![]);
        assert_eq!(args.positional, vec!["in.svg", "out.png"]);
    }

    #[test]
    fn parse_2() {
        let args = ["--pretend", "in.svg", "out.png"];
        let args = ArgsList::parse(&args).unwrap();
        assert_eq!(args.flags,      vec!["--pretend"]);
        assert_eq!(args.options,    vec![]);
        assert_eq!(args.positional, vec!["in.svg", "out.png"]);
    }

    #[test]
    fn parse_3() {
        let args = ["-w", "50", "in.svg", "out.png"];
        let args = ArgsList::parse(&args).unwrap();
        assert_eq!(args.flags,      vec!["";0]);
        assert_eq!(args.options,    vec![("--width", "50")]);
        assert_eq!(args.positional, vec!["in.svg", "out.png"]);
    }

    #[test]
    fn parse_4() {
        let args = ["--width=50", "in.svg", "out.png"];
        let args = ArgsList::parse(&args).unwrap();
        assert_eq!(args.flags,      vec!["";0]);
        assert_eq!(args.options,    vec![("--width", "50")]);
        assert_eq!(args.positional, vec!["in.svg", "out.png"]);
    }

    #[test]
    fn parse_5() {
        let args = ["--width", "50", "in.svg", "out.png"];
        let args = ArgsList::parse(&args).unwrap();
        assert_eq!(args.flags,      vec!["";0]);
        assert_eq!(args.options,    vec![("--width", "50")]);
        assert_eq!(args.positional, vec!["in.svg", "out.png"]);
    }

    #[test]
    fn parse_6() {
        let args = ["--version"];
        let args = ArgsList::parse(&args).unwrap();
        assert_eq!(args.flags,      vec!["--version"]);
        assert_eq!(args.options,    vec![]);
        assert_eq!(args.positional, vec!["";0]);
    }

    #[test]
    fn parse_7() {
        let args = ["--help"];
        let args = ArgsList::parse(&args).unwrap();
        assert_eq!(args.flags,      vec!["--help"]);
        assert_eq!(args.options,    vec![]);
        assert_eq!(args.positional, vec!["";0]);
    }

    #[test]
    fn parse_8() {
        let args = ["-h"];
        let args = ArgsList::parse(&args).unwrap();
        assert_eq!(args.flags,      vec!["--help"]);
        assert_eq!(args.options,    vec![]);
        assert_eq!(args.positional, vec!["";0]);
    }

    #[test]
    fn parse_9() {
        let args = ["--query-all", "in.svg"];
        let args = ArgsList::parse(&args).unwrap();
        assert_eq!(args.flags,      vec!["--query-all"]);
        assert_eq!(args.options,    vec![]);
        assert_eq!(args.positional, vec!["in.svg"]);
    }

    #[test]
    fn parse_err_1() {
        let args = ["out.png"];
        assert_eq!(ArgsList::parse(&args).unwrap_err(),
                   "<in-svg> and <out-png> must be set");
    }

    #[test]
    fn parse_err_2() {
        let args = ["-w=50", "in.svg", "out.png"];
        assert_eq!(ArgsList::parse(&args).unwrap_err(),
                   "Invalid option '-w=50'");
    }

    #[test]
    fn parse_err_3() {
        let args = ["--width", "in.svg", "out.png"];
        assert_eq!(ArgsList::parse(&args).unwrap_err(),
                   "Missing value for option '--width'");
    }

    #[test]
    fn parse_err_4() {
        let args = ["-w", "in.svg", "out.png"];
        assert_eq!(ArgsList::parse(&args).unwrap_err(),
                   "Missing value for option '-w'");
    }

    #[test]
    fn parse_err_5() {
        let args = ["-g", "in.svg", "out.png"];
        assert_eq!(ArgsList::parse(&args).unwrap_err(),
                   "Unknown option '-g'");
    }

    #[test]
    fn parse_err_6() {
        let args = ["-g", "50", "in.svg", "out.png"];
        assert_eq!(ArgsList::parse(&args).unwrap_err(),
                   "Unknown option '-g'");
    }

    #[test]
    fn parse_err_7() {
        let args = ["--long", "50", "in.svg", "out.png"];
        assert_eq!(ArgsList::parse(&args).unwrap_err(),
                   "Unknown option '--long'");
    }

    #[test]
    fn parse_err_8() {
        let args = ["in.svg", "out.png", "--pretend"];
        assert_eq!(ArgsList::parse(&args).unwrap_err(),
                   "OPTIONS should be set before ARGS");
    }

    #[test]
    fn parse_err_9() {
        let args = ["-w", "50", "50", "in.svg", "out.png"];
        assert_eq!(ArgsList::parse(&args).unwrap_err(),
                   "Invalid option '50'");
    }
}
