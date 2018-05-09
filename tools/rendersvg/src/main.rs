// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#[macro_use]
extern crate failure;
extern crate resvg;
extern crate fern;
extern crate log;
extern crate time;
extern crate getopts;


use std::fs;
use std::fmt;
use std::path;
use std::io::{ self, Write };

use resvg::{
    usvg,
    svgdom,
    Options,
    RectExt,
    Render,
};
use usvg::prelude::*;

use svgdom::WriteBuffer;

mod args;


/// Errors list.
#[derive(Fail, Debug)]
pub enum Error {
    #[fail(display = "SVG doesn't have '{}' ID", _0)]
    InvalidId(String),

    #[fail(display = "Failed to allocate an image")]
    NoCanvas,

    #[fail(display = "{}", _0)]
    USvg(usvg::Error),

    #[fail(display = "{}", _0)]
    Io(::std::io::Error),

    #[fail(display = "{}", _0)]
    String(String),
}

impl From<usvg::Error> for Error {
    fn from(value: usvg::Error) -> Self {
        Error::USvg(value)
    }
}

impl From<::std::io::Error> for Error {
    fn from(value: ::std::io::Error) -> Self {
        Error::Io(value)
    }
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Error::String(value)
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
    let (args, opt) = match args::parse() {
        Ok((args, opt)) => (args, opt),
        Err(e) => {
            args::print_help();
            println!();
            return Err(e.into());
        }
    };

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

    let backend: Box<Render> = match args.backend_name.as_str() {
        #[cfg(feature = "cairo-backend")]
        "cairo" => Box::new(resvg::render_cairo::Backend),
        #[cfg(feature = "qt-backend")]
        "qt" => Box::new(resvg::render_qt::Backend),
        _ => return Err(format!("Unknown backend").into()),
    };

    macro_rules! timed {
        ($name:expr, $task:expr) => { run_task(args.perf, $name, || $task) };
    }

    // Load file.
    let tree = timed!("Preprocessing", usvg::Tree::from_file(&args.in_svg, &opt.usvg))?;

    // We have to init only Qt backend.
    #[cfg(feature = "qt-backend")]
    let _resvg = timed!("Backend init", init_qt_gui(&tree, &args));

    if args.query_all {
        query_all(backend, &tree, &opt);
        return Ok(());
    }

    // Dump before rendering in case of panic.
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
                timed!("Rendering", backend.render_node_to_image(&node, &opt)
                                           .ok_or(Error::NoCanvas))
            } else {
                return Err(Error::InvalidId(id.clone()));
            }
        } else {
            timed!("Rendering", backend.render_to_image(&tree, &opt)
                                       .ok_or(Error::NoCanvas))
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
    tree: &usvg::Tree,
    args: &args::Args,
) -> Option<resvg::InitObject> {
    if args.backend_name != "qt" {
        return None;
    }

    // Check that tree has any text nodes.
    let has_text = tree.root().descendants().any(|n|
        if let usvg::NodeKind::Text { .. } = *n.borrow() { true } else { false }
    );

    if has_text {
        // Init Qt backend.
        Some(resvg::init())
    } else {
        None
    }
}

fn query_all(
    backend: Box<Render>,
    tree: &usvg::Tree,
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

        if let Some(bbox) = backend.calc_node_bbox(&node, &opt) {
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

fn dump_svg(tree: &usvg::Tree, path: &path::Path) -> Result<(), io::Error> {
    let mut f = fs::File::create(path)?;

    let opt = svgdom::WriteOptions {
        indent: svgdom::Indent::Spaces(2),
        attributes_indent: svgdom::Indent::Spaces(3),
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
