extern crate gio;
extern crate gtk;
extern crate resvg;
extern crate fern;

use std::fmt;
use std::env::args;
use std::path::Path;

use gio::prelude::*;
use gtk::prelude::*;
use gtk::DrawingArea;

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
        .level(resvg::log::LogLevelFilter::Warn)
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

    let opt = resvg::Options {
        path: Some(file_path.into()),
        .. resvg::Options::default()
    };

    let svg_doc = resvg::parse_doc_from_file(file_path, &opt).unwrap();

    drawing_area.connect_draw(move |w, cr| {
        let r = resvg::Rect::new(
            0.0, 0.0,
            w.get_allocated_width() as f64, w.get_allocated_height() as f64
        );
        resvg::render_cairo::render_to_canvas(cr, r, &svg_doc);

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

fn log_format(out: fern::FormatCallback, message: &fmt::Arguments, record: &resvg::log::LogRecord) {
    use resvg::log::LogLevel;

    let lvl = match record.level() {
        LogLevel::Error => "Error",
        LogLevel::Warn => "Warning",
        LogLevel::Info => "Info",
        LogLevel::Debug => "Debug",
        LogLevel::Trace => "Trace",
    };

    out.finish(format_args!(
        "{} (in {}:{}): {}",
        lvl,
        record.target(),
        record.location().line(),
        message
    ))
}
