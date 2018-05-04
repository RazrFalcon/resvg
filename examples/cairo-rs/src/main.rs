extern crate gio;
extern crate gtk;
extern crate resvg;
extern crate log;
extern crate fern;

use std::fmt;
use std::env::args;
use std::path::Path;

use gio::prelude::*;
use gtk::prelude::*;
use gtk::DrawingArea;

use resvg::usvg;

// make moving clones into closures more convenient
macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move |$(clone!(@param $p),)+| $body
        }
    );
}

fn main() {
    fern::Dispatch::new()
        .format(log_format)
        .level(log::LevelFilter::Warn)
        .chain(std::io::stderr())
        .apply().unwrap();

    let application = gtk::Application::new("com.github.cairo-example",
                                            gio::ApplicationFlags::from_bits_truncate(4))
                                       .expect("Initialization failed...");

    application.connect_activate(|_| {});
    application.connect_open(move |app, files, _| {
        let path = files[0].get_path().unwrap();
        build_ui(app, &path);
    });

    application.run(&args().collect::<Vec<_>>());
}

fn build_ui(application: &gtk::Application, file_path: &Path) {
    let window = gtk::ApplicationWindow::new(application);
    let drawing_area = Box::new(DrawingArea::new)();

    let mut opt = resvg::Options::default();
    opt.usvg.path = Some(file_path.into());

    let tree = usvg::Tree::from_file(file_path, &opt.usvg).unwrap();

    drawing_area.connect_draw(move |w, cr| {
        let s = resvg::ScreenSize::new(
            w.get_allocated_width() as u32,
            w.get_allocated_height() as u32,
        );
        resvg::render_cairo::render_to_canvas(&tree, &opt, s, cr);

        Inhibit(false)
    });

    window.set_default_size(500, 500);

    window.connect_delete_event(clone!(window => move |_, _| {
        window.destroy();
        Inhibit(false)
    }));
    window.add(&drawing_area);
    window.show_all();
}

fn log_format(out: fern::FormatCallback, message: &fmt::Arguments, record: &log::Record) {
    let lvl = match record.level() {
        log::Level::Error => "Error",
        log::Level::Warn => "Warning",
        log::Level::Info => "Info",
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
