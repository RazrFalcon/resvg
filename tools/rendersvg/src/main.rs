// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt;
use std::fs;
use std::io::Write;
use std::path;

use usvg::NodeExt;

mod args;


macro_rules! timed {
    ($args:expr, $name:expr, $task:expr) => { run_task($args.perf, $name, || $task) };
}

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
    #[cfg(all(not(feature = "cairo-backend"),
              not(feature = "qt-backend"),
              not(feature = "skia-backend"),
              not(feature = "raqote-backend")))]
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
            .apply()
            .unwrap();
    }

    // Load file.
    let tree = timed!(args, "Preprocessing", {
        usvg::Tree::from_file(&args.in_svg, &opt.usvg).map_err(|e| e.to_string())
    })?;

    if args.query_all {
        return query_all(&tree);
    }

    // Dump before rendering in case of panic.
    if let Some(ref dump_path) = args.dump {
        dump_svg(&tree, dump_path)?;
    }

    if args.pretend {
        return Ok(());
    }

    let out_png = match args.out_png {
        Some(ref path) => path,
        None => return Ok(()),
    };

    // Render.
    match args.backend_name.as_str() {
        #[cfg(feature = "cairo-backend")]
        "cairo" => render_svg_cairo(&args, &tree, opt, out_png)?,
        #[cfg(feature = "qt-backend")]
        "qt" => render_svg_qt(&args, &tree, opt, out_png)?,
        #[cfg(feature = "skia-backend")]
        "skia" => render_svg_skia(&args, &tree, opt, out_png)?,
        #[cfg(feature = "raqote-backend")]
        "raqote" => render_svg_raqote(&args, &tree, opt, out_png)?,
        _ => bail!("unknown backend"),
    }

    Ok(())
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
        bail!("the file has no valid ID's");
    }

    Ok(())
}

#[cfg(feature = "cairo-backend")]
fn render_svg_cairo(args: &args::Args, tree: &usvg::Tree, opt: args::Options, out_png: &path::Path) -> Result<(), String> {
    let opt = resvg_cairo::Options {
        usvg: opt.usvg,
        fit_to: opt.fit_to,
        background: opt.background,
    };

    let img = if let Some(ref id) = args.export_id {
        if let Some(node) = tree.root().descendants().find(|n| &*n.id() == id) {
            timed!(args, "Rendering", resvg_cairo::render_node_to_image(&node, &opt))
        } else {
            bail!("SVG doesn't have '{}' ID", id)
        }
    } else {
        timed!(args, "Rendering", resvg_cairo::render_to_image(&tree, &opt))
    };

    match img {
        Some(img) => {
            run_task(args.perf, "Saving", || {
                let mut file = std::fs::File::create(out_png).map_err(|e| e.to_string())?;
                img.write_to_png(&mut file).map_err(|e| e.to_string())
            })?;

            Ok(())
        }
        None => {
            bail!("failed to allocate an image")
        }
    }
}

#[cfg(feature = "qt-backend")]
fn render_svg_qt(args: &args::Args, tree: &usvg::Tree, opt: args::Options, out_png: &path::Path) -> Result<(), String> {
    let opt = resvg_qt::Options {
        usvg: opt.usvg,
        fit_to: opt.fit_to,
        background: opt.background,
    };

    let img = if let Some(ref id) = args.export_id {
        if let Some(node) = tree.root().descendants().find(|n| &*n.id() == id) {
            timed!(args, "Rendering", resvg_qt::render_node_to_image(&node, &opt))
        } else {
            bail!("SVG doesn't have '{}' ID", id)
        }
    } else {
        timed!(args, "Rendering", resvg_qt::render_to_image(&tree, &opt))
    };

    match img {
        Some(img) => {
            run_task(args.perf, "Saving", || {
                img.save(out_png.to_str().unwrap());
            });

            Ok(())
        }
        None => {
            bail!("failed to allocate an image")
        }
    }
}

#[cfg(feature = "skia-backend")]
fn render_svg_skia(args: &args::Args, tree: &usvg::Tree, opt: args::Options, out_png: &path::Path) -> Result<(), String> {
    let opt = resvg_skia::Options {
        usvg: opt.usvg,
        fit_to: opt.fit_to,
        background: opt.background,
    };

    let img = if let Some(ref id) = args.export_id {
        if let Some(node) = tree.root().descendants().find(|n| &*n.id() == id) {
            timed!(args, "Rendering", resvg_skia::render_node_to_image(&node, &opt))
        } else {
            bail!("SVG doesn't have '{}' ID", id)
        }
    } else {
        timed!(args, "Rendering", resvg_skia::render_to_image(&tree, &opt))
    };

    match img {
        Some(img) => {
            timed!(args, "Saving", img.save_png(out_png.to_str().unwrap()));
            Ok(())
        }
        None => {
            bail!("failed to allocate an image")
        }
    }
}

#[cfg(feature = "raqote-backend")]
fn render_svg_raqote(args: &args::Args, tree: &usvg::Tree, opt: args::Options, out_png: &path::Path) -> Result<(), String> {
    let opt = resvg_raqote::Options {
        usvg: opt.usvg,
        fit_to: opt.fit_to,
        background: opt.background,
    };

    let img = if let Some(ref id) = args.export_id {
        if let Some(node) = tree.root().descendants().find(|n| &*n.id() == id) {
            timed!(args, "Rendering", resvg_raqote::render_node_to_image(&node, &opt))
        } else {
            bail!("SVG doesn't have '{}' ID", id)
        }
    } else {
        timed!(args, "Rendering", resvg_raqote::render_to_image(&tree, &opt))
    };

    match img {
        Some(img) => {
            run_task(args.perf, "Saving", || {
                img.write_png(out_png).map_err(|e| e.to_string())
            })?;

            Ok(())
        }
        None => {
            bail!("failed to allocate an image")
        }
    }
}

fn run_task<P, T>(perf: bool, title: &str, p: P) -> T
    where P: FnOnce() -> T
{
    if perf {
        let now = std::time::Instant::now();
        let res = p();
        println!("{}: {:.2}ms", title, now.elapsed().as_micros() as f64 / 1000.0);
        res
    } else {
        p()
    }
}

fn dump_svg(tree: &usvg::Tree, path: &path::Path) -> Result<(), String> {
    let mut f = fs::File::create(path)
        .map_err(|_| format!("failed to create a file {:?}", path))?;

    f.write_all(tree.to_string(usvg::XmlOptions::default()).as_bytes())
     .map_err(|_| format!("failed to write a file {:?}", path))?;

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
