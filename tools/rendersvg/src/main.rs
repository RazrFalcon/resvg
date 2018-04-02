// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#[macro_use]
extern crate clap;
#[macro_use]
extern crate failure;
extern crate resvg;
extern crate fern;
extern crate log;
extern crate time;
extern crate libflate;


use std::str::FromStr;
use std::fs;
use std::fmt;
use std::path;
use std::io::{ self, Write };

use clap::{ App, Arg, ArgMatches };

use resvg::{
    usvg,
    svgdom,
    FitTo,
    Options,
    RectExt,
    Render,
};
use resvg::tree::prelude::*;

use svgdom::WriteBuffer;


struct Args {
    in_svg: path::PathBuf,
    out_png: Option<path::PathBuf>,
    backend: Box<Render>,
    #[allow(dead_code)]
    backend_name: String,
    query_all: bool,
    export_id: Option<String>,
    dump: Option<path::PathBuf>,
    pretend: bool,
    perf: bool,
    quiet: bool,
}

/// Errors list.
#[derive(Fail, Debug)]
pub enum Error {
    /// An invalid node's ID.
    #[fail(display = "SVG doesn't have '{}' ID", _0)]
    InvalidId(String),

    /// `resvg` errors.
    #[fail(display = "{}", _0)]
    Resvg(resvg::Error),

    /// IO errors.
    #[fail(display = "{}", _0)]
    Io(::std::io::Error),
}

impl From<resvg::Error> for Error {
    fn from(value: resvg::Error) -> Error {
        Error::Resvg(value)
    }
}

impl From<::std::io::Error> for Error {
    fn from(value: ::std::io::Error) -> Error {
        Error::Io(value)
    }
}


fn main() {
    #[cfg(all(not(feature = "cairo-backend"), not(feature = "qt-backend")))]
    {
        eprintln!("Error: rendersvg has been built without any backends.");
        return;
    }

    if let Err(e) = process() {
        eprintln!("Error: {}.", e);
        std::process::exit(1);
    }
}

fn process() -> Result<(), Error> {
    let (args, opt) = parse_args();

    // Do not print warning during the ID querying.
    //
    // Some crates still can print to stdout/stderr, but we can't do anything about it.
    if !(args.query_all || args.quiet) {
        fern::Dispatch::new()
            .format(log_format)
            .level(log::LevelFilter::Warn)
            .chain(std::io::stderr())
            .apply().unwrap();
    }

    macro_rules! timed {
        ($name:expr, $task:expr) => { run_task(args.perf, $name, || $task) };
    }

    // Load file.
    let tree = timed!("Preprocessing", resvg::parse_rtree_from_file(&args.in_svg, &opt))?;

    // We have to init only Qt backend.
    #[cfg(feature = "qt-backend")]
    let _resvg = timed!("Backend init", init_qt_gui(&tree, &args));

    if args.query_all {
        query_all(&tree, &args, &opt);
        return Ok(());
    }

    if let Some(ref dump_path) = args.dump {
        dump_svg(&tree, dump_path)?;
    }

    if args.pretend {
        return Ok(());
    }

    // Render.
    if let Some(ref out_png) = args.out_png {
        let img = if let Some(ref id) = args.export_id {
            if let Some(node) = tree.root().descendants().find(|n| &*n.id() == id) {
                timed!("Rendering", args.backend.render_node_to_image(&node, &opt))
            } else {
                return Err(Error::InvalidId(id.clone()));
            }
        } else {
            timed!("Rendering", args.backend.render_to_image(&tree, &opt))
        }?;

        timed!("Saving", img.save(out_png));
    };

    Ok(())
}

// Qt backend initialization is pretty slow
// and needed only for files with text nodes.
// So we skip it file doesn't have one.
#[cfg(feature = "qt-backend")]
fn init_qt_gui(
    tree: &tree::Tree,
    args: &Args,
) -> Option<resvg::InitObject> {
    if args.backend_name != "qt" {
        return None;
    }

    // Check that tree has any text nodes.
    let has_text = tree.root().descendants().any(|n|
        if let tree::NodeKind::Text { .. } = *n.kind() { true } else { false }
    );

    if has_text {
        // Init Qt backend.
        Some(resvg::init())
    } else {
        None
    }
}

fn query_all(
    tree: &tree::Tree,
    args: &Args,
    opt: &Options,
) {
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

        if let Some(bbox) = args.backend.calc_node_bbox(&node, &opt) {
            println!("{},{},{},{},{}", node.id(),
                     round_len(bbox.x()), round_len(bbox.y()),
                     round_len(bbox.width()), round_len(bbox.height()));
        }
    }

    if count == 0 {
        eprintln!("Error: The file has no valid ID's.");
    }
}

fn run_task<P, T>(perf: bool, title: &str, p: P) -> T
    where P: FnOnce() -> T
{
    if perf {
        let start = time::precise_time_ns();
        let res = p();
        let end = time::precise_time_ns();
        println!("{}: {:.2}ms", title, (end - start) as f64 / 1_000_000.0);
        res
    } else {
        p()
    }
}

fn parse_args() -> (Args, Options) {
    let app = prepare_app();
    let args = match app.get_matches_safe() {
        Ok(a) => a,
        Err(mut e) => {
            // change case before printing an error
            if e.message.starts_with("error:") {
                e.message = e.message.replace("error:", "Error:");
            }
            e.exit();
        }
    };

    let app_args = fill_args(&args);
    let opt = fill_options(&args, &app_args);
    (app_args, opt)
}

fn prepare_app<'a, 'b>() -> App<'a, 'b> {
    App::new("rendersvg")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(Arg::with_name("in-svg")
            .help("Input file")
            .required(true)
            .index(1)
            .validator(is_svg))
        .arg(Arg::with_name("out-png")
            .help("Output file")
            .required_unless_one(&["query-all"])
            .index(2)
            .validator(is_png))
        .arg(Arg::with_name("dpi")
            .long("dpi")
            .help("Sets the resolution [72..4000]")
            .value_name("DPI")
            .default_value("96")
            .validator(is_dpi))
        .arg(Arg::with_name("width")
            .short("w")
            .long("width")
            .value_name("LENGTH")
            .help("Sets the width in pixels")
            .conflicts_with_all(&["height", "zoom"])
            .validator(is_length))
        .arg(Arg::with_name("height")
            .short("h")
            .long("height")
            .value_name("LENGTH")
            .help("Sets the height in pixels")
            .conflicts_with_all(&["width", "zoom"])
            .validator(is_length))
        .arg(Arg::with_name("zoom")
            .short("z")
            .long("zoom")
            .value_name("ZOOM")
            .help("Zooms the image by a factor")
            .conflicts_with_all(&["width", "height"])
            .validator(is_zoom))
        .arg(Arg::with_name("background")
            .long("background")
            .value_name("COLOR")
            .help("Sets the background color")
            .validator(is_color))
        .arg(Arg::with_name("backend")
            .long("backend")
            .help("Sets the rendering backend.\nHas no effect if built with only one backend")
            .takes_value(true)
            .default_value(default_backend())
            .possible_values(&backends()))
        .arg(Arg::with_name("query-all")
            .long("query-all")
            .help("Queries all valid SVG ids with bounding boxes"))
        .arg(Arg::with_name("export-id")
            .long("export-id")
            .help("Renders an object only with a specified ID")
            .value_name("ID"))
        .arg(Arg::with_name("dump-svg")
            .long("dump-svg")
            .help("Saves a preprocessed SVG to the selected file")
            .value_name("PATH"))
        .arg(Arg::with_name("pretend")
            .long("pretend")
            .help("Does all the steps except rendering"))
        .arg(Arg::with_name("perf")
            .long("perf")
            .help("Prints performance stats"))
        .arg(Arg::with_name("quiet")
            .long("quiet")
            .help("Disables warnings"))
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

    list
}

#[allow(unreachable_code)]
fn default_backend() -> &'static str {
    #[cfg(feature = "cairo-backend")]
    {
        return "cairo"
    }

    #[cfg(feature = "qt-backend")]
    {
        return "qt"
    }

    unreachable!();
}

// TODO: simplify is_* methods
fn is_svg(val: String) -> Result<(), String> {
    let val = val.to_lowercase();
    if val.ends_with(".svg") || val.ends_with(".svgz") {
        Ok(())
    } else {
        Err(String::from("The input file format must be SVG(Z)."))
    }
}

fn is_png(val: String) -> Result<(), String> {
    if val.ends_with(".png") || val.ends_with(".PNG") {
        Ok(())
    } else {
        Err(String::from("The output file format must be PNG."))
    }
}

fn is_dpi(val: String) -> Result<(), String> {
    let n = match val.parse::<u32>() {
        Ok(v) => v,
        Err(e) => return Err(format!("{}", e)),
    };

    if n >= 72 && n <= 4000 {
        Ok(())
    } else {
        Err(String::from("Invalid DPI value."))
    }
}

fn is_length(val: String) -> Result<(), String> {
    let n = match val.parse::<u32>() {
        Ok(v) => v,
        Err(e) => return Err(format!("{}", e)),
    };

    if n > 0 {
        Ok(())
    } else {
        Err(String::from("Invalid length value."))
    }
}

fn is_zoom(val: String) -> Result<(), String> {
    let n = match val.parse::<f32>() {
        Ok(v) => v,
        Err(e) => return Err(format!("{}", e)),
    };

    if n > 0.0 {
        Ok(())
    } else {
        Err(String::from("Invalid zoom value."))
    }
}

fn is_color(val: String) -> Result<(), String> {
    match usvg::tree::Color::from_str(&val) {
        Ok(_) => Ok(()),
        Err(_) => Err("Invalid color.".into()),
    }
}

fn fill_args(args: &ArgMatches) -> Args {
    let in_svg  = args.value_of("in-svg").unwrap().into();
    let out_png = if args.is_present("out-png") {
        Some(args.value_of("out-png").unwrap().into())
    } else {
        None
    };

    let dump = if args.is_present("dump-svg") {
        Some(args.value_of("dump-svg").unwrap().into())
    } else {
        None
    };

    let export_id = if args.is_present("export-id") {
        Some(args.value_of("export-id").unwrap().into())
    } else {
        None
    };

    let backend_name = args.value_of("backend").unwrap().to_string();
    let backend: Box<Render> = match backend_name.as_str() {
        #[cfg(feature = "cairo-backend")]
        "cairo" => Box::new(resvg::render_cairo::Backend),
        #[cfg(feature = "qt-backend")]
        "qt" => Box::new(resvg::render_qt::Backend),
        _ => unreachable!(),
    };

    Args {
        in_svg,
        out_png,
        backend,
        backend_name,
        query_all: args.is_present("query-all"),
        export_id,
        dump,
        pretend: args.is_present("pretend"),
        perf: args.is_present("perf"),
        quiet: args.is_present("quiet"),
    }
}

fn fill_options(args: &ArgMatches, app_args: &Args) -> Options {
    let mut fit_to = FitTo::Original;
    if args.is_present("width") {
        fit_to = FitTo::Width(value_t!(args.value_of("width"), u32).unwrap());
    } else if args.is_present("height") {
        fit_to = FitTo::Height(value_t!(args.value_of("height"), u32).unwrap());
    } else if args.is_present("zoom") {
        fit_to = FitTo::Zoom(value_t!(args.value_of("zoom"), f32).unwrap());
    }

    let mut background = None;
    if args.is_present("background") {
        let s = args.value_of("background").unwrap();
        background = Some(usvg::tree::Color::from_str(s).unwrap());
    }

    // We don't have to keep named groups when we don't need them
    // because it will slow down rendering.
    let keep_named_groups = app_args.query_all || app_args.export_id.is_some();

    Options {
        usvg: usvg::Options {
            path: Some(args.value_of("in-svg").unwrap().into()),
            dpi: value_t!(args.value_of("dpi"), u16).unwrap() as f64,
            keep_named_groups,
        },
        fit_to,
        background,
    }
}

fn dump_svg(tree: &tree::Tree, path: &path::Path) -> Result<(), io::Error> {
    let mut f = fs::File::create(path)?;

    let opt = svgdom::WriteOptions {
        indent: svgdom::Indent::Spaces(2),
        attributes_indent: svgdom::Indent::Spaces(3),
        write_hidden_attributes: true,
        attributes_order: svgdom::AttributesOrder::Specification,
        .. svgdom::WriteOptions::default()
    };

    let svgdoc = tree.to_svgdom();

    let mut out = Vec::new();
    svgdoc.write_buf_opt(&opt, &mut out);
    f.write_all(&out)?;

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
