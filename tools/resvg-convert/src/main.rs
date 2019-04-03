// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#[allow(unused_imports)] // for Rust >= 1.30
#[macro_use]
extern crate gumdrop;

extern crate fern;
extern crate log;
extern crate resvg;
extern crate time;


use std::fmt;
use std::fs::{File};
use std::io::{self, Write, Read};
use std::path;

use resvg::prelude::*;
use resvg::svgdom;
use resvg::RenderFormat;

use svgdom::WriteBuffer;

mod args;


macro_rules! bail {
    ($msg:expr) => {
        return Err(format!("{}", $msg));
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err(format!($fmt, $($arg)*));
    };
}


fn main() {
    if let Err(e) = process() {
        eprintln!("Error: {}.", e);
        std::process::exit(1);
    }
}

fn process() -> Result<(), String> {
    #[cfg(all(not(feature = "cairo-backend"), not(feature = "qt-backend")))]
    {
        bail!("rendersvg has been built without any backends")
    }

    let (args, opt) = match args::parse() {
        Ok((args, opt)) => (args, opt),
        Err(e) => {
            args::print_help();
            println!();
            bail!(e)
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
        "cairo" => Box::new(resvg::backend_cairo::Backend),
        _ => bail!("unknown backend"),
    };

    macro_rules! timed {
        ($name:expr, $task:expr) => { run_task(args.perf, $name, || $task) };
    }

    // Load file.
    let tree = if !args.in_svg.eq("-") {
        timed!("Preprocessing", {
            usvg::Tree::from_file(&args.in_svg, &opt.usvg)
                .map_err(|e| e.to_string())
        })?
    } else {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).expect("Cannot read string");
        timed!("Preprocessing", {
            usvg::Tree::from_str(&buffer, &opt.usvg)
                    .map_err(|e| e.to_string())
        })?
    };

    if args.query_all {
        return query_all(backend, &tree, &opt);
    }

    // Dump before rendering in case of panic.
    if let Some(ref dump_path) = args.dump {
        dump_svg(&tree, dump_path)?;
    }

    if args.pretend {
        return Ok(());
    }

    let format = match args.out_format.as_ref() {
        "svg" => RenderFormat::SVG,
        "pdf" => RenderFormat::PDF,
        "png" => RenderFormat::PNG,
        "eps" => RenderFormat::EPS,
        "ps" => RenderFormat::PS,
        _ => bail!("unknown format"),
    };

    if args.out_file.eq("-") {
        let mut stream = io::stdout();
        stream.lock();
        if let Some(ref id) = args.export_id {
            if let Some(node) = tree.root().descendants().find(|n| &*n.id() == id) {
                timed!("Rendering", backend.render_node_to_stream(&node, &opt, format, &mut stream));
            } else {
                bail!("SVG doesn't have '{}' ID", id)
            }
        } else {
            timed!("Rendering", backend.render_to_stream(&tree, &opt, format, &mut stream));
        }
    } else {
        let mut file = File::create(args.out_file).expect("Cannot create file for writing");
        if let Some(ref id) = args.export_id {
            if let Some(node) = tree.root().descendants().find(|n| &*n.id() == id) {
                timed!("Rendering", backend.render_node_to_stream(&node, &opt, format, &mut file));
            } else {
                bail!("SVG doesn't have '{}' ID", id)
            }
        } else {
            timed!("Rendering", backend.render_to_stream(&tree, &opt, format, &mut file));
        }
    };

    Ok(())
}

fn query_all(
    backend: Box<Render>,
    tree: &usvg::Tree,
    opt: &Options,
) -> Result<(), String> {
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
                     round_len(bbox.x), round_len(bbox.y),
                     round_len(bbox.width), round_len(bbox.height));
        }
    }

    if count == 0 {
        bail!("the file has no valid ID's");
    }

    Ok(())
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

fn dump_svg(tree: &usvg::Tree, path: &path::Path) -> Result<(), String> {
    let mut f = File::create(path)
                   .map_err(|_| format!("failed to create a file {:?}", path))?;

    let opt = svgdom::WriteOptions {
        indent: svgdom::Indent::Spaces(2),
        attributes_indent: svgdom::Indent::Spaces(3),
        attributes_order: svgdom::AttributesOrder::Specification,
        .. svgdom::WriteOptions::default()
    };

    let svgdoc = tree.to_svgdom();

    let mut out = Vec::new();
    svgdoc.write_buf_opt(&opt, &mut out);
    f.write_all(&out).map_err(|_| format!("failed to write a file {:?}", path))?;

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
