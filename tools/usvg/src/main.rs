// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process;

use pico_args::Arguments;


fn print_help() {
    print!("\
usvg (micro SVG) is an SVG simplification tool.

USAGE:
    usvg [OPTIONS] <in-svg> <out-svg> # from file to file
    usvg [OPTIONS] -c <in-svg>        # from file to stdout
    usvg [OPTIONS] <out-svg> -        # from stdin to file
    usvg [OPTIONS] -c -               # from stdin to stdout

OPTIONS:
    -h, --help                  Prints help information
    -V, --version               Prints version information
    -c                          Prints the output SVG to the stdout
        --keep-named-groups     Disables removing of groups with non-empty ID
        --dpi DPI               Sets the resolution
                                [default: 96] [possible values: 10..4000]
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
        --indent INDENT         Sets the XML nodes indent
                                [values: none, 0, 1, 2, 3, 4, tabs] [default: 4]
        --attrs-indent INDENT   Sets the XML attributes indent
                                [values: none, 0, 1, 2, 3, 4, tabs] [default: none]
        --quiet                 Disables warnings

ARGS:
    <in-svg>                    Input file
    <out-svg>                   Output file
");
}

#[derive(Debug)]
struct Args {
    help: bool,
    version: bool,
    stdout: bool,
    keep_named_groups: bool,
    dpi: u32,
    font_family: String,
    font_size: u32,
    languages: Vec<String>,
    shape_rendering: usvg::ShapeRendering,
    text_rendering: usvg::TextRendering,
    image_rendering: usvg::ImageRendering,
    indent: usvg::XmlIndent,
    attrs_indent: usvg::XmlIndent,
    quiet: bool,
    free: Vec<String>,
}

fn collect_args() -> Result<Args, pico_args::Error> {
    let mut input = Arguments::from_env();
    Ok(Args {
        help:               input.contains(["-h", "--help"]),
        version:            input.contains(["-V", "--version"]),
        stdout:             input.contains("-c"),
        keep_named_groups:  input.contains("--keep-named-groups"),
        dpi:                input.value_from_fn("--dpi", parse_dpi)?.unwrap_or(96),
        font_family:        input.value_from_str("--font-family")?
                                 .unwrap_or_else(|| "Times New Roman".to_string()),
        font_size:          input.value_from_fn("--font-size", parse_font_size)?.unwrap_or(12),
        languages:          input.value_from_fn("--languages", parse_languages)?
                                 .unwrap_or(vec!["en".to_string()]), // TODO: use system language
        shape_rendering:    input.value_from_str("--shape-rendering")?.unwrap_or_default(),
        text_rendering:     input.value_from_str("--text-rendering")?.unwrap_or_default(),
        image_rendering:    input.value_from_str("--image-rendering")?.unwrap_or_default(),
        indent:             input.value_from_fn("--indent", parse_indent)?
                                 .unwrap_or(usvg::XmlIndent::Spaces(4)),
        attrs_indent:       input.value_from_fn("--attrs-indent", parse_indent)?
                                 .unwrap_or(usvg::XmlIndent::None),
        quiet:              input.contains("--quiet"),
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

fn parse_indent(s: &str) -> Result<usvg::XmlIndent, String> {
    let indent = match s {
        "none" => usvg::XmlIndent::None,
        "0" => usvg::XmlIndent::Spaces(0),
        "1" => usvg::XmlIndent::Spaces(1),
        "2" => usvg::XmlIndent::Spaces(2),
        "3" => usvg::XmlIndent::Spaces(3),
        "4" => usvg::XmlIndent::Spaces(4),
        "tabs" => usvg::XmlIndent::Tabs,
        _ => return Err("invalid INDENT value".to_string()),
    };

    Ok(indent)
}

#[derive(Clone, PartialEq, Debug)]
enum InputFrom<'a> {
    Stdin,
    File(&'a str),
}

#[derive(Clone, PartialEq, Debug)]
enum OutputTo<'a> {
    Stdout,
    File(&'a str),
}


fn main() {
    let args = match collect_args() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: {}.", e);
            process::exit(1);
        }
    };

    if args.help {
        print_help();
        process::exit(0);
    }

    if args.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        process::exit(0);
    }

    if !args.quiet {
        fern::Dispatch::new()
            .format(log_format)
            .level(log::LevelFilter::Warn)
            .chain(std::io::stderr())
            .apply()
            .unwrap();
    }

    if let Err(e) = process(&args) {
        eprintln!("Error: {}.", e.to_string());
        std::process::exit(1);
    }
}

fn process(args: &Args) -> Result<(), String> {
    if args.free.is_empty() {
        return Err(format!("no positional arguments are provided"));
    }

    let (in_svg, out_svg) = {
        let in_svg = &args.free[0];
        let out_svg = args.free.get(1);
        let out_svg = out_svg.map(String::as_ref);

        let svg_from = if in_svg == "-" && args.stdout {
            InputFrom::Stdin
        } else if let Some("-") = out_svg {
            InputFrom::Stdin
        } else {
            InputFrom::File(in_svg)
        };

        let svg_to = if args.stdout {
            OutputTo::Stdout
        } else if let Some("-") = out_svg {
            OutputTo::File(in_svg)
        } else {
            OutputTo::File(out_svg.unwrap())
        };

        (svg_from, svg_to)
    };

    let re_opt = usvg::Options {
        path: match in_svg {
            InputFrom::Stdin => None,
            InputFrom::File(ref f) => Some(f.into()),
        },
        dpi: args.dpi as f64,
        font_family: args.font_family.clone(),
        font_size: args.font_size as f64,
        languages: args.languages.clone(),
        shape_rendering: args.shape_rendering,
        text_rendering: args.text_rendering,
        image_rendering: args.image_rendering,
        keep_named_groups: args.keep_named_groups,
    };

    let input_str = match in_svg {
        InputFrom::Stdin => load_stdin(),
        InputFrom::File(ref path) => {
            usvg::load_svg_file(Path::new(path)).map_err(|e| e.to_string())
        }
    }?;

    let tree = usvg::Tree::from_str(&input_str, &re_opt).map_err(|e| format!("{}", e))?;

    let xml_opt = usvg::XmlOptions {
        use_single_quote: false,
        indent: args.indent,
        attributes_indent: args.attrs_indent,
    };

    let s = tree.to_string(xml_opt);
    match out_svg {
        OutputTo::Stdout => {
            io::stdout()
                .write_all(s.as_bytes())
                .map_err(|_| format!("failed to write to the stdout"))?;
        }
        OutputTo::File(path) => {
            let mut f = File::create(path)
                .map_err(|_| format!("failed to create the output file"))?;
            f.write_all(s.as_bytes())
                .map_err(|_| format!("failed to write to the output file"))?;
        }
    }

    Ok(())
}

fn log_format(
    out: fern::FormatCallback,
    message: &fmt::Arguments,
    record: &log::Record,
) {
    let lvl = match record.level() {
        log::Level::Error => "Error",
        log::Level::Warn  => "Warning",
        log::Level::Info  => "Info",
        log::Level::Debug => "Debug",
        log::Level::Trace => "Trace",
    };

    out.finish(format_args!(
        "{} (in {}:{}): {}",
        lvl,
        record.target(),
        record.line().unwrap_or(0),
        message
    ))
}

fn load_stdin() -> Result<String, String> {
    let mut s = String::new();
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    handle
        .read_to_string(&mut s)
        .map_err(|_| format!("provided data has not an UTF-8 encoding"))?;

    Ok(s)
}
