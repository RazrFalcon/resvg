use std::env::args;

use gio::prelude::*;
use gtk::prelude::*;
use gtk::DrawingArea;

fn main() {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{}] {}",
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Warn)
        .chain(std::io::stderr())
        .apply()
        .unwrap();

    let application = gtk::Application::new(
        Some("com.github.cairo-example"),
        gio::ApplicationFlags::from_bits_truncate(4)
    ).expect("Initialization failed...");

    application.connect_activate(|_| {});
    application.connect_open(move |app, files, _| {
        let path = files[0].get_path().unwrap();
        build_ui(app, &path);
    });

    application.run(&args().collect::<Vec<_>>());
}

fn build_ui(application: &gtk::Application, file_path: &std::path::Path) {
    let window = gtk::ApplicationWindow::new(application);
    let drawing_area = Box::new(DrawingArea::new)();

    let mut opt = resvg::Options::default();
    opt.usvg.path = Some(file_path.into());

    let tree = resvg::usvg::Tree::from_file(file_path, &opt.usvg).unwrap();

    drawing_area.connect_draw(move |w, cr| {
        let s = resvg::ScreenSize::new(
            w.get_allocated_width() as u32,
            w.get_allocated_height() as u32,
        ).unwrap();
        resvg::backend_cairo::render_to_canvas(&tree, &opt, s, cr);

        Inhibit(false)
    });

    window.set_default_size(500, 500);

    let window_clone = window.clone();
    window.connect_delete_event(move |_, _| {
        window_clone.destroy();
        Inhibit(false)
    });
    window.add(&drawing_area);
    window.show_all();
}
